//! Shared support functions for Payment Service command handling.
//!
//! These helpers keep serialization, trace validation, and adapter-error
//! translation consistent across every command path. They are intentionally
//! stateless so providers remain easy to replace and test.

use std::collections::BTreeMap;

use macaca_proto::{
    A2AError, CleanupPolicy, ServiceCallResult, ServiceCommand, ServiceError, ServiceResult,
    TraceContext,
};

use crate::payment_admission::PaymentTraceSpec;

pub fn trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
    let trace = command
        .trace
        .clone()
        .ok_or(ServiceError::MissingTraceContext)?;
    PaymentTraceSpec::check(&trace)?;
    Ok(trace)
}

pub fn result<T: serde::Serialize>(
    value: T,
    trace: TraceContext,
) -> ServiceResult<ServiceCallResult> {
    Ok(ServiceCallResult {
        output: serde_json::to_value(value)
            .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?,
        trace,
        status: "ok".into(),
        metadata: BTreeMap::new(),
        cleanup_hint: Some(CleanupPolicy::None),
    })
}

pub fn decode<T: serde::de::DeserializeOwned>(payload: serde_json::Value) -> ServiceResult<T> {
    serde_json::from_value(payload).map_err(|error| ServiceError::AdapterFailure(error.to_string()))
}

pub fn a2a_error(error: A2AError) -> ServiceError {
    ServiceError::AdapterFailure(error.to_string())
}
