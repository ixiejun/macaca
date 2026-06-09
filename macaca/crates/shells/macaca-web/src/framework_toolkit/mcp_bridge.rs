//! MCP service bridge: registration, snapshot loading, and auditable runtime events.
//!
//! Web shell adapts typed MCP service commands into legacy EventLog/SSE lifecycle
//! events. Canonical MCP runtime ownership remains with `service.mcp`; this module
//! is an Adapter that preserves UI-visible trace contracts during migration.

use std::sync::Arc;

use macaca_proto::{
    ApplicationId, McpRegisterCommand, McpServicePolicyHints, McpServiceScope,
    McpServiceSnapshotCommand, TraceContext,
};
use macaca_sdk::runtime_host::{
    McpDefinitionSource, McpRegistryConfig, McpRuntimeStatus, McpRuntimeStatusState,
    McpServerDefinition,
};

use crate::runtime_event_bridge::emit_runtime_event;
use crate::state::AppState;

pub(super) async fn register_mcp_definitions_with_service(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: Option<&str>,
    agent_name: &str,
    definitions: &[McpServerDefinition],
) {
    if definitions.is_empty() {
        return;
    }
    let payloads = definitions
        .iter()
        .filter_map(|definition| match serde_json::to_value(definition) {
            Ok(value) => Some(value),
            Err(error) => {
                tracing::warn!(
                    server_id = %definition.id,
                    error = %error,
                    "failed to serialize MCP definition for service registration"
                );
                None
            }
        })
        .collect::<Vec<_>>();
    if payloads.is_empty() {
        return;
    }
    let command = McpRegisterCommand {
        trace: TraceContext::new("web-toolkit-mcp-register"),
        scope: McpServiceScope::agent_session(*app_id, session_id.unwrap_or_default(), agent_name)
            .unwrap_or_default(),
        definitions: payloads,
        policy: McpServicePolicyHints::default(),
    };
    if let Err(error) = state.mcp_client.register(command).await {
        tracing::warn!(
            error = %error,
            "MCP Service registration failed during toolkit assembly"
        );
    }
}

pub(super) async fn emit_driver_catalog_unavailable(
    state: &Arc<AppState>,
    session_id: Option<&str>,
    agent_name: &str,
    reason: String,
) {
    let Some(session_id) = session_id else {
        return;
    };

    // This event is a compact audit Memento: it preserves the service boundary
    // failure, traceable actor, and policy decision without leaking provider
    // internals or trying to emulate Driver Service behavior in Web.
    emit_runtime_event(
        state,
        session_id,
        "driver_catalog_unavailable",
        agent_name,
        Some(agent_name),
        serde_json::json!({
            "agent": agent_name,
            "service": "driver",
            "stage": "toolkit_catalog",
            "policy_decision": "unavailable",
            "reason": bounded_diagnostic_reason(reason),
        }),
    )
    .await;
}

fn bounded_diagnostic_reason(reason: String) -> String {
    let mut value = reason.replace('\n', " ").replace('\r', " ");
    value.truncate(240);
    value
}

pub(super) async fn load_service_mcp_snapshot_definitions(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: Option<&str>,
    agent_name: &str,
) -> Vec<McpServerDefinition> {
    let command = McpServiceSnapshotCommand {
        trace: TraceContext::new("web-toolkit-mcp-snapshot"),
        scope: McpServiceScope::agent_session(*app_id, session_id.unwrap_or_default(), agent_name)
            .unwrap_or_default(),
        include_definitions: true,
    };

    // The MCP service snapshot is the typed command boundary for definition
    // visibility.  Web deserializes the optional payloads only to preserve the
    // existing registration/probe bridge during migration; ownership of the
    // canonical runtime inventory remains with the MCP service provider.
    match state.mcp_client.snapshot(command).await {
        Ok(snapshot) => {
            tracing::info!(
                app_id = %app_id,
                session_id = session_id.unwrap_or_default(),
                agent = agent_name,
                registered_definitions = snapshot.registered_definitions,
                emitted_definitions = snapshot.definitions.len(),
                "MCP Service snapshot loaded for toolkit assembly"
            );
            snapshot
                .definitions
                .into_iter()
                .filter_map(|payload| {
                    match serde_json::from_value::<McpServerDefinition>(payload) {
                        Ok(definition) => Some(definition),
                        Err(error) => {
                            tracing::warn!(
                                app_id = %app_id,
                                agent = agent_name,
                                error = %error,
                                "MCP Service snapshot contained an unreadable definition payload"
                            );
                            None
                        }
                    }
                })
                .collect()
        }
        Err(error) => {
            tracing::warn!(
                app_id = %app_id,
                session_id = session_id.unwrap_or_default(),
                agent = agent_name,
                error = %error,
                "MCP Service snapshot unavailable during toolkit assembly"
            );
            Vec::new()
        }
    }
}

