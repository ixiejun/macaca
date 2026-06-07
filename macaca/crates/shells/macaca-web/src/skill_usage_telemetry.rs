//! Web adapter for governed Skill usage telemetry.
//!
//! The Skill service owns telemetry counters and lifecycle records. This module
//! is deliberately a thin Observer/Command adapter: it observes a generic
//! Web-side execution boundary, reloads only bounded cached snapshot metadata,
//! and sends typed usage commands through the SDK Skill client. It never parses
//! prompts, reads full `SKILL.md` bodies, branches on application names, or
//! decides whether a Skill produced a quality improvement.

use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use macaca_proto::{AgentExecutionResult, ApplicationId, TraceContext};
use macaca_sdk::skill::{
    SkillGovernanceRecord, SkillGovernanceRecordUsageCommand, SkillGovernanceSnapshotCommand,
    SkillLifecycleState, SkillServiceScope, SkillSnapshot, SkillUsageEventKind,
    SkillUsageObservation,
};

use crate::state::AppState;

/// Record a successful task outcome for governed Skills visible to the agent.
///
/// This function is best-effort by design. Agent Execution has already
/// completed, so telemetry failures must not change user-visible task success.
/// The cached snapshot is treated as the bounded memento proving which governed
/// Skills were visible to the agent during the run; missing cache evidence is a
/// skip, not an inferred failure or success.
pub(crate) async fn record_governed_skill_successful_task(
    state: &Arc<AppState>,
    result: &AgentExecutionResult,
) {
    let session_id = result.session_id.as_str();
    let agent_name = result.target_agent.as_str();
    let snapshot_module = format!("skill_snapshot/{agent_name}");

    let snapshot = match state
        .sessions
        .framework_session_store
        .load(session_id, &snapshot_module)
        .await
    {
        Ok(Some(value)) => match serde_json::from_value::<SkillSnapshot>(value) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                tracing::warn!(
                    session_id,
                    agent = agent_name,
                    error = %error,
                    "skipping governed Skill task outcome telemetry because cached snapshot could not be decoded"
                );
                return;
            }
        },
        Ok(None) => {
            tracing::info!(
                session_id,
                agent = agent_name,
                "skipping governed Skill task outcome telemetry because no cached Skill snapshot exists"
            );
            return;
        }
        Err(error) => {
            tracing::warn!(
                session_id,
                agent = agent_name,
                error = %error,
                "skipping governed Skill task outcome telemetry because snapshot cache lookup failed"
            );
            return;
        }
    };

    let scope = match SkillServiceScope::agent(result.application_id, session_id, agent_name) {
        Ok(scope) => scope,
        Err(error) => {
            tracing::warn!(
                app_id = %result.application_id,
                session_id,
                agent = agent_name,
                error = %error,
                "skipping governed Skill task outcome telemetry because scope is invalid"
            );
            return;
        }
    };

    let governance = match state
        .skill_client
        .governance_snapshot(SkillGovernanceSnapshotCommand {
            trace: TraceContext::new("web-skill-task-outcome-telemetry"),
            scope: scope.clone(),
            include_archived: false,
            lifecycle_filters: vec![SkillLifecycleState::Active],
        })
        .await
    {
        Ok(snapshot) => snapshot,
        Err(error) => {
            tracing::warn!(
                app_id = %result.application_id,
                session_id,
                agent = agent_name,
                error = %error,
                "governed Skill task outcome telemetry snapshot lookup failed"
            );
            return;
        }
    };

    let commands = build_governed_skill_task_outcome_usage_commands(
        &snapshot,
        &governance.records,
        result.application_id,
        session_id,
        agent_name,
        result.task_id.map(|task_id| task_id.to_string()).as_deref(),
        result.trace.trace_id.as_str(),
        SkillUsageEventKind::SuccessfulTask,
    );

    for command in commands {
        let skill_id = command.observation.skill_id.clone();
        if let Err(error) = state.skill_client.record_governance_usage(command).await {
            tracing::warn!(
                app_id = %result.application_id,
                session_id,
                agent = agent_name,
                skill_id,
                error = %error,
                "governed Skill task outcome telemetry recording failed"
            );
        }
    }
}

