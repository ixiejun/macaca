//! Sanitized Skill provenance event contracts.
//!
//! Provenance is the audit-facing trail for how a governed skill changed.  The
//! records in this module deliberately store only identifiers, scope hints, and
//! evidence references.  They never copy prompt text, provider payloads,
//! package bytes, manifests, or full skill bodies, which keeps Store/EventLog
//! replay useful without turning audit history into a sensitive-content sink.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::alias::SkillAliasRecord;
use crate::evolution::SkillExperienceProposalRecord;
use crate::governance::{SkillAuthorKind, SkillUsageEventKind, SkillUsageObservation};
use crate::governance_store::{
    SkillCurationRunRecord, SkillGovernanceEventPayload, SkillGovernanceSnapshotRefRecord,
    SkillRollbackRefRecord,
};
use crate::service_contract::SkillServiceScope;

/// Durable provenance for a governed skill package or draft.
///
/// This DTO answers who created or changed a skill, which task/session/app
/// supplied evidence, and which trace can replay the decision.  It stores refs
/// only; raw task output, package content, and skill instructions stay outside
/// the governance event log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillProvenance {
    pub skill_id: String,
    pub version: Option<String>,
    pub author_kind: SkillAuthorKind,
    pub author_agent_id: Option<String>,
    pub application_id: Option<String>,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub trace_id: String,
    pub evidence_refs: Vec<String>,
    pub source_scope: String,
    pub trust_level: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// The generic provenance action captured for one governance event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillProvenanceAction {
    Discovered,
    Viewed,
    Activated,
    ResourceRead,
    SuccessfulTask,
    FailedTask,
    Patched,
    ProposalCreated,
    AliasUpdated,
    CurationRecorded,
    SnapshotRecorded,
    RollbackRecorded,
    Archived,
    Restored,
    Quarantined,
    QuarantineReleased,
    Superseded,
    Rejected,
    Pinned,
    Unpinned,
}

/// One prompt-safe provenance event derived from a governance event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillProvenanceEventRecord {
    pub provenance: SkillProvenance,
    pub action: SkillProvenanceAction,
    pub action_ref: Option<String>,
    pub target_skill_id: Option<String>,
    pub policy_decision_refs: Vec<String>,
    pub audit_event_ids: Vec<String>,
    pub captured_at: DateTime<Utc>,
}

impl SkillProvenanceEventRecord {
    /// Derive a provenance event from the already-sanitized governance payload.
    pub fn from_payload(
        trace_id: &str,
        scope: &SkillServiceScope,
        occurred_at: DateTime<Utc>,
        payload: &SkillGovernanceEventPayload,
    ) -> Self {
        match payload {
            SkillGovernanceEventPayload::UsageRecorded(observation)
            | SkillGovernanceEventPayload::LifecycleApplied(observation) => {
                Self::from_observation(trace_id, scope, occurred_at, observation)
            }
            SkillGovernanceEventPayload::AliasUpserted(record) => {
                Self::from_alias(trace_id, scope, occurred_at, record)
            }
            SkillGovernanceEventPayload::ProposalCreated(record) => {
                Self::from_proposal(trace_id, scope, occurred_at, record)
            }
            SkillGovernanceEventPayload::CurationRunRecorded(record) => {
                Self::from_curation_run(scope, occurred_at, record)
            }
            SkillGovernanceEventPayload::SnapshotRefRecorded(record) => {
                Self::from_snapshot_ref(scope, occurred_at, record)
            }
            SkillGovernanceEventPayload::RollbackRefRecorded(record) => {
                Self::from_rollback_ref(scope, occurred_at, record)
            }
        }
    }

    fn from_observation(
        trace_id: &str,
        scope: &SkillServiceScope,
        occurred_at: DateTime<Utc>,
        observation: &SkillUsageObservation,
    ) -> Self {
        let action = action_from_usage(&observation.event);
        Self {
            provenance: base_provenance(
                observation.key(),
                observation.author_kind.clone(),
                observation
                    .created_by
                    .clone()
                    .or_else(|| scope.agent_name.clone()),
                scope,
                observation.metadata.get("task_id").cloned(),
                trace_id,
                observation.evidence_id.iter().cloned().collect(),
                observation.source_scope.clone(),
                occurred_at,
            ),
            action,
            action_ref: observation.metadata.get("action_ref").cloned(),
            target_skill_id: observation.metadata.get("target_skill_id").cloned(),
            policy_decision_refs: Vec::new(),
            audit_event_ids: Vec::new(),
            captured_at: occurred_at,
        }
    }

    fn from_alias(
        trace_id: &str,
        scope: &SkillServiceScope,
        occurred_at: DateTime<Utc>,
        record: &SkillAliasRecord,
    ) -> Self {
        Self {
            provenance: base_provenance(
                record.source_skill_id.clone(),
                SkillAuthorKind::Agent,
                scope.agent_name.clone(),
                scope,
                None,
                trace_id,
                record.evidence_ids.clone(),
                "alias".into(),
                occurred_at,
            ),
            action: SkillProvenanceAction::AliasUpdated,
            action_ref: Some(record.key()),
            target_skill_id: Some(record.target_skill_id.clone()),
            policy_decision_refs: Vec::new(),
            audit_event_ids: Vec::new(),
            captured_at: occurred_at,
        }
    }

