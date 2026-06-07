//! System prompt construction through the macaca-context composer boundary.

use std::sync::Arc;
use macaca_agent::AgentCapabilitySet;
use macaca_app::app_agent_prompt_semantics;
use macaca_context::{ContextSourceKind, PromptComposer, PromptStability, TrustLevel};
use macaca_persist::AppendEventCommand;
use macaca_proto::ApplicationId;
use macaca_sdk::AgentPersona;
use crate::state::AppState;
use super::FrameworkRunner;
use super::prompt_helpers::{load_workspace_guide_sections, prompt_section};
use super::skill_policy::resolve_agent_skill_policy;

/// Load the agent's persona and build the system prompt through the context boundary.
pub(crate) async fn build_context_system_prompt(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    agent_name: &str,
    session_id: Option<String>,
    capabilities: &AgentCapabilitySet,
) -> String {
    let merged_context = FrameworkRunner::resolve_context_config(state, app_id, agent_name).await;
        let app_manifest = {
            let registry = crate::application_shell_adapter::registry_read_guard(&state).await;
            registry.get_app(app_id).map(|app| app.manifest.clone())
        };
        let app_dir = {
            let dirs = state.config.app_dirs.read().await;
            dirs.iter()
                .find(|(id, _)| **id == *app_id)
                .map(|(_, path)| path.clone())
        };

        let persona = if let Some(ref dir) = app_dir {
            let persona_dir = dir.join("personas").join(agent_name);
            if persona_dir.exists() {
                AgentPersona::load_from_directory(&persona_dir).await.ok()
            } else {
                None
            }
        } else {
            None
        };

        let mut composer = PromptComposer::new();

        let base_prompt = if let Some(ref p) = persona {
            if merged_context.agent_profile.enabled {
                p.to_system_prompt_delegating_profile_files(None)
            } else {
                p.to_system_prompt(None)
            }
        } else if let Some(manifest) = app_manifest.as_ref() {
            app_agent_prompt_semantics(manifest, agent_name).base_prompt
        } else {
            format!("You are the {} agent in Macaca OS.", agent_name)
        };
        composer = composer.push_section(prompt_section(
            "000-base",
            ContextSourceKind::SystemPrompt,
            PromptStability::Stable,
            TrustLevel::Trusted,
            base_prompt,
        ));

        // Inject capabilities from the macaca-agent capability abstraction.
        let flattened = capabilities.flatten_for_legacy_api();
        let caps: Vec<&str> = flattened.iter().map(|cap| cap.name.as_str()).collect();
        if !caps.is_empty() {
            composer = composer.push_section(prompt_section(
                "100-capabilities",
                ContextSourceKind::SystemPrompt,
                PromptStability::Stable,
                TrustLevel::Trusted,
                format!("Your capabilities: {}", caps.join(", ")),
            ));
        }

        if let Some(ref dir) = app_dir {
            for section in
                load_workspace_guide_sections(dir, &merged_context.workspace_guides).await
            {
                composer = composer.push_section(section);
            }
        }

        // Inject workspace paths
        let workspace_root = {
            let workspaces = state.config.app_workspaces.read().await;
            workspaces.get(app_id).map(|ws| ws.root.clone())
        };
        let skill_policy = resolve_agent_skill_policy(state, app_id, agent_name).await;
        {
            let workspaces = state.config.app_workspaces.read().await;
            if let Some(ws) = workspaces.get(app_id) {
                composer = composer.push_section(prompt_section(
                    "300-workspace-paths",
                    ContextSourceKind::Workspace,
                    PromptStability::Dynamic,
                    TrustLevel::Trusted,
                    format!(
                        "## Workspace Paths\n\
                         - Workspace root (default cwd for file/shell tools): {}\n\
                         - Shared workspace: {}\n\
                         - Your private workspace: {}\n\
                         Relative paths are resolved from the workspace root above. \
                         Create project files in the shared workspace. \
                         Use your private workspace for temporary/scratch files only.",
                        ws.root.display(),
                        ws.shared.display(),
                        ws.agent_workspace(agent_name).display(),
                    ),
                ));
            }
        }

        // Skill discovery telemetry + session cache (Tier-1 progressive disclosure now flows through
        // context composer providers; do not duplicate `snapshot.prompt` here — see
        // `ContextReportingChatModel` capability providers / `capability_catalog`).
        match crate::capability_catalog::resolve_skill_snapshot_cached(
            state,
            app_id,
            agent_name,
            session_id.as_deref(),
            skill_policy,
            workspace_root,
            app_dir,
        )
        .await
        {
            Ok(snapshot) => {
                tracing::info!(
                    agent = %agent_name,
                    visible = snapshot.skills.len(),
                    filtered = snapshot.filtered.len(),
                    truncated = snapshot.truncated,
                    compact = snapshot.compact,
                    "skill_catalog_built"
                );
                if let Some(session_id) = session_id.as_deref() {
                    state
                        .persist
                        .event_log
                        .append_command(AppendEventCommand::new(
                            session_id,
                            "skill_catalog_built",
                            agent_name,
                            serde_json::json!({
                                "agent": agent_name,
                                "visible_count": snapshot.skills.len(),
                                "filtered_count": snapshot.filtered.len(),
                                "truncated": snapshot.truncated,
                                "compact": snapshot.compact,
                            }),
                        ))
                        .await;
                    state
                        .persist
                        .event_log
                        .append_command(AppendEventCommand::new(
                            session_id,
                            "skill_snapshot_created",
                            agent_name,
                            serde_json::json!({
                                "agent": agent_name,
                                "version": snapshot.version,
                                "skills": snapshot.skills.iter().map(|skill| {
                                    serde_json::json!({
                                        "name": skill.name,
                                        "location": skill.location,
                                        "source": skill.source,
                                    })
                                }).collect::<Vec<_>>(),
                                "filtered": snapshot.filtered,
                            }),
                        ))
                        .await;
                }
            }
            Err(error) => {
                tracing::warn!(agent = %agent_name, error = %error, "failed to build skill catalog");
            }
        }

    composer.compile().text
}
