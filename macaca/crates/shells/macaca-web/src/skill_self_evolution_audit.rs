//! Canonical API-first audit adapter for Skill self-evolution proof.
//!
//! This Web module is intentionally diagnostic-only.  It aggregates already
//! canonical service surfaces: Skill governance operations, Skill runtime
//! registry/load-path snapshots, and bounded EventLog evidence.  It never reads
//! full Skill bodies, task prompts, provider payloads, or package bytes, and it
//! never decides curation or materialization semantics locally.

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use macaca_app::AppLoader;
use macaca_proto::{ApplicationId, TraceContext};
use macaca_skill::{
    SkillGovernanceRecord, SkillGovernanceSnapshotCommand, SkillLifecycleState, SkillPolicy,
    SkillRuntimeFacade, SkillServiceScope, SkillSnapshotRequest,
};
use serde::{Deserialize, Serialize};

use crate::routes::{err, proto_err, ErrorResponse};
use crate::state::AppState;

const MAX_AUDIT_EVENTS: usize = 200;

/// Query parameters for the canonical self-evolution audit route.
#[derive(Debug, Deserialize)]
pub struct SkillSelfEvolutionAuditQuery {
    pub agent: String,
    pub session_id: Option<String>,
    pub target_skill: String,
}

/// Top-level API-first audit result.
#[derive(Debug, Serialize)]
pub struct SkillSelfEvolutionAuditReport {
    pub status: CanonicalEvidenceStatus,
    pub target_skill: String,
    pub operations: CanonicalEvidenceCheck,
    pub registry_load_path: CanonicalEvidenceCheck,
    pub observer_events: CanonicalEvidenceCheck,
    pub missing_evidence: Vec<String>,
    pub trace_id: String,
}

/// Small status enum for machine-readable audit decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalEvidenceStatus {
    Passed,
    Failed,
}

/// One evidence category in the API-first audit response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CanonicalEvidenceCheck {
    pub status: CanonicalEvidenceStatus,
    pub count: usize,
    pub refs: Vec<String>,
    pub reason: Option<String>,
}

impl CanonicalEvidenceCheck {
    fn passed(refs: Vec<String>) -> Self {
        Self {
            status: CanonicalEvidenceStatus::Passed,
            count: refs.len(),
            refs,
            reason: None,
        }
    }

    fn failed(reason: impl Into<String>) -> Self {
        Self {
            status: CanonicalEvidenceStatus::Failed,
            count: 0,
            refs: Vec::new(),
            reason: Some(reason.into()),
        }
    }
}

/// GET /api/apps/{app_id}/skills/self-evolution/audit
/// returns a canonical API-first proof snapshot for one target Skill.
pub async fn get_skill_self_evolution_audit(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Query(query): Query<SkillSelfEvolutionAuditQuery>,
) -> Result<Json<SkillSelfEvolutionAuditReport>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = parse_application_id(&app_id)?;
    let target_skill = bounded_target_skill(&query.target_skill)?;
    let trace = TraceContext::new(format!("web-skill-self-evolution-audit-{}", app_id.0));
    let scope = SkillServiceScope {
        application_id: Some(app_id),
        session_id: query.session_id.clone(),
        tenant_id: None,
        agent_name: Some(query.agent.clone()),
    };

    let governance = state
        .skill_client
        .governance_snapshot(SkillGovernanceSnapshotCommand {
            trace: trace.clone(),
            scope,
            include_archived: false,
            lifecycle_filters: vec![SkillLifecycleState::Active],
        })
        .await
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;
    let operations = operations_check(&governance.records, &target_skill);
    let registry_load_path =
        registry_load_path_check(&state, app_id, &query.agent, &target_skill).await?;
    let observer_events = observer_event_check(&state, query.session_id.as_deref()).await;
    let report = build_audit_report(
        target_skill,
        operations,
        registry_load_path,
        observer_events,
        trace.trace_id,
    );

    tracing::info!(
        app_id = %app_id.0,
        agent = %query.agent,
        target_skill = %report.target_skill,
        status = ?report.status,
        missing = report.missing_evidence.len(),
        operations_refs = report.operations.count,
        registry_refs = report.registry_load_path.count,
        observer_refs = report.observer_events.count,
        "web route emitted canonical API-first Skill self-evolution audit"
    );

    Ok(Json(report))
}

fn operations_check(
    records: &[SkillGovernanceRecord],
    target_skill: &str,
) -> CanonicalEvidenceCheck {
    let refs = records
        .iter()
        .filter(|record| record.lifecycle == SkillLifecycleState::Active)
        .filter(|record| {
            record.provenance.name == target_skill || record.provenance.skill_id == target_skill
        })
        .map(|record| format!("skill-governance://{}", record.provenance.skill_id))
        .collect::<Vec<_>>();
    if refs.is_empty() {
        CanonicalEvidenceCheck::failed("missing_active_operations_governance_record")
    } else {
        CanonicalEvidenceCheck::passed(refs)
    }
}

