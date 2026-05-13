//! Provider-neutral WASM runtime error taxonomy and reports.
//!
//! This child module keeps runtime failure DTOs separate from the base provider
//! descriptor/profile DTOs.  It is still data-only and avoids concrete engine
//! errors, raw bytes, host payloads, and secrets.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{PackageRuntimeKind, TraceContext};

use super::{sanitize_diagnostic_text, FORBIDDEN_DIAGNOSTIC_MARKERS};

/// Provider-neutral runtime error categories emitted by WASM providers.
///
/// The enum is an error taxonomy rather than an engine error wrapper.  Runtime
/// providers map Wasmtime, WasmEdge, Wasmer, process isolation, cache, policy,
/// and host bridge failures into these stable categories before returning data
/// across the protocol boundary.  Keeping the list small and explicit makes
/// telemetry, audits, retries, and policy rules comparable across providers.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WasmRuntimeErrorKind {
    RuntimeUnavailable,
    ArtifactLoadFailed,
    CompileFailed,
    AbiMismatch,
    InstantiateFailed,
    InvokeFailed,
    Trap,
    Timeout,
    PolicyDenied,
    ResourceExhausted,
    MissingTrace,
    Unsupported,
}

impl WasmRuntimeErrorKind {
    /// Return the stable snake_case reason code used in logs and reports.
    pub fn as_code(&self) -> &'static str {
        match self {
            Self::RuntimeUnavailable => "runtime_unavailable",
            Self::ArtifactLoadFailed => "artifact_load_failed",
            Self::CompileFailed => "compile_failed",
            Self::AbiMismatch => "abi_mismatch",
            Self::InstantiateFailed => "instantiate_failed",
            Self::InvokeFailed => "invoke_failed",
            Self::Trap => "trap",
            Self::Timeout => "timeout",
            Self::PolicyDenied => "policy_denied",
            Self::ResourceExhausted => "resource_exhausted",
            Self::MissingTrace => "missing_trace",
            Self::Unsupported => "unsupported",
        }
    }
}

/// Sanitized, provider-neutral error report for runtime execution failures.
///
/// This report is the auditable form of runtime failure data.  It intentionally
/// stores a stable error kind, reason code, runtime family, optional trace id,
/// and bounded metadata, while excluding raw WASM bytes, command payloads,
/// environment variables, secrets, and concrete engine errors.  Providers
/// should log this structure or a subset of it at failure boundaries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmRuntimeErrorReport {
    pub kind: WasmRuntimeErrorKind,
    pub reason_code: String,
    pub message: String,
    pub runtime_kind: PackageRuntimeKind,
    pub trace_id: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

impl WasmRuntimeErrorReport {
    /// Build a sanitized report from a provider failure.
    pub fn new(
        kind: WasmRuntimeErrorKind,
        runtime_kind: PackageRuntimeKind,
        message: impl Into<String>,
        trace: Option<TraceContext>,
    ) -> Self {
        let reason_code = kind.as_code().to_string();
        Self {
            kind,
            reason_code,
            message: sanitize_diagnostic_text(message),
            runtime_kind,
            trace_id: trace.map(|value| value.trace_id),
            metadata: BTreeMap::new(),
        }
    }

    /// Return whether this report avoids known forbidden raw material.
    pub fn is_sanitized(&self) -> bool {
        let lower = self.message.to_lowercase();
        !FORBIDDEN_DIAGNOSTIC_MARKERS
            .iter()
            .any(|marker| lower.contains(marker))
    }
}
