//! Shared Adapter helpers for autonomy system service providers.
//!
//! **Pattern:** Adapter support utilities — decode generic `ServiceCommand`
//! envelopes into typed DTOs, preserve trace context, and shape provider-neutral
//! `ServiceCallResult` replies without embedding scheduling or heartbeat semantics.

use macaca_proto::{
    MacacaError, ServiceCallResult, ServiceCommand, ServiceError, ServiceResult, TraceContext,
};

/// Extract the required trace context from an incoming service command.
///
/// Autonomy adapters reject commands without trace because Scheduler, Heartbeat,
/// and Scheduled Agent Task audit trails must remain correlated end-to-end.
pub(crate) fn command_trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
    command
        .trace
        .clone()
        .ok_or(ServiceError::MissingTraceContext)
}

/// Decode a JSON payload into a typed autonomy service command DTO.
///
/// Invalid payloads surface as `InvalidArgument` so callers receive structured
/// errors instead of panicking on malformed envelopes.
pub(crate) fn decode<T>(payload: serde_json::Value) -> ServiceResult<T>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_value(payload).map_err(|error| {
        ServiceError::InvalidArgument(format!("invalid autonomy service payload: {error}"))
    })
}

/// Wrap a typed provider result into the generic service-runtime reply envelope.
///
/// Serialization failures are treated as adapter faults because the provider
/// already produced a value object; only the bridge encoding step failed.
pub(crate) fn service_result<T>(output: T, trace: TraceContext) -> ServiceResult<ServiceCallResult>
where
    T: serde::Serialize,
{
    Ok(ServiceCallResult {
        output: serde_json::to_value(output).map_err(|error| {
            ServiceError::InvalidArgument(format!("autonomy result serialization failed: {error}"))
        })?,
        trace,
        status: "ok".into(),
        metadata: Default::default(),
        cleanup_hint: Some(macaca_proto::CleanupPolicy::None),
    })
}

/// Map domain `MacacaError` values into `ServiceUnavailable` for syscall boundaries.
pub(crate) fn service_adapter_error(error: MacacaError) -> ServiceError {
    ServiceError::ServiceUnavailable(error.to_string())
}

/// Map `ServiceRuntime` registration/start failures into configuration errors.
pub(crate) fn runtime_error(error: crate::ServiceRuntimeError) -> MacacaError {
    MacacaError::Config(error.to_string())
}
