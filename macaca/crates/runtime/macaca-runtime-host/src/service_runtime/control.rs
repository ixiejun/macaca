//! Runtime-owned call controls for the ServiceRuntime facade.
//!
//! This module keeps timeout, cancellation, and bounded-output enforcement
//! generic.  It never branches on pack ids, provider ids, application names, or
//! business commands; callers opt into per-call controls through neutral command
//! metadata while the runtime still applies conservative defaults.

use std::{
    collections::BTreeMap,
    future::Future,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock,
    },
    time::Duration,
};

use chrono::{Duration as ChronoDuration, Utc};
use macaca_proto::{ServiceBusError, ServiceCommand, ServiceEnvelope, ServiceReply};
use tokio::sync::Notify;

use crate::service_runtime_error::ServiceRuntimeError;

const TIMEOUT_METADATA_KEY: &str = "runtime.timeout_ms";
const CANCELLATION_TOKEN_METADATA_KEY: &str = "runtime.cancellation_token";
const CANCELLED_METADATA_KEY: &str = "runtime.cancelled";
const CANCEL_REQUESTED_METADATA_KEY: &str = "runtime.cancel_requested";
const STREAM_FRAME_COUNT_METADATA_KEY: &str = "stream.frame_count";

/// Runtime policy defaults that keep service calls bounded even when callers do
/// not provide per-command metadata.
pub(super) struct ServiceRuntimeControl {
    default_timeout: Option<Duration>,
    max_reply_output_bytes: usize,
    max_stream_frames: usize,
    cancellations: Arc<RwLock<BTreeMap<String, Arc<CancellationSlot>>>>,
}

