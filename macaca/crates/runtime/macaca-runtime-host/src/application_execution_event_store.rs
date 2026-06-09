//! EventLog adapter for the application execution protocol.
//!
//! Macaca already has a durable append-only session EventLog.  This adapter
//! reuses that infrastructure instead of introducing a parallel store.  The
//! service writes one sanitized `ApplicationExecutionEventEnvelope` per EventLog
//! row, assigns the final sequence, and reconstructs replay/current-state views
//! from the persisted rows.

use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::Utc;
use macaca_persist::{AppendEventCommand, EventLog};
use macaca_proto::{
    audit_redaction::{is_sensitive_json_key, REDACTED_JSON_VALUE},
    AppendExecutionEventCommand, ApplicationExecutionCurrentState,
    ApplicationExecutionEventEnvelope, ApplicationExecutionEventType, ApplicationExecutionPayload,
    ApplicationExecutionProviderLease, ApplicationExecutionReplayRequest,
    ApplicationExecutionReplayResult, ApplicationExecutionScope, ServiceError,
};
use tracing::{info, warn};

use crate::application_execution_projection::project_application_execution_state;
use crate::application_execution_service_logs::log_event_append;

const EVENT_SOURCE: &str = "service.application_execution";
const MAX_INLINE_PAYLOAD_BYTES: usize = 32 * 1024;
const SUPPORTED_SCHEMA_VERSION: &str = "application-execution.v1";
const MAX_SUMMARY_CHARS: usize = 512;
const MAX_DISPLAY_FIELD_CHARS: usize = 24 * 1024;
const MAX_SANITIZED_JSON_DEPTH: usize = 12;
const MAX_SANITIZED_ARRAY_ITEMS: usize = 64;

/// EventLog-backed repository for application execution facts.
#[derive(Clone)]
pub struct ApplicationExecutionEventStore {
    event_log: Arc<EventLog>,
    max_inline_payload_bytes: usize,
    leases: BTreeMap<String, ApplicationExecutionProviderLease>,
}

impl ApplicationExecutionEventStore {
    /// Build a repository around the shared host EventLog.
    pub fn new(event_log: Arc<EventLog>) -> Self {
        Self {
            event_log,
            max_inline_payload_bytes: MAX_INLINE_PAYLOAD_BYTES,
            leases: BTreeMap::new(),
        }
    }

    /// Add one scoped provider lease to the validation book.
    pub fn with_lease(mut self, lease: ApplicationExecutionProviderLease) -> Self {
        self.leases.insert(lease.lease_id.clone(), lease);
        self
    }

    /// Validate gateway identity/lease data, then append the enclosed event.
    pub async fn append_gateway_event(
        &self,
        command: AppendExecutionEventCommand,
    ) -> Result<ApplicationExecutionEventEnvelope, ServiceError> {
        self.validate_gateway_ingress(
            command.lease_id.as_deref(),
            &command.callback_identity_ref,
            Some(&ApplicationExecutionScope {
                application_id: command.event.application_id,
                session_id: command.event.session_id.clone(),
                run_id: command.event.run_id.clone(),
                tenant_id: None,
                actor: command.event.actor.clone(),
            }),
            Some(&command.event.provider_id),
            command.event.event_type,
        )?;
        self.append_idempotent(command.event).await
    }

