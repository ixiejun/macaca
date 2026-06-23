//! Application shell adapter — service-first application/registry access.
//!
//! Routes and framework adapters must not read `AppRuntime` or `AppRegistry` through
//! `AppState` directly. This module centralizes the **Adapter** pattern for:
//! - primary reads through `SystemApplicationClient`
//! - bounded manifest reads that still require package-local declarations

use std::sync::Arc;

use macaca_host_composition::app::AppLlmConfig;
use macaca_host_composition::app::AppRegistry;
use macaca_proto::{
    AgentExecutionCommand, AgentId, AgentManifest, ApplicationId, ApplicationMetadataQueryCommand,
    ApplicationServiceScope, ApplicationStatusCommand, TraceContext,
};
use tokio::sync::RwLockReadGuard;

use crate::state::AppState;

/// Borrow the application registry read lock from the composition bundle.
///
/// Callers must prefer `SystemApplicationClient::metadata` when sanitized views
/// contain the required declaration data. This helper is limited to package-
/// local declarations that are not yet projected by Application Service.
pub async fn registry_read_guard(state: &Arc<AppState>) -> RwLockReadGuard<'_, AppRegistry> {
    tracing::trace!("application shell adapter acquiring registry read guard");
    state.composition.registry.read().await
}

/// Count running applications for status surfaces.
pub async fn running_app_count(state: &Arc<AppState>) -> usize {
    let command = ApplicationStatusCommand {
        trace: TraceContext::new("web-shell-adapter-app-count"),
        scope: ApplicationServiceScope::default(),
    };
    match state.application_client.status(command).await {
        Ok(views) => views.len(),
        Err(error) => {
            tracing::warn!(
                error = %error,
                "application shell adapter status failed; returning zero running applications"
            );
            0
        }
    }
}

/// Return the kernel agent ids that the Application Runtime registered for one app.
///
/// This Adapter is the approved shell boundary for reading the in-process
/// `AppRuntime` memento. The Application Service metadata projection exposes
/// sanitized agent names, but the runtime owns the authoritative app-to-agent-id
/// binding that prevents duplicate role names or reload leftovers from
/// polluting another application's status view. Callers use an empty result as
/// structured "unavailable" evidence and should never fall back to global
/// name-only matching when app isolation matters.
pub async fn app_runtime_agent_ids(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    trace_id: &'static str,
) -> Vec<AgentId> {
    match state.composition.runtime.app_agents(app_id).await {
        Ok(agent_ids) => {
            tracing::info!(
                trace_id,
                application_id = %app_id,
                runtime_agent_id_count = agent_ids.len(),
                "application shell adapter resolved runtime agent ids"
            );
            agent_ids
        }
        Err(error) => {
            tracing::warn!(
                trace_id,
                application_id = %app_id,
                error = %error,
                "application shell adapter could not resolve runtime agent ids"
            );
            Vec::new()
        }
    }
}

/// Select the application-scoped agent manifests that belong to one runtime app.
///
/// Application Service metadata is the authoritative app boundary for declared
/// agent names, while `AppRuntime::app_agents` is the authoritative runtime
/// binding for the kernel ids created during bootstrap. This pure Strategy
/// combines both read models: it preserves declaration order, returns at most
/// one manifest per declared name, and only selects manifests whose id is bound
/// to the requested application. Missing runtime bindings deliberately produce
/// no selected manifest because a name-only match is not a safe application
/// boundary in the terminal single-path architecture.
pub(crate) fn select_app_scoped_agent_manifests(
    manifests: Vec<AgentManifest>,
    runtime_agent_ids: &[AgentId],
    service_agent_names: Option<&[String]>,
) -> Vec<AgentManifest> {
    let runtime_agent_ids: std::collections::HashSet<AgentId> =
        runtime_agent_ids.iter().copied().collect();

    if let Some(service_agent_names) = service_agent_names {
        let mut selected = Vec::with_capacity(service_agent_names.len());
        let mut consumed_names = std::collections::HashSet::new();

        for declared_name in service_agent_names {
            if !consumed_names.insert(declared_name.as_str()) {
                continue;
            }

            let preferred = manifests.iter().find(|manifest| {
                manifest.name == *declared_name && runtime_agent_ids.contains(&manifest.id)
            });

            if let Some(manifest) = preferred {
                selected.push(manifest.clone());
            }
        }

        return selected;
    }

    manifests
        .into_iter()
        .filter(|manifest| runtime_agent_ids.contains(&manifest.id))
        .collect()
}

