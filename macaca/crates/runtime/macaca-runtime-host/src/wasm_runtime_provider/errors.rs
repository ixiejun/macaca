//! Runtime-host error mapping for WASM provider execution.
//!
//! The concrete adapter can fail while loading, compiling, instantiating, or
//! invoking a module.  This file converts those failures into the stable error
//! taxonomy defined in `macaca-proto` so callers and audit logs never depend on
//! private engine internals.

use macaca_proto::{
    ApplicationAbiError, ApplicationHostCommandResult, PackageRuntimeKind, TraceContext,
    WasmRuntimeErrorKind, WasmRuntimeErrorReport,
};

/// Internal runtime error with a stable provider-neutral category.
#[derive(Debug, Clone)]
pub(crate) struct WasmRuntimeHostError {
    pub kind: WasmRuntimeErrorKind,
    pub message: String,
}

impl WasmRuntimeHostError {
    /// Build a sanitized error shell before it becomes public report data.
    pub(crate) fn new(kind: WasmRuntimeErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// Convert the internal error into the protocol-level report.
    pub(crate) fn report(
        &self,
        runtime_kind: PackageRuntimeKind,
        trace: Option<TraceContext>,
    ) -> WasmRuntimeErrorReport {
        WasmRuntimeErrorReport::new(self.kind.clone(), runtime_kind, self.message.clone(), trace)
    }

    /// Convert the internal error into an ABI error for session creation.
    pub(crate) fn abi_error(&self) -> ApplicationAbiError {
        ApplicationAbiError::RuntimeUnavailable(self.message.clone())
    }
}

/// Convert a runtime report into a structured command result.
pub(crate) fn runtime_error_result(
    report: WasmRuntimeErrorReport,
    trace: Option<TraceContext>,
) -> ApplicationHostCommandResult {
    let mut result = ApplicationHostCommandResult::runtime_unavailable(report.message, trace);
    result
        .metadata
        .insert("reason_code".into(), report.reason_code);
    result
        .metadata
        .insert("runtime_kind".into(), report.runtime_kind.to_string());
    if let Some(trace_id) = report.trace_id {
        result.metadata.insert("trace_id".into(), trace_id);
    }
    result
}