    fn from_proposal(
        trace_id: &str,
        scope: &SkillServiceScope,
        occurred_at: DateTime<Utc>,
        record: &SkillExperienceProposalRecord,
    ) -> Self {
        let skill_id = record
            .target_skill_id
            .clone()
            .or_else(|| record.target_skill_name.clone())
            .unwrap_or_else(|| record.proposal_id.clone());
        Self {
            provenance: base_provenance(
                skill_id,
                SkillAuthorKind::Agent,
                record
                    .agent_name
                    .clone()
                    .or_else(|| scope.agent_name.clone()),
                scope,
                Some(record.task_id.clone()),
                trace_id,
                record.evidence_ids.clone(),
                "proposal".into(),
                occurred_at,
            ),
            action: SkillProvenanceAction::ProposalCreated,
            action_ref: Some(record.proposal_id.clone()),
            target_skill_id: record.target_skill_id.clone(),
            policy_decision_refs: Vec::new(),
            audit_event_ids: Vec::new(),
            captured_at: occurred_at,
        }
    }

    fn from_curation_run(
        scope: &SkillServiceScope,
        occurred_at: DateTime<Utc>,
        record: &SkillCurationRunRecord,
    ) -> Self {
        Self {
            provenance: base_provenance(
                record.run_id.clone(),
                SkillAuthorKind::System,
                scope.agent_name.clone(),
                scope,
                None,
                &record.trace_id,
                Vec::new(),
                "curation".into(),
                occurred_at,
            ),
            action: SkillProvenanceAction::CurationRecorded,
            action_ref: Some(record.run_id.clone()),
            target_skill_id: None,
            policy_decision_refs: record.policy_decision_ids.clone(),
            audit_event_ids: record.audit_event_ids.clone(),
            captured_at: occurred_at,
        }
    }

    fn from_snapshot_ref(
        scope: &SkillServiceScope,
        occurred_at: DateTime<Utc>,
        record: &SkillGovernanceSnapshotRefRecord,
    ) -> Self {
        Self {
            provenance: base_provenance(
                record.snapshot_ref.clone(),
                SkillAuthorKind::System,
                scope.agent_name.clone(),
                scope,
                None,
                &record.trace_id,
                Vec::new(),
                "snapshot".into(),
                occurred_at,
            ),
            action: SkillProvenanceAction::SnapshotRecorded,
            action_ref: Some(record.snapshot_ref.clone()),
            target_skill_id: None,
            policy_decision_refs: Vec::new(),
            audit_event_ids: Vec::new(),
            captured_at: occurred_at,
        }
    }

    fn from_rollback_ref(
        scope: &SkillServiceScope,
        occurred_at: DateTime<Utc>,
        record: &SkillRollbackRefRecord,
    ) -> Self {
        Self {
            provenance: base_provenance(
                record.rollback_ref.clone(),
                SkillAuthorKind::System,
                scope.agent_name.clone(),
                scope,
                None,
                &record.trace_id,
                Vec::new(),
                "rollback".into(),
                occurred_at,
            ),
            action: SkillProvenanceAction::RollbackRecorded,
            action_ref: Some(record.rollback_ref.clone()),
            target_skill_id: None,
            policy_decision_refs: Vec::new(),
            audit_event_ids: Vec::new(),
            captured_at: occurred_at,
        }
    }
}

fn base_provenance(
    skill_id: String,
    author_kind: SkillAuthorKind,
    author_agent_id: Option<String>,
    scope: &SkillServiceScope,
    task_id: Option<String>,
    trace_id: &str,
    evidence_refs: Vec<String>,
    source_scope: String,
    occurred_at: DateTime<Utc>,
) -> SkillProvenance {
    SkillProvenance {
        skill_id,
        version: None,
        author_kind,
        author_agent_id,
        application_id: scope.application_id.as_ref().map(ToString::to_string),
        session_id: scope.session_id.clone(),
        task_id: task_id.filter(|value| !value.trim().is_empty()),
        trace_id: trace_id.to_string(),
        evidence_refs: evidence_refs
            .into_iter()
            .filter(|value| !value.trim().is_empty())
            .collect(),
        source_scope,
        trust_level: None,
        created_at: occurred_at,
        updated_at: occurred_at,
    }
}

fn action_from_usage(event: &SkillUsageEventKind) -> SkillProvenanceAction {
    match event {
        SkillUsageEventKind::Created => SkillProvenanceAction::Discovered,
        SkillUsageEventKind::Viewed => SkillProvenanceAction::Viewed,
        SkillUsageEventKind::Activated | SkillUsageEventKind::Used => {
            SkillProvenanceAction::Activated
        }
        SkillUsageEventKind::ResourceRead => SkillProvenanceAction::ResourceRead,
        SkillUsageEventKind::Patched => SkillProvenanceAction::Patched,
        SkillUsageEventKind::SuccessfulTask => SkillProvenanceAction::SuccessfulTask,
        SkillUsageEventKind::FailedTask => SkillProvenanceAction::FailedTask,
        SkillUsageEventKind::Archived => SkillProvenanceAction::Archived,
        SkillUsageEventKind::Restored => SkillProvenanceAction::Restored,
        SkillUsageEventKind::Quarantined => SkillProvenanceAction::Quarantined,
        SkillUsageEventKind::QuarantineReleased => SkillProvenanceAction::QuarantineReleased,
        SkillUsageEventKind::Superseded => SkillProvenanceAction::Superseded,
        SkillUsageEventKind::Rejected => SkillProvenanceAction::Rejected,
        SkillUsageEventKind::Pinned => SkillProvenanceAction::Pinned,
        SkillUsageEventKind::Unpinned => SkillProvenanceAction::Unpinned,
    }
}