pub(super) async fn load_app_mcp_overlay_definitions(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
) -> Vec<McpServerDefinition> {
    let app = {
        let registry = crate::application_shell_adapter::registry_read_guard(&state).await;
        registry.get_app(app_id).cloned()
    };
    let Some(app) = app else {
        return Vec::new();
    };
    let path = app.path.join("mcp.yaml");
    if !path.exists() {
        return Vec::new();
    }
    let Ok(content) = tokio::fs::read_to_string(&path).await else {
        return Vec::new();
    };
    match serde_yaml::from_str::<McpRegistryConfig>(&content)
        .map_err(|e| e.to_string())
        .and_then(|config| {
            macaca_sdk::runtime_host::McpServerFactory::with_bundled_mapping_registry()
                .from_registry_config(config, McpDefinitionSource::App)
        }) {
        Ok(definitions) => definitions,
        Err(error) => {
            tracing::warn!(
                app_id = %app_id,
                path = %path.display(),
                error = %error,
                "Failed to load app MCP overlay"
            );
            Vec::new()
        }
    }
}

pub(super) async fn emit_mcp_runtime_events(
    state: &Arc<AppState>,
    session_id: Option<&str>,
    agent_name: &str,
    statuses: &[McpRuntimeStatus],
) {
    let Some(session_id) = session_id else {
        return;
    };

    for plan in mcp_runtime_event_plans(agent_name, statuses) {
        emit_mcp_event(
            state,
            session_id,
            plan.event_type,
            agent_name,
            &plan.payload,
        )
        .await;
    }
}

pub(super) async fn emit_skill_mcp_alias_events(
    state: &Arc<AppState>,
    session_id: Option<&str>,
    agent_name: &str,
    statuses: &[McpRuntimeStatus],
) {
    let Some(session_id) = session_id else {
        return;
    };
    for plan in skill_mcp_alias_event_plans(agent_name, statuses) {
        emit_mcp_event(
            state,
            session_id,
            plan.event_type,
            agent_name,
            &plan.payload,
        )
        .await;
    }
}

pub(super) async fn emit_mcp_starting_events(
    state: &Arc<AppState>,
    session_id: Option<&str>,
    agent_name: &str,
    definitions: &[McpServerDefinition],
) {
    let Some(session_id) = session_id else {
        return;
    };
    for plan in mcp_starting_event_plans(agent_name, definitions) {
        emit_mcp_event(
            state,
            session_id,
            plan.event_type,
            agent_name,
            &plan.payload,
        )
        .await;
    }
}

/// A deterministic EventLog/SSE event plan for MCP lifecycle visibility.
///
/// `build_toolkit` emits these plans through [`emit_runtime_event`], which
/// persists to EventLog before forwarding to SSE.  Keeping this as a pure value
/// object makes the service-backed migration auditable without constructing a
/// full `AppState` in unit tests, while preserving the existing event names and
/// payload shape used by the UI.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct McpRuntimeEventPlan {
    pub(crate) event_type: &'static str,
    pub(crate) payload: serde_json::Value,
}

