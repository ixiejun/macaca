//! Hardened WASM daemon transport boundary.
//!
//! This module is the Bridge/Adapter seam between `runtime-host` and a future
//! out-of-process WASM daemon.  The trait deliberately carries only sanitized
//! provider-neutral envelopes and responses, so callers never learn whether the
//! backing implementation is an in-memory test transport, a local child
//! process, a socket daemon, or a remote executor.  That separation keeps the
//! hardened provider as a deployment profile of the existing WASM runtime
//! contract instead of a new Application ABI.

use async_trait::async_trait;
use macaca_proto::TraceContext;

use super::{WasmHardenedProviderEnvelope, WasmHardenedProviderResponse};

/// Health state reported by a hardened transport before session creation.
///
/// The enum stores only stable reason codes and sanitized messages.  This is
/// important because real daemons often fail with process paths, socket names,
/// environment values, or stderr snippets that must not cross into public
/// diagnostics, logs, or certification reports.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum WasmHardenedHealth {
    Healthy,
    Unavailable { reason: String },
    Overloaded { reason: String },
}

impl WasmHardenedHealth {
    /// Return the stable reason code used by provider logs and result metadata.
    pub(crate) fn reason_code(&self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Unavailable { .. } => "daemon_unavailable",
            Self::Overloaded { .. } => "backpressure",
        }
    }

    /// Return a safe human-readable reason without raw daemon material.
    pub(crate) fn safe_reason(&self) -> String {
        match self {
            Self::Healthy => "hardened daemon is healthy".into(),
            Self::Unavailable { reason } | Self::Overloaded { reason } => reason.clone(),
        }
    }
}

/// Provider-private transport trait implemented by daemon adapters.
///
/// The trait is intentionally small: health checks and command dispatch are the
/// only operations needed by the runtime provider.  Cancellation, timeout, and
/// backpressure controls travel inside `WasmHardenedProviderEnvelope` so the
/// transport can evolve without changing the public provider/session traits.
#[async_trait]
pub(crate) trait WasmHardenedTransport: Send + Sync + std::fmt::Debug {
    /// Check daemon readiness with trace correlation before creating a session.
    async fn health(&self, trace: TraceContext) -> WasmHardenedHealth;

    /// Dispatch one sanitized execution envelope to the backing daemon.
    async fn dispatch(
        &self,
        envelope: WasmHardenedProviderEnvelope,
    ) -> WasmHardenedProviderResponse;
}

/// Deterministic transport used by tests and local certification fixtures.
///
/// The in-memory transport follows the same contract as a real daemon adapter:
/// callers must check health, dispatch receives sanitized envelopes, and
/// responses are provider-neutral DTOs.  Keeping this implementation beside the
/// trait lets tests exercise fail-closed semantics without starting an external
/// process or adding OS-specific sandbox behavior.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct InMemoryHardenedTransport {
    health: WasmHardenedHealth,
    response: Option<WasmHardenedProviderResponse>,
}

#[allow(dead_code)]
impl InMemoryHardenedTransport {
    /// Build a healthy transport that echoes successful daemon completion.
    pub(crate) fn healthy() -> Self {
        Self {
            health: WasmHardenedHealth::Healthy,
            response: None,
        }
    }

    /// Build an unavailable transport with a sanitized diagnostic reason.
    pub(crate) fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            health: WasmHardenedHealth::Unavailable {
                reason: sanitize_hardened_text(reason.into()),
            },
            response: None,
        }
    }

    /// Build an overloaded transport with a sanitized backpressure reason.
    #[allow(dead_code)]
    pub(crate) fn overloaded(reason: impl Into<String>) -> Self {
        Self {
            health: WasmHardenedHealth::Overloaded {
                reason: sanitize_hardened_text(reason.into()),
            },
            response: None,
        }
    }

    /// Override the next daemon response for deterministic failure tests.
    pub(crate) fn with_response(mut self, response: WasmHardenedProviderResponse) -> Self {
        self.response = Some(response);
        self
    }

    /// Expose health as an inherent helper for tests and local certification.
    ///
    /// Production code should depend on the `WasmHardenedTransport` trait, but
    /// an inherent method keeps deterministic fixtures easy to call without
    /// requiring every test to import the trait explicitly.
    pub(crate) async fn health(&self, trace: TraceContext) -> WasmHardenedHealth {
        <Self as WasmHardenedTransport>::health(self, trace).await
    }
}

#[async_trait]
impl WasmHardenedTransport for InMemoryHardenedTransport {
    async fn health(&self, _trace: TraceContext) -> WasmHardenedHealth {
        self.health.clone()
    }

    async fn dispatch(
        &self,
        envelope: WasmHardenedProviderEnvelope,
    ) -> WasmHardenedProviderResponse {
        if let Some(response) = &self.response {
            return sanitize_response(response.clone());
        }
        let mut metadata = envelope.metadata.clone();
        metadata.insert("provider_class".into(), "hardened_out_of_process".into());
        metadata.insert("operation".into(), envelope.operation.clone());
        WasmHardenedProviderResponse {
            trace_id: envelope.trace_id,
            request_id: envelope.request_id,
            status: macaca_proto::ApplicationHostCommandStatus::Ok,
            reason_code: "hardened_daemon_completed".into(),
            metadata,
            diagnostics: vec!["hardened daemon completed sanitized dispatch".into()],
        }
    }
}

/// Sanitize daemon-provided response fields before provider validation.
pub(crate) fn sanitize_response(
    mut response: WasmHardenedProviderResponse,
) -> WasmHardenedProviderResponse {
    response.trace_id = sanitize_label(response.trace_id);
    response.request_id = sanitize_label(response.request_id);
    response.reason_code = sanitize_label(response.reason_code);
    response.metadata = response
        .metadata
        .into_iter()
        .map(|(key, value)| (sanitize_label(key), sanitize_hardened_text(value)))
        .collect();
    response.diagnostics = response
        .diagnostics
        .into_iter()
        .map(sanitize_hardened_text)
        .collect();
    response
}

/// Reduce identifiers to safe ASCII labels while preserving trace delimiters.
pub(crate) fn sanitize_label(value: impl Into<String>) -> String {
    let sanitized: String = value
        .into()
        .trim()
        .chars()
        .filter_map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-' | ':') {
                Some(ch)
            } else if ch == '/' {
                Some(':')
            } else {
                None
            }
        })
        .collect();
    if sanitized.is_empty() {
        "sanitized".into()
    } else {
        sanitized
    }
}

/// Remove common raw payload and secret markers from daemon-facing text.
///
/// The sanitizer favors conservative, boring diagnostics.  If text contains a
/// known unsafe marker, the entire message is replaced with a generic sanitized
/// diagnostic instead of trying to redact substrings and risking a partial leak.
pub(crate) fn sanitize_hardened_text(value: impl Into<String>) -> String {
    let value = value.into();
    let lower = value.to_ascii_lowercase();
    let unsafe_marker = [
        "raw_payload",
        "raw payload",
        "raw wasm",
        "raw manifest",
        "secret",
        "api_key",
        "private_key",
        " env",
        "prompt",
    ]
    .iter()
    .any(|marker| lower.contains(marker));
    if unsafe_marker {
        "sanitized hardened provider diagnostic".into()
    } else {
        sanitize_label(value)
    }
}
