//! Governed skill activation telemetry (Observer side-effect of snapshot materialization).
//!
//! When a skill snapshot becomes visible to an agent, record `Activated` usage events for
//! skills that are both in the snapshot and in the active governance registry.

use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use macaca_app::AppLoader;
use macaca_proto::{ApplicationId, TraceContext};
use macaca_sdk::skill::{
    SkillGovernanceRecord, SkillGovernanceRecordUsageCommand, SkillGovernanceSnapshotCommand,
    SkillLifecycleState, SkillPolicy, SkillServiceScope, SkillSnapshot, SkillUsageEventKind,
    SkillUsageObservation,
};

use crate::state::AppState;

/// Resolve per-agent allow/deny skill exposure policy from the application manifest.
pub(super) async fn resolve_agent_skill_policy(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    agent_name: &str,
) -> SkillPolicy {

    let registry = crate::application_shell_adapter::registry_read_guard(&state).await;
    let Some(app) = registry.get_app(app_id) else {
        return SkillPolicy::default();
    };
    let Ok(agent_configs) = AppLoader::resolve_agent_configs(&app.manifest, &app.path) else {
        return SkillPolicy::default();
    };
    agent_configs
        .into_iter()
        .find(|agent| agent.name == agent_name)
        .and_then(|agent| agent.skills)
        .map(|skills| SkillPolicy {
            allow: skills.allow,
            deny: skills.deny,
        })
        .unwrap_or_default()

}

/// Build usage commands for governed skills that appear in the current snapshot.
pub(crate) fn build_governed_skill_activation_usage_commands(
    snapshot: &SkillSnapshot,
    governance_records: &[SkillGovernanceRecord],
    app_id: ApplicationId,
    session_id: &str,
    agent_name: &str,
    trace_id: &str,
) -> Vec<SkillGovernanceRecordUsageCommand> {

    let visible_skill_names = snapshot
        .skills
        .iter()
        .map(|skill| skill.name.as_str())
        .collect::<HashSet<_>>();
    let Ok(scope) = SkillServiceScope::agent(app_id, session_id, agent_name) else {
        return Vec::new();
    };

    governance_records
        .iter()
        .filter(|record| record.lifecycle == SkillLifecycleState::Active)
        .filter(|record| visible_skill_names.contains(record.provenance.name.as_str()))
        .map(|record| {
            let mut metadata = BTreeMap::new();
            metadata.insert("activation_surface".into(), "agent_skill_snapshot".into());
            metadata.insert("agent_name".into(), agent_name.into());
            metadata.insert("session_id".into(), session_id.into());
            metadata.insert("snapshot_version".into(), snapshot.version.to_string());
            SkillGovernanceRecordUsageCommand {
                trace: TraceContext::new(trace_id),
                scope: scope.clone(),
                observation: SkillUsageObservation {
                    skill_id: record.provenance.skill_id.clone(),
                    name: record.provenance.name.clone(),
                    source: record.provenance.source.clone(),
                    source_scope: record.provenance.source_scope.clone(),
                    event: SkillUsageEventKind::Activated,
                    author_kind: record.provenance.author_kind.clone(),
                    created_by: record.provenance.created_by.clone(),
                    pinned: Some(record.pinned),
                    evidence_id: Some(format!(
                        "eventlog://sessions/{session_id}/skill_snapshot/{agent_name}"
                    )),
                    metadata,
                },
            }
        })
        .collect()

}

/// Record governed activation telemetry after a snapshot is loaded or built.
pub(super) async fn record_governed_skill_snapshot_activation(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    agent_name: &str,
    session_id: &str,
    snapshot: &SkillSnapshot,
    trace_id: &str,
) {

    let scope = match SkillServiceScope::agent(*app_id, session_id, agent_name) {
        Ok(scope) => scope,
        Err(error) => {
            tracing::warn!(
                error = %error,
                app_id = %app_id,
                agent = %agent_name,
                session_id,
                "skipping governed Skill activation telemetry because scope is invalid"
            );
            return;
        }
    };
    let governance = match state
        .skill_client
        .governance_snapshot(SkillGovernanceSnapshotCommand {
            trace: TraceContext::new(trace_id),
            scope: scope.clone(),
            include_archived: false,
            lifecycle_filters: vec![SkillLifecycleState::Active],
        })
        .await
    {
        Ok(snapshot) => snapshot,
        Err(error) => {
            tracing::warn!(
                error = %error,
                app_id = %app_id,
                agent = %agent_name,
                session_id,
                "governed Skill activation telemetry snapshot lookup failed"
            );
            return;
        }
    };

    for command in build_governed_skill_activation_usage_commands(
        snapshot,
        &governance.records,
        *app_id,
        session_id,
        agent_name,
        trace_id,
    ) {
        let skill_id = command.observation.skill_id.clone();
        if let Err(error) = state.skill_client.record_governance_usage(command).await {
            tracing::warn!(
                error = %error,
                app_id = %app_id,
                agent = %agent_name,
                session_id,
                skill_id,
                "governed Skill activation telemetry recording failed"
            );
        }
    }

}