    /// Validate gateway lease and callback binding before helper-built events append.
    ///
    /// This is a reusable Specification for gateway DTOs that do not carry a
    /// full event envelope yet.  It accepts only protocol metadata and returns a
    /// `ServiceError` before the caller constructs or appends any durable event.
    pub(crate) fn validate_gateway_ingress(
        &self,
        lease_id: Option<&str>,
        callback_identity_ref: &str,
        scope: Option<&ApplicationExecutionScope>,
        provider_id: Option<&str>,
        event_type: ApplicationExecutionEventType,
    ) -> Result<(), ServiceError> {
        if callback_identity_ref.trim().is_empty() {
            return Err(ServiceError::InvalidArgument(
                "application execution callback identity is required".into(),
            ));
        }
        let Some(lease_id) = lease_id else {
            return Ok(());
        };
        let Some(lease) = self.leases.get(lease_id) else {
            return Err(ServiceError::InvalidArgument(
                "application execution stale lease: lease is not registered".into(),
            ));
        };
        let now = Utc::now();
        if lease.expires_at <= now || lease.heartbeat_deadline <= now {
            return Err(ServiceError::InvalidArgument(
                "application execution stale lease: lease or heartbeat deadline expired".into(),
            ));
        }
        if lease.callback_identity_ref != callback_identity_ref {
            return Err(ServiceError::InvalidArgument(
                "application execution callback identity does not match lease".into(),
            ));
        }
        if let Some(provider_id) = provider_id {
            if lease.provider_id != provider_id {
                return Err(ServiceError::InvalidArgument(
                    "application execution provider binding does not match lease".into(),
                ));
            }
        }
        if let Some(scope) = scope {
            if lease.scope.application_id != scope.application_id
                || lease.scope.session_id != scope.session_id
                || lease.scope.run_id != scope.run_id
            {
                return Err(ServiceError::InvalidArgument(
                    "application execution session/run binding does not match lease".into(),
                ));
            }
        }
        if !lease.allowed_event_types.is_empty()
            && !lease
                .allowed_event_types
                .iter()
                .any(|allowed| allowed == &event_type)
        {
            return Err(ServiceError::InvalidArgument(
                "application execution event type is not allowed by lease".into(),
            ));
        }
        Ok(())
    }

    /// Append one protocol event idempotently and return the persisted envelope.
    ///
    /// Idempotency is scoped by application/session/run/idempotency key.  When a
    /// duplicate is observed, the previously persisted row is returned without a
    /// second side effect.  This lets external backends and remote agents retry
    /// callbacks safely after network loss.
    pub async fn append_idempotent(
        &self,
        mut event: ApplicationExecutionEventEnvelope,
    ) -> Result<ApplicationExecutionEventEnvelope, ServiceError> {
        self.validate_event(&event)?;
        self.sanitize_event(&mut event)?;
        self.validate_event(&event)?;
        if let Some(existing) = self.find_duplicate(&event).await {
            info!(
                application_id = %event.application_id,
                session_id = %event.session_id,
                run_id = %event.run_id,
                idempotency_key = %event.idempotency_key,
                "application execution event append deduplicated"
            );
            return Ok(existing);
        }

        let event_type = format!("{:?}", event.event_type);
        let app_id = event.application_id.to_string();
        let payload = serde_json::to_value(&event).map_err(adapter_error)?;
        let mut command =
            AppendEventCommand::new(&event.session_id, event_type, EVENT_SOURCE, payload);
        command = command.with_app_id(app_id);
        let seq = self.event_log.append_command(command).await;
        event.seq = Some(seq);

        // EventLog assigns the sequence after payload serialization.  Append a
        // second immutable protocol row is unacceptable, so replay conversion
        // treats EventLog.seq as authoritative even if the stored envelope had
        // `seq = None`.  Returning the assigned value here gives callers the
        // immediate cursor they need for replay/control references.
        log_event_append(
            &event.application_id.to_string(),
            &event.session_id,
            &event.run_id,
            &event.provider_id,
            event.provider_kind,
            &format!("{:?}", event.event_type),
            &event.trace.trace_id,
            seq,
        );
        Ok(event)
    }

    /// Return the previously persisted idempotent event without appending.
    ///
    /// Control delivery needs this read-before-side-effect hook so a retry can
    /// reuse the durable cursor while avoiding a second provider call.  The
    /// method still applies the same structural validation as append so callers
    /// cannot turn malformed protocol envelopes into silent cache hits.
    pub(crate) async fn existing_idempotent_event(
        &self,
        event: &ApplicationExecutionEventEnvelope,
    ) -> Result<Option<ApplicationExecutionEventEnvelope>, ServiceError> {
        self.validate_event(event)?;
        Ok(self.find_duplicate(event).await)
    }

