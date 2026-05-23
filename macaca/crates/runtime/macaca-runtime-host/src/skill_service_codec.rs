//! Shared codec helpers for the built-in Skill service provider.
//!
//! The Skill provider accepts generic service commands at the kernel boundary
//! and immediately converts them into typed DTOs owned by `macaca-skill`.
//! Keeping the conversion and standard response shape in this small module
//! prevents dispatch code from mixing transport concerns with governance logic.

use std::collections::BTreeMap;

use macaca_proto::{
    CleanupPolicy, MacacaError, ServiceCallResult, ServiceError, ServiceResult, TraceContext,
};

/// Wrap a typed Skill service payload in the standard service-call envelope.
///
/// Runtime-host services all return an auditable status, the original trace
/// context, and an explicit cleanup hint.  Skill commands do not allocate
/// command-scoped resources today, so the cleanup hint is intentionally `None`.
pub(crate) fn service_result(output: serde_json::Value, trace: TraceContext) -> ServiceCallResult {
    ServiceCallResult {
        output,
        trace,
        status: "ok".into(),
        metadata: BTreeMap::new(),
        cleanup_hint: Some(CleanupPolicy::None),
    }
}

/// Decode a generic service payload into a command DTO.
///
/// Decode failures are reported as unsupported command shapes because the
/// service command name was recognized, but the payload did not satisfy that
/// command contract.
pub(crate) fn decode<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> ServiceResult<T> {
    serde_json::from_value(value).map_err(|err| ServiceError::UnsupportedCommand(err.to_string()))
}

/// Encode a typed service result into the generic service payload slot.
///
/// Serialization failures are adapter failures because the provider produced a
/// Rust value that could not be represented at the service boundary.
pub(crate) fn to_value<T: serde::Serialize>(value: T) -> ServiceResult<serde_json::Value> {
    serde_json::to_value(value).map_err(|err| ServiceError::AdapterFailure(err.to_string()))
}

/// Convert domain facade errors into the kernel service error model.
pub(crate) fn service_adapter_error(err: MacacaError) -> ServiceError {
    ServiceError::AdapterFailure(err.to_string())
}