/// Resolve the kernel manifest for the app-local target agent in an execution command.
///
/// The Agent Execution service receives an app-local agent name, while the
/// kernel status tracker is keyed by `AgentId`. This Adapter performs the
/// provider-neutral join through Application Service metadata and AppRuntime
/// id mementos, then falls back only within the runtime id set. It deliberately
/// avoids global name-only matching so one application's "coder" cannot update
/// another application's status after reloads or multi-app startup.
pub(crate) async fn resolve_app_scoped_agent_manifest(
    state: &Arc<AppState>,
    command: &AgentExecutionCommand,
) -> Option<AgentManifest> {
    let runtime_agent_ids = app_runtime_agent_ids(
        state,
        &command.application_id,
        "web-agent-execution-activity-runtime-agent-ids",
    )
    .await;
    let manifests = state.kernel.list_agents().await;
    if let Ok(metadata_command) =
        ApplicationMetadataQueryCommand::application(command.trace.clone(), command.application_id)
    {
        match state.application_client.metadata(metadata_command).await {
            Ok(view) => {
                let declared_names = view
                    .application
                    .agents
                    .iter()
                    .map(|agent| agent.name.clone())
                    .collect::<Vec<_>>();
                let selected = select_app_scoped_agent_manifests(
                    manifests.clone(),
                    &runtime_agent_ids,
                    Some(&declared_names),
                );
                if let Some(manifest) = selected
                    .into_iter()
                    .find(|manifest| manifest.name == command.target_agent)
                {
                    return Some(manifest);
                }
                tracing::warn!(
                    application_id = %command.application_id,
                    session_id = %command.session_id,
                    target_agent = %command.target_agent,
                    declared_agent_count = declared_names.len(),
                    runtime_agent_id_count = runtime_agent_ids.len(),
                    "application metadata did not contain target agent manifest for activity update"
                );
            }
            Err(error) => tracing::warn!(
                application_id = %command.application_id,
                session_id = %command.session_id,
                target_agent = %command.target_agent,
                error = %error,
                "application metadata query failed while resolving app-scoped activity target"
            ),
        }
    }

    select_app_scoped_agent_manifests(manifests, &runtime_agent_ids, None)
        .into_iter()
        .find(|manifest| manifest.name == command.target_agent)
}

/// Load manifest-declared LLM defaults from package-local declarations.
pub async fn manifest_llm_config(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
) -> Option<AppLlmConfig> {
    let registry = registry_read_guard(state).await;
    registry
        .get_app(app_id)
        .and_then(|app| app.manifest.llm_config.clone())
}

/// Detect WASM layer from package-local declarations when metadata is absent.
pub async fn is_registry_wasm_layer_app(state: &Arc<AppState>, app_id: &ApplicationId) -> bool {
    let registry = registry_read_guard(state).await;
    registry
        .get_app(app_id)
        .map(|app| app.manifest.layer == macaca_host_composition::app::AppLayer::L2Wasm)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use macaca_proto::{AgentState, Capability, Permission, PermissionLevel};

    const FIXTURE_ENTRY_AGENT: &str = "entry-agent";
    const FIXTURE_PLAN_AGENT: &str = "plan-agent";

    fn test_agent_manifest(name: &str, capability: &str) -> AgentManifest {
        AgentManifest {
            id: AgentId::new(),
            name: name.to_string(),
            capabilities: vec![Capability {
                name: capability.to_string(),
                description: String::new(),
            }],
            permission: Permission {
                level: PermissionLevel::User,
                allowed_tools: Vec::new(),
                allowed_paths: Vec::new(),
                network_access: false,
            },
            state: AgentState::Created,
            created_at: Utc::now(),
            model: String::new(),
        }
    }

    #[test]
    fn app_scoped_agent_selection_prefers_runtime_ids_for_duplicate_names() {
        let previous_entry = test_agent_manifest(FIXTURE_ENTRY_AGENT, "todo_goal_management");
        let app_entry = test_agent_manifest(FIXTURE_ENTRY_AGENT, "coding_session_coordination");
        let planner = test_agent_manifest(FIXTURE_PLAN_AGENT, "code_change_planning");
        let coder = test_agent_manifest("coder", "patch_authoring");
        let reviewer = test_agent_manifest("reviewer", "structured_review");
        let previous_planner = test_agent_manifest(FIXTURE_PLAN_AGENT, "todo_planning");
        let runtime_ids = vec![app_entry.id, planner.id, coder.id, reviewer.id];
        let declared_names = vec![
            FIXTURE_ENTRY_AGENT.to_string(),
            FIXTURE_PLAN_AGENT.to_string(),
            "coder".to_string(),
            "reviewer".to_string(),
        ];

        let selected = select_app_scoped_agent_manifests(
            vec![
                previous_entry,
                planner.clone(),
                coder.clone(),
                reviewer.clone(),
                previous_planner,
                app_entry.clone(),
            ],
            &runtime_ids,
            Some(&declared_names),
        );

        assert_eq!(selected.len(), 4);
        assert_eq!(
            selected
                .iter()
                .map(|agent| agent.name.as_str())
                .collect::<Vec<_>>(),
            vec![FIXTURE_ENTRY_AGENT, FIXTURE_PLAN_AGENT, "coder", "reviewer"]
        );
        assert_eq!(selected[0].id, app_entry.id);
        assert_eq!(
            selected[0].capabilities[0].name,
            "coding_session_coordination"
        );
    }
}