impl ServiceRuntimeControl {
    /// Build call controls from config.  The registry is owned by the runtime so
    /// cancellation does not require a provider-specific side channel.
    pub(super) fn new(
        default_timeout: Option<Duration>,
        max_reply_output_bytes: usize,
        max_stream_frames: usize,
    ) -> Self {
        Self {
            default_timeout,
            max_reply_output_bytes,
            max_stream_frames,
            cancellations: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }

    /// Resolve metadata into a neutral call-control record before dispatch.
    pub(super) fn prepare_call(
        &self,
        metadata: &BTreeMap<String, String>,
    ) -> Result<ServiceRuntimeCallControl, ServiceRuntimeError> {
        if metadata_flag(metadata, CANCELLED_METADATA_KEY)
            || metadata_flag(metadata, CANCEL_REQUESTED_METADATA_KEY)
        {
            return Err(ServiceRuntimeError::CallCancelled {
                cancellation_token: "metadata_requested".into(),
            });
        }

        let timeout = self.effective_timeout(metadata)?;
        let cancellation_token = metadata
            .get(CANCELLATION_TOKEN_METADATA_KEY)
            .map(|value| normalize_token(value))
            .transpose()?;
        let cancellation_slot = match cancellation_token.as_ref() {
            Some(token) => Some(self.slot_for(token)?),
            None => None,
        };

        if let (Some(token), Some(slot)) = (&cancellation_token, &cancellation_slot) {
            if slot.is_cancelled() {
                return Err(ServiceRuntimeError::CallCancelled {
                    cancellation_token: audit_token_ref(token),
                });
            }
        }

        Ok(ServiceRuntimeCallControl {
            timeout,
            cancellation_token,
            cancellation_slot,
        })
    }

    /// Return a provider-visible command with runtime-only control metadata
    /// removed.  Timeout and cancellation controls are interpreted by the host
    /// runtime and then re-expressed through envelope deadlines and sanitized
    /// metadata, so service providers never need raw control tokens.
    pub(super) fn command_for_dispatch(&self, mut command: ServiceCommand) -> ServiceCommand {
        for key in runtime_control_metadata_keys() {
            command.metadata.remove(key);
        }
        command
    }

    /// Attach call controls to the service-bus envelope for downstream trace
    /// middleware and future remote transports.
    pub(super) fn apply_to_envelope(
        &self,
        envelope: &mut ServiceEnvelope,
        control: &ServiceRuntimeCallControl,
    ) {
        if let Some(timeout) = control.timeout {
            if let Ok(delta) = ChronoDuration::from_std(timeout) {
                envelope.deadline = Some(Utc::now() + delta);
            }
            envelope.metadata.insert(
                "runtime.timeout_ms".into(),
                duration_millis(timeout).to_string(),
            );
        }
        if control.cancellation_token.is_some() {
            envelope
                .metadata
                .insert("runtime.cancellation".into(), "supported".into());
        }
    }

    /// Execute the service-bus future while racing runtime timeout and
    /// cancellation signals.  Dropping the service-bus future is the generic
    /// cooperative cancellation boundary for async providers.
    pub(super) async fn dispatch<F>(
        &self,
        control: ServiceRuntimeCallControl,
        future: F,
    ) -> Result<ServiceReply, ServiceRuntimeError>
    where
        F: Future<Output = Result<ServiceReply, ServiceBusError>>,
    {
        let result = match (control.timeout, control.cancellation_slot.as_ref()) {
            (Some(timeout), Some(slot)) => {
                tokio::select! {
                    reply = future => reply.map_err(ServiceRuntimeError::from),
                    _ = tokio::time::sleep(timeout) => Err(timeout_error(timeout)),
                    _ = slot.cancelled() => Err(cancelled_error(&control)),
                }
            }
            (Some(timeout), None) => match tokio::time::timeout(timeout, future).await {
                Ok(reply) => reply.map_err(ServiceRuntimeError::from),
                Err(_) => Err(timeout_error(timeout)),
            },
            (None, Some(slot)) => {
                tokio::select! {
                    reply = future => reply.map_err(ServiceRuntimeError::from),
                    _ = slot.cancelled() => Err(cancelled_error(&control)),
                }
            }
            (None, None) => future.await.map_err(ServiceRuntimeError::from),
        };
        self.finish_call(&control);
        result
    }

    /// Mark an active cancellation token as cancelled.
    pub(super) fn cancel(&self, token: impl Into<String>) -> Result<String, ServiceRuntimeError> {
        let token = normalize_token(&token.into())?;
        let slot = {
            let slots = self
                .cancellations
                .read()
                .map_err(|_| ServiceRuntimeError::State("cancellation registry poisoned".into()))?;
            slots.get(&token).cloned()
        }
        .ok_or_else(|| {
            ServiceRuntimeError::InvalidArgument("unknown runtime cancellation token".into())
        })?;
        slot.cancel();
        let audit_ref = audit_token_ref(&token);
        tracing::info!(
            cancellation_token_ref = %audit_ref,
            "service runtime cancellation signal recorded"
        );
        Ok(audit_ref)
    }

    /// Enforce bounded single-result and framed-stream replies before exposing
    /// provider output to SDKs, shells, or audit consumers.
    pub(super) fn validate_reply(&self, reply: &ServiceReply) -> Result<(), ServiceRuntimeError> {
        if let Some(output) = &reply.output {
            let actual_bytes = serde_json::to_vec(output)
                .map_err(|error| ServiceRuntimeError::Service(error.to_string()))?
                .len();
            if actual_bytes > self.max_reply_output_bytes {
                return Err(ServiceRuntimeError::ReplyTooLarge {
                    actual_bytes,
                    max_bytes: self.max_reply_output_bytes,
                });
            }
            if let Some(actual_frames) = output_stream_frame_count(output) {
                self.validate_stream_frame_count(actual_frames)?;
            }
        }
        if let Some(actual_frames) = metadata_stream_frame_count(&reply.metadata)? {
            self.validate_stream_frame_count(actual_frames)?;
        }
        Ok(())
    }

    fn effective_timeout(
        &self,
        metadata: &BTreeMap<String, String>,
    ) -> Result<Option<Duration>, ServiceRuntimeError> {
        let override_timeout = metadata
            .get(TIMEOUT_METADATA_KEY)
            .map(|value| parse_timeout_ms(value))
            .transpose()?;
        Ok(match (self.default_timeout, override_timeout) {
            (Some(default), Some(override_timeout)) => Some(default.min(override_timeout)),
            (Some(default), None) => Some(default),
            (None, Some(override_timeout)) => Some(override_timeout),
            (None, None) => None,
        })
    }

    fn slot_for(&self, token: &str) -> Result<Arc<CancellationSlot>, ServiceRuntimeError> {
        let mut slots = self
            .cancellations
            .write()
            .map_err(|_| ServiceRuntimeError::State("cancellation registry poisoned".into()))?;
        Ok(slots
            .entry(token.to_string())
            .or_insert_with(|| Arc::new(CancellationSlot::default()))
            .clone())
    }

    fn finish_call(&self, control: &ServiceRuntimeCallControl) {
        if let Some(token) = &control.cancellation_token {
            if let Ok(mut slots) = self.cancellations.write() {
                slots.remove(token);
            }
        }
    }

    fn validate_stream_frame_count(&self, actual_frames: usize) -> Result<(), ServiceRuntimeError> {
        if actual_frames > self.max_stream_frames {
            return Err(ServiceRuntimeError::StreamFrameLimitExceeded {
                actual_frames,
                max_frames: self.max_stream_frames,
            });
        }
        Ok(())
    }
}

/// Per-call controls derived from command metadata and runtime defaults.
pub(super) struct ServiceRuntimeCallControl {
    timeout: Option<Duration>,
    cancellation_token: Option<String>,
    cancellation_slot: Option<Arc<CancellationSlot>>,
}

#[derive(Default)]
struct CancellationSlot {
    cancelled: AtomicBool,
    notify: Notify,
}

impl CancellationSlot {
    fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        self.notify.notify_waiters();
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    async fn cancelled(&self) {
        loop {
            if self.is_cancelled() {
                return;
            }
            self.notify.notified().await;
        }
    }
}

fn parse_timeout_ms(value: &str) -> Result<Duration, ServiceRuntimeError> {
    let parsed = value.trim().parse::<u64>().map_err(|_| {
        ServiceRuntimeError::InvalidArgument("runtime.timeout_ms must be a positive integer".into())
    })?;
    if parsed == 0 {
        return Err(ServiceRuntimeError::InvalidArgument(
            "runtime.timeout_ms must be greater than zero".into(),
        ));
    }
    Ok(Duration::from_millis(parsed))
}

fn normalize_token(value: &str) -> Result<String, ServiceRuntimeError> {
    let token = value.trim();
    if token.is_empty() {
        return Err(ServiceRuntimeError::InvalidArgument(
            "runtime cancellation token must not be empty".into(),
        ));
    }
    Ok(token.to_string())
}

fn metadata_flag(metadata: &BTreeMap<String, String>, key: &str) -> bool {
    metadata
        .get(key)
        .map(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

fn metadata_stream_frame_count(
    metadata: &BTreeMap<String, String>,
) -> Result<Option<usize>, ServiceRuntimeError> {
    metadata
        .get(STREAM_FRAME_COUNT_METADATA_KEY)
        .map(|value| {
            value.trim().parse::<usize>().map_err(|_| {
                ServiceRuntimeError::InvalidArgument(
                    "stream.frame_count metadata must be a positive integer".into(),
                )
            })
        })
        .transpose()
}

fn output_stream_frame_count(output: &serde_json::Value) -> Option<usize> {
    output
        .get("stream_frames")
        .or_else(|| output.get("frames"))
        .and_then(|value| value.as_array())
        .map(Vec::len)
}

fn timeout_error(timeout: Duration) -> ServiceRuntimeError {
    ServiceRuntimeError::CallTimedOut {
        timeout_ms: duration_millis(timeout),
    }
}

fn cancelled_error(control: &ServiceRuntimeCallControl) -> ServiceRuntimeError {
    ServiceRuntimeError::CallCancelled {
        cancellation_token: control
            .cancellation_token
            .as_deref()
            .map(audit_token_ref)
            .unwrap_or_else(|| "runtime_signal".into()),
    }
}

fn duration_millis(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

fn audit_token_ref(token: &str) -> String {
    format!("redacted:length:{}", token.len())
}

fn runtime_control_metadata_keys() -> [&'static str; 4] {
    [
        TIMEOUT_METADATA_KEY,
        CANCELLATION_TOKEN_METADATA_KEY,
        CANCELLED_METADATA_KEY,
        CANCEL_REQUESTED_METADATA_KEY,
    ]
}
