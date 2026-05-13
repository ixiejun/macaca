//! Production WASM runtime telemetry sinks.
//!
//! Telemetry is implemented as an Observer boundary: providers create
//! sanitized events at important runtime decisions and fan them out to an
//! optional sink.  Sinks are deliberately non-authoritative; they must never
//! change admission, execution, lifecycle, daemon, certification, or host
//! import results.  This keeps observability useful for production audit while
//! preserving deterministic runtime behavior when a sink is missing.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// Stable telemetry stages emitted by WASM runtime industrialization paths.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum WasmTelemetryStage {
    Admission,
    Compile,
    Instantiate,
    Invoke,
    Resource,
    HostImport,
    Lifecycle,
    Daemon,
    Certification,
    SupplyChain,
    Availability,
    Session,
}

impl WasmTelemetryStage {
    /// Return a stable label for logs and external sinks.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Admission => "admission",
            Self::Compile => "compile",
            Self::Instantiate => "instantiate",
            Self::Invoke => "invoke",
            Self::Resource => "resource",
            Self::HostImport => "host_import",
            Self::Lifecycle => "lifecycle",
            Self::Daemon => "daemon",
            Self::Certification => "certification",
            Self::SupplyChain => "supply_chain",
            Self::Availability => "availability",
            Self::Session => "session",
        }
    }
}

/// Sanitized runtime telemetry event.
///
/// The event stores bounded identifiers and reason codes only.  It must not
/// contain raw guest bytes, raw host payloads, raw manifests, secrets,
/// environment values, API keys, private keys, filesystem bodies, network
/// payloads, or unbounded provider diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasmTelemetryEvent {
    pub stage: WasmTelemetryStage,
    pub outcome: String,
    pub provider_class: String,
    pub trace_id: Option<String>,
    pub session_id: Option<String>,
    pub reason_code: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

impl WasmTelemetryEvent {
    /// Build an event with sanitized fields.
    pub fn new(
        stage: WasmTelemetryStage,
        outcome: impl Into<String>,
        provider_class: impl Into<String>,
    ) -> Self {
        Self {
            stage,
            outcome: sanitize_telemetry_text(outcome),
            provider_class: sanitize_telemetry_text(provider_class),
            trace_id: None,
            session_id: None,
            reason_code: None,
            metadata: BTreeMap::new(),
        }
    }

    /// Attach trace context as an identifier only.
    pub fn trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(sanitize_telemetry_text(trace_id));
        self
    }

    /// Attach a provider-local session id.
    pub fn session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(sanitize_telemetry_text(session_id));
        self
    }

    /// Attach a stable reason code.
    pub fn reason_code(mut self, reason_code: impl Into<String>) -> Self {
        self.reason_code = Some(sanitize_telemetry_text(reason_code));
        self
    }

    /// Attach one sanitized metadata pair.
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata
            .insert(sanitize_telemetry_text(key), sanitize_telemetry_text(value));
        self
    }

    /// Report whether the event avoids known sensitive/raw material.
    pub fn is_sanitized(&self) -> bool {
        let encoded = format!("{self:?}").to_ascii_lowercase();
        !FORBIDDEN_TELEMETRY_MARKERS
            .iter()
            .any(|marker| encoded.contains(marker))
    }
}

/// Observer sink for WASM telemetry events.
pub trait WasmTelemetrySink: Send + Sync + std::fmt::Debug {
    /// Emit one sanitized event.  Implementations should be best-effort and
    /// must avoid panicking on downstream sink failures.
    fn emit(&self, event: WasmTelemetryEvent);
}

pub type WasmTelemetrySinkRef = Arc<dyn WasmTelemetrySink>;

/// In-memory sink used by deterministic tests and local certification.
#[derive(Debug, Default)]
pub struct InMemoryWasmTelemetrySink {
    events: Mutex<Vec<WasmTelemetryEvent>>,
}

impl InMemoryWasmTelemetrySink {
    /// Return a snapshot so tests can inspect emitted events without holding
    /// the sink mutex across assertions.
    pub fn events(&self) -> Vec<WasmTelemetryEvent> {
        self.events
            .lock()
            .expect("wasm telemetry sink mutex poisoned")
            .clone()
    }
}

impl WasmTelemetrySink for InMemoryWasmTelemetrySink {
    fn emit(&self, event: WasmTelemetryEvent) {
        if event.is_sanitized() {
            self.events
                .lock()
                .expect("wasm telemetry sink mutex poisoned")
                .push(event);
        }
    }
}

/// Sink that forwards sanitized telemetry to the tracing subscriber.
#[derive(Debug, Default, Clone, Copy)]
pub struct TracingWasmTelemetrySink;

impl WasmTelemetrySink for TracingWasmTelemetrySink {
    fn emit(&self, event: WasmTelemetryEvent) {
        tracing::info!(
            stage = %event.stage.as_str(),
            outcome = %event.outcome,
            provider_class = %event.provider_class,
            trace_id = event.trace_id.as_deref().unwrap_or(""),
            session_id = event.session_id.as_deref().unwrap_or(""),
            reason_code = event.reason_code.as_deref().unwrap_or(""),
            metadata = ?event.metadata,
            "WASM runtime telemetry event emitted"
        );
    }
}

/// Emit to an optional sink without making telemetry part of the control path.
pub(crate) fn emit_wasm_telemetry(sink: Option<&WasmTelemetrySinkRef>, event: WasmTelemetryEvent) {
    if let Some(sink) = sink {
        if event.is_sanitized() {
            sink.emit(event);
        }
    }
}

const FORBIDDEN_TELEMETRY_MARKERS: &[&str] = &[
    "raw wasm",
    "raw_payload",
    "raw payload",
    "raw_manifest",
    "raw manifest",
    "secret",
    "api_key",
    "apikey",
    "private key",
    "env=",
    "prompt",
];

/// Redact common sensitive markers from telemetry fields.
pub fn sanitize_telemetry_text(value: impl Into<String>) -> String {
    let mut sanitized = value.into();
    for marker in FORBIDDEN_TELEMETRY_MARKERS {
        sanitized = sanitized.replace(marker, "[redacted]");
        sanitized = sanitized.replace(&marker.to_ascii_uppercase(), "[redacted]");
    }
    sanitized.chars().take(256).collect()
}
