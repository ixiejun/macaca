//! Shared service-call bridge for the SDK MCP client facade.
//!
//! Every runtime-backed MCP command crosses the generic `SystemServiceClient`
//! boundary through this helper.  Centralizing serialization keeps the
//! service-backed implementation a thin Adapter over command DTOs.

use std::sync::Arc;

use macaca_proto::{MacacaError, MacacaResult, MCP_SERVICE_ID};

use crate::service_client::{ServiceCallCommand, SystemServiceClient};

/// Dispatch a typed MCP command through the generic SDK service client.
///
/// The helper wraps the payload in a [`ServiceCallCommand`], forwards the trace
/// context for audit correlation, and deserializes the service output envelope.
/// It intentionally contains no MCP protocol knowledge beyond service/command ids.
pub(super) async fn call<T, R>(
    service: &Arc<dyn SystemServiceClient>,
    command_name: &str,
    trace: macaca_proto::TraceContext,
    payload: T,
) -> MacacaResult<R>
where
    T: serde::Serialize,
    R: serde::de::DeserializeOwned,
{
    let service_command =
        ServiceCallCommand::new(MCP_SERVICE_ID, command_name, serde_json::to_value(payload)?)?
            .with_trace(trace);
    let result = service.call_service(&service_command).await?;
    serde_json::from_value(result.output).map_err(MacacaError::from)
}
