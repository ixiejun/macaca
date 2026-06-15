//! Agent Context service snapshot builder for replayable system-context evidence.

use super::context_prompt_builder;
use super::skill_policy::resolve_agent_skill_policy;
use super::FrameworkRunner;
use crate::state::AppState;
use macaca_proto::AppendEventCommand;
use std::sync::Arc;

impl FrameworkRunner {
    pub(crate) async fn build_agent_context_snapshot(
        state: &Arc<AppState>,
        command: macaca_proto::AgentContextBuildCommand,
    ) -> macaca_proto::AgentContextSnapshot {
        let capabilities = FrameworkRunner::resolve_agent_capability_set(
            state,
            &command.application_id,
            &command.target_agent,
        )
        .await;
        let merged_context = FrameworkRunner::resolve_context_config(
            state,
            &command.application_id,
            &command.target_agent,
        )
        .await;
        let app_dir = {
            let dirs = state.config.app_dirs.read().await;
            dirs.iter()
                .find(|(id, _)| **id == command.application_id)
                .map(|(_, path)| path.clone())
        };
        let workspace_root = {
            let workspaces = state.config.app_workspaces.read().await;
            workspaces
                .get(&command.application_id)
                .map(|ws| ws.root.clone())
        };
        let system_prompt = context_prompt_builder::build_context_system_prompt(
            state,
            &command.application_id,
            &command.target_agent,
            Some(command.session_id.clone()),
            &capabilities,
        )
        .await;
        let mut snapshot = macaca_proto::AgentContextSnapshot::minimal(&command, system_prompt);
        snapshot.sources.push(macaca_proto::AgentContextSource {
            kind: "persona_or_manifest".into(),
            name: command.target_agent.clone(),
            location: app_dir.as_ref().map(|dir| dir.display().to_string()),
            metadata: std::collections::BTreeMap::from([
                (
                    "agent_profile_enabled".into(),
                    merged_context.agent_profile.enabled.to_string(),
                ),
                (
                    "workspace_guide_entries".into(),
                    merged_context.workspace_guides.entries.len().to_string(),
                ),
            ]),
        });
        if merged_context.agent_profile.enabled && merged_context.agent_profile.inject_heartbeat {
            if let Some(profile_root) = FrameworkRunner::resolve_agent_profile_root(
                state,
                &command.application_id,
                &command.target_agent,
                &merged_context.agent_profile,
            )
            .await
            {
                let heartbeat_path = profile_root.join("HEARTBEAT.md");
                if heartbeat_path.is_file() {
                    // Record source evidence for heartbeat-intent execution.
                    // The Agent Context composer remains responsible for the
                    // actual profile-file content injection. This evidence row
                    // gives Agent Execution a fail-closed, auditable proof that
                    // HEARTBEAT.md participated in the trusted profile source
                    // set without reading or duplicating prompt text here.
                    snapshot.sources.push(macaca_proto::AgentContextSource {
                        kind: "profile_file".into(),
                        name: "HEARTBEAT.md".into(),
                        location: Some(heartbeat_path.display().to_string()),
                        metadata: std::collections::BTreeMap::from([(
                            "source_owner".into(),
                            "agent_context".into(),
                        )]),
                    });
                }
            }
        }
        snapshot.sources.push(macaca_proto::AgentContextSource {
            kind: "tool_policy".into(),
            name: command.target_agent.clone(),
            location: None,
            metadata: command.policy.metadata.clone(),
        });
        snapshot.tool_policy.insert(
            "capability_scope".into(),
            serde_json::to_string(&command.policy.capability_scope).unwrap_or_else(|_| "[]".into()),
        );
        snapshot.tool_policy.insert(
            "required_permissions".into(),
            serde_json::to_string(&command.policy.required_permissions)
                .unwrap_or_else(|_| "[]".into()),
        );
        let skill_policy =
            resolve_agent_skill_policy(state, &command.application_id, &command.target_agent).await;
        match crate::capability_catalog::resolve_skill_snapshot_cached(
            state,
            &command.application_id,
            &command.target_agent,
            Some(&command.session_id),
            skill_policy,
            workspace_root,
            app_dir.clone(),
        )
        .await
        {
            Ok(skill_snapshot) => {
                snapshot.visible_skills = skill_snapshot
                    .skills
                    .iter()
                    .map(|skill| skill.name.clone())
                    .collect();
                snapshot.filtered_skills = skill_snapshot
                    .filtered
                    .iter()
                    .map(|filtered| filtered.name.clone())
                    .collect();
                snapshot.sources.push(macaca_proto::AgentContextSource {
                    kind: "skill_snapshot".into(),
                    name: format!("{} skills", command.target_agent),
                    location: None,
                    metadata: std::collections::BTreeMap::from([
                        ("version".into(), skill_snapshot.version.to_string()),
                        ("truncated".into(), skill_snapshot.truncated.to_string()),
                        ("compact".into(), skill_snapshot.compact.to_string()),
                    ]),
                });
            }
            Err(error) => {
                snapshot.sources.push(macaca_proto::AgentContextSource {
                    kind: "skill_snapshot_unavailable".into(),
                    name: command.target_agent.clone(),
                    location: None,
                    metadata: std::collections::BTreeMap::from([(
                        "error".into(),
                        error.to_string(),
                    )]),
                });
            }
        }
        snapshot.metadata.insert(
            "provider".into(),
            "macaca.web.framework_runner.context".into(),
        );
        snapshot.metadata.insert(
            "execution_intent".into(),
            serde_json::to_string(&command.execution_intent).unwrap_or_else(|_| "unknown".into()),
        );
        state
            .persist
            .event_log
            .append_command(AppendEventCommand::new(
                &command.session_id,
                "agent_context_built",
                &command.target_agent,
                serde_json::json!({
                    "agent": command.target_agent,
                    "task_id": command.task_id.map(|id| id.0.to_string()),
                    "execution_intent": command.execution_intent,
                    "trace_id": command.trace.trace_id,
                    "system_prompt_chars": snapshot.system_prompt.chars().count(),
                    "visible_skill_count": snapshot.visible_skills.len(),
                    "filtered_skill_count": snapshot.filtered_skills.len(),
                    "source_count": snapshot.sources.len(),
                    "provider": snapshot.metadata.get("provider").cloned(),
                }),
            ))
            .await;
        snapshot
    }
}