    /// Replay persisted protocol events and return the deterministic projection.
    pub async fn replay(
        &self,
        request: ApplicationExecutionReplayRequest,
    ) -> Result<ApplicationExecutionReplayResult, ServiceError> {
        let since_seq = request
            .from_cursor
            .as_deref()
            .and_then(parse_cursor)
            .unwrap_or(0);
        let page_size = request.page_size.clamp(1, 500);
        let mut events = Vec::new();
        for row in self
            .event_log
            .query(&request.session_id, since_seq, page_size)
            .await
        {
            let Some(event) = self.row_to_event(row.seq, row.payload) else {
                continue;
            };
            if event.application_id != request.application_id {
                continue;
            }
            if let Some(run_id) = &request.run_id {
                if &event.run_id != run_id {
                    continue;
                }
            }
            if let Some(visibility) = &request.visibility {
                if &event.visibility != visibility {
                    continue;
                }
            }
            if !request.event_types.is_empty()
                && !request
                    .event_types
                    .iter()
                    .any(|kind| kind == &event.event_type)
            {
                continue;
            }
            events.push(event);
        }
        let next_cursor = events.last().and_then(|event| event.seq).map(cursor);
        let current_state = events.first().map(|first| {
            let scope = ApplicationExecutionScope {
                application_id: first.application_id,
                session_id: first.session_id.clone(),
                run_id: first.run_id.clone(),
                tenant_id: None,
                actor: first.actor.clone(),
            };
            project_application_execution_state(scope, &events, next_cursor.clone())
        });
        Ok(ApplicationExecutionReplayResult {
            events,
            next_cursor,
            current_state,
        })
    }

    /// Return the latest current-state projection for one scope.
    pub async fn current_state(
        &self,
        scope: ApplicationExecutionScope,
    ) -> Result<ApplicationExecutionCurrentState, ServiceError> {
        let request = ApplicationExecutionReplayRequest {
            application_id: scope.application_id,
            session_id: scope.session_id.clone(),
            run_id: Some(scope.run_id.clone()),
            from_cursor: None,
            page_size: 500,
            event_types: Vec::new(),
            visibility: None,
            trace: macaca_proto::TraceContext::new("application-execution-current-state"),
        };
        let replay = self.replay(request).await?;
        Ok(project_application_execution_state(
            scope,
            &replay.events,
            replay.next_cursor,
        ))
    }

    fn validate_event(
        &self,
        event: &ApplicationExecutionEventEnvelope,
    ) -> Result<(), ServiceError> {
        if event.trace.trace_id.trim().is_empty() {
            return Err(ServiceError::MissingTraceContext);
        }
        if event.schema_version != SUPPORTED_SCHEMA_VERSION {
            return Err(ServiceError::InvalidArgument(format!(
                "unsupported application execution event schema version '{}'",
                event.schema_version
            )));
        }
        if event.session_id.trim().is_empty()
            || event.run_id.trim().is_empty()
            || event.actor.trim().is_empty()
            || event.provider_id.trim().is_empty()
            || event.idempotency_key.trim().is_empty()
            || event.schema_version.trim().is_empty()
        {
            return Err(ServiceError::InvalidArgument(
                "application execution event is missing required scope fields".into(),
            ));
        }
        let payload_size = serde_json::to_vec(&event.sanitized_payload)
            .map_err(adapter_error)?
            .len();
        if payload_size > self.max_inline_payload_bytes && event.payload_ref.is_none() {
            return Err(ServiceError::InvalidArgument(
                "application execution event payload exceeds inline limit without payload_ref"
                    .into(),
            ));
        }
        Ok(())
    }

    fn sanitize_event(
        &self,
        event: &mut ApplicationExecutionEventEnvelope,
    ) -> Result<(), ServiceError> {
        sanitize_payload(&mut event.sanitized_payload)?;
        if let Some(payload_ref) = event.payload_ref.as_mut() {
            payload_ref.redaction_profile =
                normalize_redaction_profile(&payload_ref.redaction_profile);
        }
        Ok(())
    }

    async fn find_duplicate(
        &self,
        event: &ApplicationExecutionEventEnvelope,
    ) -> Option<ApplicationExecutionEventEnvelope> {
        let latest = self.event_log.latest_seq(&event.session_id).await;
        for row in self
            .event_log
            .query(&event.session_id, 0, latest as usize + 1)
            .await
        {
            let candidate = self.row_to_event(row.seq, row.payload)?;
            if candidate.application_id == event.application_id
                && candidate.run_id == event.run_id
                && candidate.idempotency_key == event.idempotency_key
            {
                return Some(candidate);
            }
        }
        None
    }

