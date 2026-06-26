use std::collections::BTreeMap;

use serde_json::Value;

use crate::{
    CleanupPolicy, MacacaError, ServiceCallResult, ServiceCommand, ServiceError, ServiceResult,
    TraceContext,
};

/// Common result builder used by descriptor-owned domain-pack service adapters.
///
/// Package providers may add bounded metadata inside the output payload, but OS-level
/// observability must stay provider-neutral through the `provider_class` dimension.
pub fn domain_pack_service_result(
    output: Value,
    trace: TraceContext,
    provider_class: &'static str,
) -> ServiceCallResult {
    let mut metadata = BTreeMap::new();
    metadata.insert("provider_class".into(), provider_class.into());
    ServiceCallResult {
        output,
        trace,
        status: "ok".into(),
        metadata,
        cleanup_hint: Some(CleanupPolicy::None),
    }
}

/// Extract a trace context or fail before domain-pack provider logic runs.
///
/// Every domain-pack call must be trace-addressable before side effects so audit replay can
/// correlate optional package providers without exposing provider payloads.
pub fn domain_pack_command_trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
    command
        .trace
        .clone()
        .ok_or(ServiceError::MissingTraceContext)
}

/// Map bounded OS errors into structured service-unavailable responses for package adapters.
pub fn domain_pack_service_adapter_error(error: MacacaError) -> ServiceError {
    ServiceError::ServiceUnavailable(error.to_string())
}