/// Build usage commands for active governed Skills visible in a Skill snapshot.
///
/// The snapshot provides the model-visible name set; governance provides the
/// authoritative Skill ids and provenance. Joining on the visible name mirrors
/// the existing activation telemetry contract while keeping the command payload
/// free of full Skill instructions and raw task output.
pub(crate) fn build_governed_skill_task_outcome_usage_commands(
    snapshot: &SkillSnapshot,
    governance_records: &[SkillGovernanceRecord],
    app_id: ApplicationId,
    session_id: &str,
    agent_name: &str,
    task_id: Option<&str>,
    trace_id: &str,
    event: SkillUsageEventKind,
) -> Vec<SkillGovernanceRecordUsageCommand> {
    let Ok(scope) = SkillServiceScope::agent(app_id, session_id, agent_name) else {
        return Vec::new();
    };
    let visible_skill_names = snapshot
        .skills
        .iter()
        .map(|skill| skill.name.as_str())
        .collect::<HashSet<_>>();

    governance_records
        .iter()
        .filter(|record| record.lifecycle == SkillLifecycleState::Active)
        .filter(|record| visible_skill_names.contains(record.provenance.name.as_str()))
        .map(|record| {
            let mut metadata = BTreeMap::new();
            metadata.insert("activation_surface".into(), "agent_execution_result".into());
            metadata.insert("agent_name".into(), agent_name.into());
            metadata.insert("session_id".into(), session_id.into());
            metadata.insert("snapshot_version".into(), snapshot.version.to_string());
            if let Some(task_id) = task_id {
                metadata.insert("task_id".into(), task_id.into());
            }

            SkillGovernanceRecordUsageCommand {
                trace: TraceContext::new(trace_id),
                scope: scope.clone(),
                observation: SkillUsageObservation {
                    skill_id: record.provenance.skill_id.clone(),
                    name: record.provenance.name.clone(),
                    source: record.provenance.source.clone(),
                    source_scope: record.provenance.source_scope.clone(),
                    event: event.clone(),
                    author_kind: record.provenance.author_kind.clone(),
                    created_by: record.provenance.created_by.clone(),
                    pinned: Some(record.pinned),
                    evidence_id: Some(format!(
                        "eventlog://sessions/{session_id}/agent_execution/{agent_name}/{}",
                        task_id.unwrap_or("unknown-task")
                    )),
                    metadata,
                },
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use macaca_sdk::skill::{
        SkillAuthorKind, SkillGovernanceProvenance, SkillSnapshotEntry, SkillSourceScope,
    };

    fn snapshot_with_visible_skill(name: &str) -> SkillSnapshot {
        SkillSnapshot {
            agent: "coordinator".into(),
            prompt: String::new(),
            skills: vec![SkillSnapshotEntry {
                name: name.into(),
                description: "visible governed skill".into(),
                source_location: PathBuf::from("/tmp/skills/visible/SKILL.md"),
                source_base_dir: PathBuf::from("/tmp/skills/visible"),
                location: PathBuf::from("/tmp/available/visible/SKILL.md"),
                base_dir: PathBuf::from("/tmp/available/visible"),
                source: "workspace".into(),
                source_scope: SkillSourceScope::Workspace,
                primary_env: None,
                required_env: Vec::new(),
                install: Vec::new(),
                mcp_servers: Vec::new(),
            }],
            filtered: Vec::new(),
            truncated: false,
            compact: false,
            version: 7,
        }
    }

    fn governance_record(name: &str, lifecycle: SkillLifecycleState) -> SkillGovernanceRecord {
        SkillGovernanceRecord {
            provenance: SkillGovernanceProvenance::new(
                format!("skill://agent/{name}"),
                name,
                "skill.evolution",
                "proposal",
                SkillAuthorKind::Agent,
            ),
            lifecycle,
            pinned: false,
            telemetry: Default::default(),
            diagnostics: Default::default(),
            updated_at: chrono::Utc::now(),
            evidence_ids: vec!["eventlog://proposal".into()],
        }
    }

    #[test]
    fn successful_task_usage_command_targets_only_active_visible_governed_skills() {
        let app_id = ApplicationId(uuid::Uuid::new_v4());
        let active_visible = governance_record(
            "materialization-registry-telemetry",
            SkillLifecycleState::Active,
        );
        let archived_visible = governance_record(
            "materialization-registry-telemetry",
            SkillLifecycleState::Archived,
        );
        let active_hidden = governance_record("hidden-skill", SkillLifecycleState::Active);

        let commands = build_governed_skill_task_outcome_usage_commands(
            &snapshot_with_visible_skill("materialization-registry-telemetry"),
            &[active_visible, archived_visible, active_hidden],
            app_id,
            "session-live",
            "coordinator",
            Some("task-live"),
            "trace-live",
            SkillUsageEventKind::SuccessfulTask,
        );

        assert_eq!(commands.len(), 1);
        assert_eq!(
            commands[0].observation.event,
            SkillUsageEventKind::SuccessfulTask
        );
        assert_eq!(
            commands[0].observation.skill_id,
            "skill://agent/materialization-registry-telemetry"
        );
        assert_eq!(
            commands[0]
                .observation
                .metadata
                .get("activation_surface")
                .map(String::as_str),
            Some("agent_execution_result")
        );
        assert_eq!(
            commands[0]
                .observation
                .metadata
                .get("task_id")
                .map(String::as_str),
            Some("task-live")
        );
    }
}
