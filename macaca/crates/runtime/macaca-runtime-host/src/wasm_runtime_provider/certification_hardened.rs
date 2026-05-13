//! Hardened WASM provider envelope and mock Adapter.
//!
//! Out-of-process execution is modeled here as a deployment profile, not a new
//! Application ABI semantic.  The envelope and response are provider-neutral
//! DTOs that future daemons can adopt while current runtime tests exercise
//! trace propagation, timeout, cancellation, backpressure, and diagnostics
//! controls without launching an external process.

use std::collections::BTreeMap;

use macaca_proto::ApplicationHostCommandStatus;
use serde::{Deserialize, Serialize};
use tracing::info;

/// Data-only request envelope for a future hardened out-of-process provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmHardenedProviderEnvelope {
    pub trace_id: String,
    pub request_id: String,
    pub operation: String,
    pub timeout_ms: Option<u64>,
    pub cancellation_token: Option<String>,
    pub backpressure_window: Option<u64>,
    pub diagnostics_level: String,
    pub metadata: BTreeMap<String, String>,
}

impl WasmHardenedProviderEnvelope {
    /// Build an envelope with required trace, request, and operation controls.
    pub fn new(
        trace_id: impl Into<String>,
        request_id: impl Into<String>,
        operation: impl Into<String>,
    ) -> Self {
        Self {
            trace_id: sanitize_label(trace_id.into()),
            request_id: sanitize_label(request_id.into()),
            operation: sanitize_label(operation.into()),
            timeout_ms: None,
            cancellation_token: None,
            backpressure_window: None,
            diagnostics_level: "sanitized".into(),
            metadata: BTreeMap::new(),
        }
    }

    /// Attach a bounded timeout control in milliseconds.
    pub fn timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    /// Attach a cancellation token identifier without exposing process handles.
    pub fn cancellation_token(mut self, token: impl Into<String>) -> Self {
        self.cancellation_token = Some(sanitize_label(token.into()));
        self
    }

    /// Attach a backpressure window used by future streaming providers.
    pub fn backpressure_window(mut self, window: u64) -> Self {
        self.backpressure_window = Some(window);
        self
    }

    /// Attach the diagnostics level requested from the hardened provider.
    pub fn diagnostics_level(mut self, level: impl Into<String>) -> Self {
        self.diagnostics_level = sanitize_label(level.into());
        self
    }
}

/// Sanitized response returned by the hardened provider mock Adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmHardenedProviderResponse {
    pub trace_id: String,
    pub request_id: String,
    pub status: ApplicationHostCommandStatus,
    pub reason_code: String,
    pub metadata: BTreeMap<String, String>,
    pub diagnostics: Vec<String>,
}

impl WasmHardenedProviderResponse {
    /// Return whether response metadata and diagnostics avoid raw material.
    pub fn is_sanitized(&self) -> bool {
        serde_json::to_string(self)
            .map(|encoded| is_sanitized_text(&encoded))
            .unwrap_or(false)
    }
}

/// Mock Adapter for the hardened out-of-process provider contract.
#[derive(Debug, Clone)]
pub struct WasmHardenedProviderMockAdapter {
    adapter_id: String,
}

impl WasmHardenedProviderMockAdapter {
    /// Create a mock adapter with a stable sanitized identifier.
    pub fn new(adapter_id: impl Into<String>) -> Self {
        Self {
            adapter_id: sanitize_label(adapter_id.into()),
        }
    }

    /// Dispatch one envelope and return provider-neutral status metadata.
    pub fn dispatch(&self, envelope: WasmHardenedProviderEnvelope) -> WasmHardenedProviderResponse {
        let mut metadata = BTreeMap::new();
        metadata.insert(
            "provider_profile".into(),
            "hardened_out_of_process_mock".into(),
        );
        metadata.insert("adapter_id".into(), self.adapter_id.clone());
        metadata.insert("operation".into(), envelope.operation.clone());
        metadata.insert(
            "diagnostics_level".into(),
            envelope.diagnostics_level.clone(),
        );
        if let Some(timeout_ms) = envelope.timeout_ms {
            metadata.insert("timeout_ms".into(), timeout_ms.to_string());
        }
        if let Some(window) = envelope.backpressure_window {
            metadata.insert("backpressure_window".into(), window.to_string());
        }
        if envelope.cancellation_token.is_some() {
            metadata.insert("cancellation".into(), "supported".into());
        }
        info!(
            adapter_id = %self.adapter_id,
            trace_id = %envelope.trace_id,
            request_id = %envelope.request_id,
            operation = %envelope.operation,
            "WASM hardened provider mock adapter dispatched envelope"
        );
        WasmHardenedProviderResponse {
            trace_id: envelope.trace_id,
            request_id: envelope.request_id,
            status: ApplicationHostCommandStatus::Ok,
            reason_code: "hardened_mock_completed".into(),
            metadata,
            diagnostics: vec!["hardened provider mock completed sanitized dispatch".into()],
        }
    }
}

fn sanitize_label(value: impl Into<String>) -> String {
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
        "fixture".into()
    } else {
        sanitized
    }
}

fn is_sanitized_text(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    ![
        "raw_payload",
        "raw manifest",
        "raw wasm",
        "must-not-leak",
        "secret",
        "api_key",
        "private_key",
        "prompt",
        " env",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}
