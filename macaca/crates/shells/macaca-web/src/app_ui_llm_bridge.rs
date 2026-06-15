//! LLM-specific payload hydration for the application-owned UI bridge.
//!
//! The generic UI route owns transport and capability admission. This module is
//! a focused Adapter that turns small UI payloads into typed `service.llm`
//! commands. Keeping the code here prevents the generic asset/bridge route from
//! becoming a God Object while preserving service ownership of provider/model
//! semantics.

use macaca_proto::{
    ApplicationId, LlmCatalogReadCommand, LlmPolicyHints, LlmRouteResolveCommand, LlmServiceScope,
    TraceContext, LLM_MODEL_LIST_COMMAND, LLM_PROVIDER_CAPABILITIES_READ_COMMAND,
    LLM_ROUTE_RESOLVE_COMMAND, LLM_SERVICE_ID,
};
use serde_json::Value;

/// Hydrate an app-owned UI `service.llm` bridge call into a typed command.
///
/// Non-LLM service calls pass through unchanged. For LLM operations, the UI is
/// allowed to provide only small selector fields such as `request_model`; this
/// adapter adds the required application/session/agent scope and trace context
/// before the command reaches the service runtime.
pub(crate) fn hydrate_llm_bridge_payload(
    app_id: ApplicationId,
    session_id: Option<&str>,
    service_id: &str,
    operation: &str,
    payload: &Value,
    trace: TraceContext,
) -> Result<Value, String> {
    if service_id != LLM_SERVICE_ID {
        return Ok(payload.clone());
    }

    let scope = LlmServiceScope::new(
        app_id,
        session_id
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("ui-bridge-sessionless"),
        "application-ui",
    )
    .map_err(|error| error.to_string())?;

    match operation {
        LLM_MODEL_LIST_COMMAND | LLM_PROVIDER_CAPABILITIES_READ_COMMAND => {
            let include_disabled = payload
                .get("include_disabled")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            serde_json::to_value(LlmCatalogReadCommand {
                scope,
                trace,
                include_disabled,
            })
            .map_err(|error| format!("failed to encode LLM catalog command: {error}"))
        }
        LLM_ROUTE_RESOLVE_COMMAND => serde_json::to_value(LlmRouteResolveCommand {
            scope,
            trace,
            request_model: optional_string(payload, "request_model"),
            agent_model: optional_string(payload, "agent_model"),
            app_model: optional_string(payload, "app_model"),
            app_provider: optional_string(payload, "app_provider"),
            system_model: optional_string(payload, "system_model"),
            fallbacks: payload
                .get("fallbacks")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default(),
            policy: LlmPolicyHints::default(),
        })
        .map_err(|error| format!("failed to encode LLM route command: {error}")),
        _ => Ok(payload.clone()),
    }
}

fn optional_string(payload: &Value, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}