async fn registry_load_path_check(
    state: &Arc<AppState>,
    app_id: ApplicationId,
    agent_name: &str,
    target_skill: &str,
) -> Result<CanonicalEvidenceCheck, (StatusCode, Json<ErrorResponse>)> {
    let app = {
        let registry = state.registry.read().await;
        registry
            .get_app(&app_id)
            .cloned()
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "App not found".into()))?
    };
    let agent_configs = AppLoader::resolve_agent_configs(&app.manifest, &app.path)
        .map_err(|error| proto_err(StatusCode::INTERNAL_SERVER_ERROR, &error))?;
    let Some(agent_config) = agent_configs
        .into_iter()
        .find(|agent| agent.name == agent_name)
    else {
        return Ok(CanonicalEvidenceCheck::failed(
            "missing_agent_configuration",
        ));
    };
    let workspace_root = {
        let workspaces = state.config.app_workspaces.read().await;
        workspaces.get(&app_id).map(|ws| ws.root.clone())
    };
    let policy = agent_config
        .skills
        .as_ref()
        .map(|skills| SkillPolicy {
            allow: skills.allow.clone(),
            deny: skills.deny.clone(),
        })
        .unwrap_or_default();
    let snapshot = SkillRuntimeFacade::new()
        .build_snapshot(
            SkillSnapshotRequest::builder(agent_config.name)
                .workspace_dir(workspace_root)
                .app_dir(Some(app.path))
                .policy(policy)
                .build(),
        )
        .await
        .map_err(|error| proto_err(StatusCode::INTERNAL_SERVER_ERROR, &error))?;
    let refs = snapshot
        .skills
        .iter()
        .filter(|skill| skill.name == target_skill)
        .map(|skill| format!("skill-registry://{}", skill.location.display()))
        .collect::<Vec<_>>();
    if refs.is_empty() {
        Ok(CanonicalEvidenceCheck::failed(
            "missing_registry_load_path_projection",
        ))
    } else {
        Ok(CanonicalEvidenceCheck::passed(refs))
    }
}

async fn observer_event_check(
    state: &Arc<AppState>,
    session_id: Option<&str>,
) -> CanonicalEvidenceCheck {
    let Some(session_id) = session_id.filter(|id| !id.trim().is_empty()) else {
        return CanonicalEvidenceCheck::failed("missing_session_id_for_observer_evidence");
    };
    let refs = state
        .persist
        .event_log
        .query(session_id, 0, MAX_AUDIT_EVENTS)
        .await
        .into_iter()
        .filter(|event| {
            event.event_type == "skill_self_evolution_observer"
                || event.event_type.starts_with("skill_snapshot")
        })
        .map(|event| format!("eventlog://sessions/{session_id}/events/{:08}", event.seq))
        .collect::<Vec<_>>();
    if refs.is_empty() {
        CanonicalEvidenceCheck::failed("missing_session_observer_or_skill_snapshot_events")
    } else {
        CanonicalEvidenceCheck::passed(refs)
    }
}

fn build_audit_report(
    target_skill: String,
    operations: CanonicalEvidenceCheck,
    registry_load_path: CanonicalEvidenceCheck,
    observer_events: CanonicalEvidenceCheck,
    trace_id: String,
) -> SkillSelfEvolutionAuditReport {
    let mut missing_evidence = Vec::new();
    collect_missing("operations", &operations, &mut missing_evidence);
    collect_missing(
        "registry_load_path",
        &registry_load_path,
        &mut missing_evidence,
    );
    collect_missing("observer_events", &observer_events, &mut missing_evidence);
    let status = if missing_evidence.is_empty() {
        CanonicalEvidenceStatus::Passed
    } else {
        CanonicalEvidenceStatus::Failed
    };

    SkillSelfEvolutionAuditReport {
        status,
        target_skill,
        operations,
        registry_load_path,
        observer_events,
        missing_evidence,
        trace_id,
    }
}

fn collect_missing(
    category: &str,
    check: &CanonicalEvidenceCheck,
    missing_evidence: &mut Vec<String>,
) {
    if check.status == CanonicalEvidenceStatus::Failed {
        missing_evidence.push(format!(
            "{}:{}",
            category,
            check.reason.as_deref().unwrap_or("missing")
        ));
    }
}

fn parse_application_id(app_id: &str) -> Result<ApplicationId, (StatusCode, Json<ErrorResponse>)> {
    app_id
        .parse::<uuid::Uuid>()
        .map(ApplicationId)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))
}

fn bounded_target_skill(target_skill: &str) -> Result<String, (StatusCode, Json<ErrorResponse>)> {
    let target = target_skill.trim();
    if target.is_empty() {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "target_skill is required".into(),
        ));
    }
    Ok(target.chars().take(128).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_first_audit_report_passes_only_when_all_canonical_evidence_exists() {
        let report = build_audit_report(
            "materialization-registry-telemetry".into(),
            CanonicalEvidenceCheck::passed(vec!["skill-governance://skill://agent/a".into()]),
            CanonicalEvidenceCheck::passed(vec!["skill-registry:///workspace/SKILL.md".into()]),
            CanonicalEvidenceCheck::passed(vec!["eventlog://sessions/s/events/00000001".into()]),
            "trace-audit".into(),
        );

        assert_eq!(report.status, CanonicalEvidenceStatus::Passed);
        assert!(report.missing_evidence.is_empty());
    }

    #[test]
    fn api_first_audit_report_fails_without_filesystem_inference() {
        let report = build_audit_report(
            "materialization-registry-telemetry".into(),
            CanonicalEvidenceCheck::failed("missing_active_operations_governance_record"),
            CanonicalEvidenceCheck::passed(vec!["skill-registry:///workspace/SKILL.md".into()]),
            CanonicalEvidenceCheck::failed("missing_session_observer_or_skill_snapshot_events"),
            "trace-audit".into(),
        );

        assert_eq!(report.status, CanonicalEvidenceStatus::Failed);
        assert_eq!(report.missing_evidence.len(), 2);
        assert!(report
            .missing_evidence
            .contains(&"operations:missing_active_operations_governance_record".into()));
        assert!(report
            .missing_evidence
            .contains(&"observer_events:missing_session_observer_or_skill_snapshot_events".into()));
    }
}
