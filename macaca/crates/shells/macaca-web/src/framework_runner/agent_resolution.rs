//! Agent capability, model, and context-config resolution helpers.

use super::FrameworkRunner;
use crate::state::AppState;
use macaca_host_composition::app::app_agent_manifest_view;
use macaca_host_composition::app::model::AppContextConfig;
use macaca_host_composition::framework::construction::AgentCapabilitySet;
use macaca_proto::config::{AgentProfileContextConfig, AgentProfileRootKind, ContextConfig};
use macaca_proto::{ApplicationId, Capability};
use std::sync::Arc;

impl FrameworkRunner {
    pub(crate) async fn resolve_agent_capability_set(
        state: &Arc<AppState>,
        app_id: &ApplicationId,
        agent_name: &str,
    ) -> AgentCapabilitySet {
        {
            let registry = crate::application_shell_adapter::registry_read_guard(&state).await;
            if let Some(app) = registry.get_app(app_id) {
                if let Some(agent) = app_agent_manifest_view(&app.manifest, agent_name) {
                    return AgentCapabilitySet::from_flat_capabilities(
                        agent
                            .capabilities()
                            .iter()
                            .map(|capability| Capability {
                                name: capability.name.clone(),
                                description: capability.description.clone(),
                            })
                            .collect(),
                    );
                }
            }
        }
        let manifests = state.kernel.list_agents().await;
        let capabilities = manifests
            .into_iter()
            .find(|manifest| manifest.name == agent_name)
            .map(|manifest| manifest.capabilities)
            .unwrap_or_default();
        AgentCapabilitySet::from_flat_capabilities(capabilities)
    }

    /// Resolve the routed model selection for an agent.
    /// Priority: agent manifest model > app llm_config > system default.
    pub(crate) async fn resolve_model_selection(
        state: &Arc<AppState>,
        app_id: &ApplicationId,
        agent_name: &str,
        session_id: Option<&str>,
    ) -> Result<macaca_host_composition::llm::ModelSelection, String> {
        let request_model = if let Some(session_id) = session_id {
            state
                .sessions
                .llm_route_hints
                .read()
                .await
                .get(session_id)
                .cloned()
        } else {
            None
        };
        let agent_model = state
            .kernel
            .get_agent_by_name(agent_name)
            .await
            .and_then(|manifest| (!manifest.model.is_empty()).then_some(manifest.model));

        let app_defaults =
            crate::application_shell_adapter::manifest_llm_config(state, app_id).await;

        crate::llm_route_shell_adapter::resolve_model_selection(
            state,
            app_id,
            agent_name,
            session_id,
            macaca_host_composition::llm::ModelSelectionRequest {
                request_model: request_model.clone(),
                agent_model,
                app_model: app_defaults.as_ref().map(|cfg| cfg.model.clone()),
                app_provider: app_defaults.as_ref().map(|cfg| cfg.provider.clone()),
                system_model: (!state.config.default_model.is_empty())
                    .then_some(state.config.default_model.clone()),
                ..Default::default()
            },
        )
        .await
    }

    pub(crate) async fn resolve_context_config(
        state: &Arc<AppState>,
        app_id: &ApplicationId,
        agent_name: &str,
    ) -> ContextConfig {
        let mut config = state.config.context.clone();
        let registry = crate::application_shell_adapter::registry_read_guard(&state).await;
        if let Some(app) = registry.get_app(app_id) {
            let agent_engine = app_agent_manifest_view(&app.manifest, agent_name)
                .and_then(|agent| agent.context_engine().map(str::to_owned))
                .filter(|value| !value.is_empty());
            config = Self::merge_context_config_overrides(
                config,
                app.manifest.context.as_ref(),
                agent_engine.as_deref(),
            );
        }
        config
    }

    /// Merge system-level context defaults with optional app and agent overrides.
    ///
    /// Precedence is intentionally narrow and deterministic:
    /// 1. system config provides the base
    /// 2. app context may override default/fallback engine and supporting profile fields
    /// 3. agent context engine may override only the primary engine selection
    pub(crate) fn merge_context_config_overrides(
        mut config: ContextConfig,
        app_context: Option<&AppContextConfig>,
        agent_engine: Option<&str>,
    ) -> ContextConfig {
        if let Some(app_context) = app_context {
            if let Some(engine) = app_context
                .engine
                .as_ref()
                .filter(|value| !value.is_empty())
            {
                config.default_engine = engine.clone();
            }
            if let Some(fallback) = app_context
                .fallback_engine
                .as_ref()
                .filter(|value| !value.is_empty())
            {
                config.fallback_engine = fallback.clone();
            }
            if let Some(ref guides) = app_context.workspace_guides {
                config.workspace_guides = guides.clone();
            }
            if let Some(ref ap) = app_context.agent_profile {
                config.agent_profile = ap.clone();
            }
        }
        if let Some(engine) = agent_engine.filter(|value| !value.is_empty()) {
            config.default_engine = engine.to_string();
        }
        config
    }

    /// Resolves the on-disk directory scanned by [`macaca_host_composition::context::ProfileFileContextProvider`].
    ///
    /// The path is never interpreted as a workflow name — only as filesystem layout dictated by
    /// [`AgentProfileRootKind`] and the active [`ApplicationId`].
    pub(crate) async fn resolve_agent_profile_root(
        state: &Arc<AppState>,
        app_id: &ApplicationId,
        agent_name: &str,
        cfg: &AgentProfileContextConfig,
    ) -> Option<std::path::PathBuf> {
        if !cfg.enabled {
            return None;
        }
        match cfg.root_kind {
            AgentProfileRootKind::PersonaDirectory => {
                let dirs = state.config.app_dirs.read().await;
                let app_dir = dirs
                    .iter()
                    .find(|(id, _)| **id == *app_id)
                    .map(|(_, path)| path.clone())?;
                Some(app_dir.join("personas").join(agent_name))
            }
            AgentProfileRootKind::AgentPrivateWorkspace => {
                let workspaces = state.config.app_workspaces.read().await;
                workspaces
                    .get(app_id)
                    .map(|ws| ws.agent_workspace(agent_name))
            }
        }
    }
}