    fn row_to_event(
        &self,
        seq: u64,
        payload: serde_json::Value,
    ) -> Option<ApplicationExecutionEventEnvelope> {
        let mut event: ApplicationExecutionEventEnvelope = match serde_json::from_value(payload) {
            Ok(event) => event,
            Err(error) => {
                warn!(
                    seq,
                    error = %error,
                    "skipping non application execution EventLog row during replay"
                );
                return None;
            }
        };
        event.seq = Some(seq);
        Some(event)
    }
}

fn parse_cursor(cursor: &str) -> Option<u64> {
    cursor.strip_prefix("event/")?.parse().ok()
}

fn cursor(seq: u64) -> String {
    format!("event/{seq}")
}

fn adapter_error(error: serde_json::Error) -> ServiceError {
    ServiceError::AdapterFailure(error.to_string())
}

/// Sanitize one bounded payload before it enters EventLog, trace, or realtime.
///
/// This is deliberately generic.  It never understands application domains or
/// provider payload formats; it only enforces OS-wide observability rules such
/// as bounded summaries and secret-like field redaction.
fn sanitize_payload(payload: &mut ApplicationExecutionPayload) -> Result<(), ServiceError> {
    let (summary, summary_truncated) = truncate_chars(&payload.summary, MAX_SUMMARY_CHARS);
    payload.summary = summary;
    payload.truncated = payload.truncated || summary_truncated;
    if let Some(data) = payload.data.as_mut() {
        sanitize_json_value(data, 0, None)?;
    }
    Ok(())
}

/// Recursively sanitize structured JSON while preserving display diagnostics.
fn sanitize_json_value(
    value: &mut serde_json::Value,
    depth: usize,
    key: Option<&str>,
) -> Result<(), ServiceError> {
    if depth > MAX_SANITIZED_JSON_DEPTH {
        *value = serde_json::Value::String("[truncated:depth]".into());
        return Ok(());
    }
    match value {
        serde_json::Value::Object(map) => {
            for (child_key, child) in map.iter_mut() {
                if is_sensitive_json_key(child_key) {
                    *child = serde_json::Value::String(REDACTED_JSON_VALUE.into());
                } else {
                    sanitize_json_value(child, depth + 1, Some(child_key))?;
                }
            }
        }
        serde_json::Value::Array(values) => {
            if values.len() > MAX_SANITIZED_ARRAY_ITEMS {
                values.truncate(MAX_SANITIZED_ARRAY_ITEMS);
                values.push(serde_json::Value::String("[truncated:array]".into()));
            }
            for child in values {
                sanitize_json_value(child, depth + 1, key)?;
            }
        }
        serde_json::Value::String(text) if text.chars().count() > string_limit_for_key(key) => {
            let (truncated, _) = truncate_chars(text, string_limit_for_key(key));
            *text = truncated;
        }
        _ => {}
    }
    Ok(())
}

fn string_limit_for_key(key: Option<&str>) -> usize {
    if key.is_some_and(is_display_payload_key) {
        MAX_DISPLAY_FIELD_CHARS
    } else {
        MAX_SUMMARY_CHARS
    }
}

fn is_display_payload_key(key: &str) -> bool {
    matches!(
        key,
        "display_body" | "display_markdown" | "assistant_content"
    )
}

/// Truncate by Unicode scalar count so logs stay bounded without byte slicing.
fn truncate_chars(input: &str, limit: usize) -> (String, bool) {
    if input.chars().count() <= limit {
        return (input.to_string(), false);
    }
    let mut output: String = input.chars().take(limit).collect();
    output.push_str("...[truncated]");
    (output, true)
}

/// Normalize payload-ref redaction metadata to a bounded, non-empty label.
fn normalize_redaction_profile(profile: &str) -> String {
    if profile.trim().is_empty() {
        "application-execution-default".into()
    } else {
        profile.trim().chars().take(128).collect()
    }
}