/// Build the legacy MCP runtime events from service-probed statuses.
///
/// The event ordering is part of the user-visible runtime trace contract:
/// every non-disabled status emits `mcp_server_resolved` first, then a terminal
/// readiness/failure event, and ready servers with exposed tools emit the
/// `mcp_tools_registered` follow-up.
pub(crate) fn mcp_runtime_event_plans(
    agent_name: &str,
    statuses: &[McpRuntimeStatus],
) -> Vec<McpRuntimeEventPlan> {
    let mut plans = Vec::new();
    for status in statuses {
        if matches!(status.state, McpRuntimeStatusState::Disabled) {
            continue;
        }

        let payload = serde_json::json!({
            "agent": agent_name,
            "server_id": status.server_id,
            "transport": status.transport,
            "lifecycle": status.lifecycle,
            "session_mode": status.session_mode,
            "state": status.state,
            "exposed_tools": status.exposed_tools,
            "failure_reason": status.failure_reason,
        });

        plans.push(McpRuntimeEventPlan {
            event_type: "mcp_server_resolved",
            payload: payload.clone(),
        });
        match status.state {
            McpRuntimeStatusState::Ready => {
                plans.push(McpRuntimeEventPlan {
                    event_type: "mcp_server_ready",
                    payload: payload.clone(),
                });
                if !status.exposed_tools.is_empty() {
                    plans.push(McpRuntimeEventPlan {
                        event_type: "mcp_tools_registered",
                        payload,
                    });
                }
            }
            McpRuntimeStatusState::Failed | McpRuntimeStatusState::DependencyMissing => {
                plans.push(McpRuntimeEventPlan {
                    event_type: "mcp_server_failed",
                    payload,
                });
            }
            McpRuntimeStatusState::Disabled => {}
        }
    }
    plans
}

/// Build skill-backed MCP alias events without changing service ownership.
///
/// Skill alias events are a Web/UI compatibility surface.  The service-backed
/// runtime still owns registration and invocation, while this planner preserves
/// the existing alias event stream for users who watch skill-specific MCP
/// readiness in EventLog/SSE.
pub(crate) fn skill_mcp_alias_event_plans(
    agent_name: &str,
    statuses: &[McpRuntimeStatus],
) -> Vec<McpRuntimeEventPlan> {
    let mut plans = Vec::new();
    for status in statuses
        .iter()
        .filter(|status| status.server_id.starts_with("skill:"))
    {
        let payload = serde_json::json!({
            "agent": agent_name,
            "server_id": status.server_id,
            "state": status.state,
            "exposed_tools": status.exposed_tools,
            "failure_reason": status.failure_reason,
        });
        let event_type = match status.state {
            McpRuntimeStatusState::Ready => "skill_mcp_ready",
            McpRuntimeStatusState::Failed | McpRuntimeStatusState::DependencyMissing => {
                "skill_mcp_failed"
            }
            McpRuntimeStatusState::Disabled => "skill_mcp_disabled",
        };
        plans.push(McpRuntimeEventPlan {
            event_type,
            payload: payload.clone(),
        });
        if matches!(status.state, McpRuntimeStatusState::Ready) && !status.exposed_tools.is_empty()
        {
            plans.push(McpRuntimeEventPlan {
                event_type: "skill_mcp_tools_registered",
                payload,
            });
        }
    }
    plans
}

/// Build `mcp_server_starting` events for enabled definitions.
///
/// Starting events intentionally happen before the service registration call in
/// `build_toolkit`, matching the old direct-registration lifecycle narrative
/// while allowing the actual runtime state to be owned by `service.mcp`.
pub(crate) fn mcp_starting_event_plans(
    agent_name: &str,
    definitions: &[McpServerDefinition],
) -> Vec<McpRuntimeEventPlan> {
    definitions
        .iter()
        .filter(|definition| definition.enabled)
        .map(|definition| McpRuntimeEventPlan {
            event_type: "mcp_server_starting",
            payload: serde_json::json!({
                "agent": agent_name,
                "server_id": definition.id,
                "lifecycle": definition.lifecycle,
                "session_mode": definition.session_mode,
                "state": "starting",
            }),
        })
        .collect()
}

async fn emit_mcp_event(
    state: &Arc<AppState>,
    session_id: &str,
    event_type: &str,
    source: &str,
    payload: &serde_json::Value,
) {
    emit_runtime_event(
        state,
        session_id,
        event_type,
        source,
        payload.get("agent").and_then(|value| value.as_str()),
        payload.clone(),
    )
    .await;
}
