//! Provider-neutral WASM host import command and audit contracts.
//!
//! These DTOs describe how a guest import is converted into a bounded Macaca
//! host command without exposing concrete providers, backend clients, raw guest
//! memory, raw payload bytes, environment values, filesystem paths, network
//! targets, prompts, or secrets.  Runtime hosts consume these structures as a
//! Command/Bridge vocabulary; SDKs can produce the same metadata shape without
//! linking to runtime-host internals.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{ApplicationImport, KernelServiceId, ServiceCommandName, TraceContext};

use super::sanitize_diagnostic_text;

/// Stable metadata key carrying the target ServiceRuntime service id.
pub const WASM_HOST_IMPORT_SERVICE_ID: &str = "service.id";
/// Stable metadata key carrying the provider-neutral service operation.
pub const WASM_HOST_IMPORT_OPERATION: &str = "service.operation";
/// Stable metadata key carrying the capability required for the import.
pub const WASM_HOST_IMPORT_CAPABILITY: &str = "capability";

/// Host import category understood by the WASM service portal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WasmHostImportCategory {
    ServiceCall,
    TraceEmit,
    Storage,
    Memory,
    Context,
    Plugin,
    Task,
    Payment,
    Web3,
    Custom,
}

impl WasmHostImportCategory {
    /// Map an Application ABI import into a stable WASM host import category.
    pub fn from_application_import(import: &ApplicationImport) -> Self {
        match import {
            ApplicationImport::ServiceCall => Self::ServiceCall,
            ApplicationImport::TraceEmit => Self::TraceEmit,
            ApplicationImport::StorageGet | ApplicationImport::StorageSet => Self::Storage,
            ApplicationImport::CapabilityRequest => Self::Context,
            ApplicationImport::AgentDelegate
            | ApplicationImport::TaskCreateGoal
            | ApplicationImport::TaskQuery => Self::Task,
            ApplicationImport::UiRender => Self::Custom,
            ApplicationImport::PaymentCreateIntent => Self::Payment,
            ApplicationImport::Custom(_) => Self::Custom,
        }
    }
}

/// Bounded command extracted from an Application ABI host import.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WasmHostImportCommand {
    pub category: WasmHostImportCategory,
    pub import_name: String,
    pub target_service: Option<KernelServiceId>,
    pub operation: Option<ServiceCommandName>,
    pub capability: Option<String>,
    pub payload: serde_json::Value,
    pub payload_bytes: u64,
    pub trace: TraceContext,
    pub metadata: BTreeMap<String, String>,
}

/// Stable host import error categories.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WasmHostImportErrorKind {
    MissingTrace,
    PayloadTooLarge,
    CapabilityMissing,
    PolicyDenied,
    ServiceUnavailable,
    ServiceFailed,
    UnsupportedImport,
    ScopeMissing,
}

impl WasmHostImportErrorKind {
    /// Return the stable reason code used by result metadata and audit logs.
    pub fn as_code(&self) -> &'static str {
        match self {
            Self::MissingTrace => "missing_trace",
            Self::PayloadTooLarge => "payload_too_large",
            Self::CapabilityMissing => "capability_missing",
            Self::PolicyDenied => "policy_denied",
            Self::ServiceUnavailable => "service_unavailable",
            Self::ServiceFailed => "service_failed",
            Self::UnsupportedImport => "unsupported_import",
            Self::ScopeMissing => "scope_missing",
        }
    }
}

/// Sanitized host import audit event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmHostImportAuditReport {
    pub decision: String,
    pub reason_code: String,
    pub import_name: String,
    pub service_id: Option<String>,
    pub operation: Option<String>,
    pub capability: Option<String>,
    pub trace_id: Option<String>,
    pub payload_bytes: u64,
    pub message: String,
    pub metadata: BTreeMap<String, String>,
}

impl WasmHostImportAuditReport {
    /// Build an audit event with sanitized text fields.
    pub fn new(
        decision: impl Into<String>,
        reason_code: impl Into<String>,
        command: &WasmHostImportCommand,
        message: impl Into<String>,
    ) -> Self {
        Self {
            decision: sanitize_label(decision),
            reason_code: sanitize_label(reason_code),
            import_name: sanitize_label(command.import_name.clone()),
            service_id: command
                .target_service
                .as_ref()
                .map(|value| sanitize_label(value.to_string())),
            operation: command
                .operation
                .as_ref()
                .map(|value| sanitize_label(value.to_string())),
            capability: command.capability.clone().map(sanitize_label),
            trace_id: Some(command.trace.trace_id.clone()),
            payload_bytes: command.payload_bytes,
            message: sanitize_diagnostic_text(message),
            metadata: BTreeMap::new(),
        }
    }
}

fn sanitize_label(value: impl Into<String>) -> String {
    let mut sanitized = String::new();
    for ch in sanitize_diagnostic_text(value).chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':' | '.') {
            sanitized.push(ch);
        } else if ch == '/' {
            sanitized.push(':');
        } else {
            sanitized.push('_');
        }
    }
    sanitized.trim_matches('_').to_string()
}
