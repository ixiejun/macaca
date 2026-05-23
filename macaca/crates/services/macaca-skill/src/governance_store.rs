//! Durable Skill governance event and read-model contracts.
//!
//! The Skill service owns governance semantics, but the durable bytes may live
//! in different Store/EventLog providers.  This module therefore defines
//! provider-neutral event DTOs and a replayable read model.  Built-in local
//! providers can keep an in-memory event vector for development, while future
//! Store-backed providers can persist the same records without changing SDK or
//! shell contracts.

use std::collections::BTreeMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use macaca_proto::TraceContext;
use serde::{Deserialize, Serialize};

use crate::alias::SkillAliasRecord;
use crate::evolution::SkillExperienceProposalRecord;
use crate::governance::{SkillGovernanceRecord, SkillUsageObservation, SkillUsageTelemetry};
use crate::provenance::{SkillProvenance, SkillProvenanceEventRecord};
use crate::service_contract::SkillServiceScope;

/// Durable governance read-record stored behind the Skill service boundary.
///
/// The record combines lifecycle metadata, pinning, ownership/trust hints,
/// provenance, telemetry, and audit refs.  Keeping this as a provider-neutral
/// DTO lets the built-in local Strategy and a future Store/EventLog Strategy
/// expose the same command surface without moving curation semantics into
/// kernel, SDK, Web, CLI, or frontend layers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableSkillGovernanceRecord {
    pub skill_id: String,
    pub lifecycle: crate::governance::SkillLifecycleState,
    pub pinned: bool,
    pub source_scope: String,
    pub ownership: String,
    pub trust_level: Option<String>,
    pub evidence_refs: Vec<String>,
    pub policy_decision_refs: Vec<String>,
    pub audit_event_ids: Vec<String>,
    pub provenance: SkillProvenance,
    pub telemetry: SkillUsageTelemetry,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Resolution behavior for a service-owned skill alias.
///
/// Consumers ask the Skill service to resolve aliases instead of rewriting
/// historical task, scheduler, or context references.  The policy keeps the
/// decision explicit and auditable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillAliasResolutionPolicy {
    Redirect,
    WarnAndRedirect,
    Deny,
}

/// Durable alias-map record from one governed skill identity to another.
///
/// Alias records are metadata mementos for curation and merge flows.  They
/// carry a validity window and run reference so future rollbacks can replay or
/// retire the mapping without touching active skill package files directly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillAliasMap {
    pub source_skill_id: String,
    pub target_skill_id: String,
    pub reason: String,
    pub run_id: Option<String>,
    pub valid_from: DateTime<Utc>,
    pub valid_until: Option<DateTime<Utc>>,
    pub resolution_policy: SkillAliasResolutionPolicy,
}

/// Structured failure class returned by governance store strategies.
///
/// The enum is intentionally small for the first Store/EventLog slice.  Future
/// strategies can add denied, conflict, or corruption classes without forcing
/// callers to parse free-form strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillGovernanceStoreUnavailableKind {
    Unavailable,
}

/// Structured unavailable state for governance store operations.
///
/// This is returned instead of panicking or pretending that an append/replay
/// succeeded.  The timestamp and retry hint make operator diagnostics
/// auditable without exposing raw event payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillGovernanceStoreUnavailable {
    pub kind: SkillGovernanceStoreUnavailableKind,
    pub reason: String,
    pub retryable_without_reconfiguration: bool,
    pub captured_at: DateTime<Utc>,
}

impl SkillGovernanceStoreUnavailable {
    /// Build a non-retryable unavailable state for missing Store/EventLog wiring.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            kind: SkillGovernanceStoreUnavailableKind::Unavailable,
            reason: reason.into(),
            retryable_without_reconfiguration: false,
            captured_at: Utc::now(),
        }
    }
}

/// Strategy interface for append-only Skill governance persistence.
///
/// Runtime-host can choose a local, Store/EventLog-backed, plugin, remote, or
/// unavailable strategy at composition time.  The trait keeps persistence
/// decisions behind the Skill service boundary and gives replay consumers a
/// single provider-neutral contract.
#[async_trait]
pub trait SkillGovernanceStoreStrategy: Send + Sync {
    async fn append_event(
        &self,
        event: SkillGovernanceEventRecord,
    ) -> Result<SkillGovernanceEventRecord, SkillGovernanceStoreUnavailable>;

    async fn read_model(&self)
        -> Result<SkillGovernanceReadModel, SkillGovernanceStoreUnavailable>;
}

/// Null Object governance store for hosts without durable Store/EventLog wiring.
///
/// The strategy never mutates state and never returns fake success.  It emits
/// sanitized logs for each denied append or replay so operators can see the
/// exact service boundary where persistence is absent.
#[derive(Debug, Clone)]
pub struct UnavailableSkillGovernanceStore {
    reason: String,
}

impl UnavailableSkillGovernanceStore {
    /// Create an unavailable strategy with a bounded operator-facing reason.
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }

    fn unavailable(&self) -> SkillGovernanceStoreUnavailable {
        SkillGovernanceStoreUnavailable::unavailable(self.reason.clone())
    }
}

#[async_trait]
impl SkillGovernanceStoreStrategy for UnavailableSkillGovernanceStore {
    async fn append_event(
        &self,
        event: SkillGovernanceEventRecord,
    ) -> Result<SkillGovernanceEventRecord, SkillGovernanceStoreUnavailable> {
        tracing::warn!(
            event_id = %event.event_id,
            trace_id = %event.trace_id,
            reason = %self.reason,
            "skill governance store append unavailable"
        );
        Err(self.unavailable())
    }

    async fn read_model(
        &self,
    ) -> Result<SkillGovernanceReadModel, SkillGovernanceStoreUnavailable> {
        tracing::warn!(
            reason = %self.reason,
            "skill governance store replay unavailable"
        );
        Err(self.unavailable())
    }
}

/// Durable metadata for one bounded curation run.
///
/// The record stores only identifiers, counters, refs, and policy/audit ids.
/// Reports and snapshots remain separate Store artifacts so this DTO never
/// carries raw provider output, prompts, skill bodies, or package bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillCurationRunRecord {
    pub run_id: String,
    pub trace_id: String,
    pub provider_id: String,
    pub dry_run: bool,
    pub candidate_count: u64,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub report_ref: Option<String>,
    pub rollback_ref: Option<String>,
    pub policy_decision_ids: Vec<String>,
    pub audit_event_ids: Vec<String>,
}

/// Durable reference to a governance snapshot artifact.
///
/// Snapshot contents are intentionally referenced, not embedded.  That keeps
/// replay DTOs bounded and allows Store/EventLog providers to enforce their own
/// retention, hashing, and access-control policies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillGovernanceSnapshotRefRecord {
    pub snapshot_ref: String,
    pub trace_id: String,
    pub record_count: u64,
    pub captured_at: DateTime<Utc>,
    pub report_ref: Option<String>,
}

/// Durable rollback memento reference for curation or mutation apply flows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillRollbackRefRecord {
    pub rollback_ref: String,
    pub run_id: String,
    pub trace_id: String,
    pub before_snapshot_ref: String,
    pub after_snapshot_ref: Option<String>,
    pub report_ref: Option<String>,
    pub captured_at: DateTime<Utc>,
}

/// Append-only governance event payload.
///
/// Each variant is a bounded, sanitized fact that can be replayed into the
/// Skill governance read model.  Side-effecting providers must append these
/// records after policy/trace validation so audit tools can reconstruct how
/// lifecycle, telemetry, aliases, proposals, and mementos changed over time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillGovernanceEventPayload {
    UsageRecorded(SkillUsageObservation),
    LifecycleApplied(SkillUsageObservation),
    AliasUpserted(SkillAliasRecord),
    ProposalCreated(SkillExperienceProposalRecord),
    CurationRunRecorded(SkillCurationRunRecord),
    SnapshotRefRecorded(SkillGovernanceSnapshotRefRecord),
    RollbackRefRecorded(SkillRollbackRefRecord),
}

/// One append-only Skill governance event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillGovernanceEventRecord {
    pub event_id: String,
    pub trace_id: String,
    pub scope: SkillServiceScope,
    pub occurred_at: DateTime<Utc>,
    pub policy_decision_ids: Vec<String>,
    pub audit_event_ids: Vec<String>,
    pub provenance: SkillProvenanceEventRecord,
    pub payload: SkillGovernanceEventPayload,
}

impl SkillGovernanceEventRecord {
    /// Build a sanitized event from a validated service command boundary.
    pub fn new(
        event_id: impl Into<String>,
        trace: &TraceContext,
        scope: SkillServiceScope,
        occurred_at: DateTime<Utc>,
        payload: SkillGovernanceEventPayload,
    ) -> Self {
        let trace_id = trace.trace_id.clone();
        let provenance =
            SkillProvenanceEventRecord::from_payload(&trace_id, &scope, occurred_at, &payload);
        Self {
            event_id: event_id.into(),
            trace_id,
            scope,
            occurred_at,
            policy_decision_ids: Vec::new(),
            audit_event_ids: Vec::new(),
            provenance,
            payload,
        }
    }
}

/// Replayable, prompt-safe Skill governance read model.
///
/// This read model mirrors the service snapshots used by SDK and shell
/// adapters, but it is built exclusively from event records.  The replay logic
/// is deterministic and omits full `SKILL.md` bodies by construction because no
/// event variant accepts instruction content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillGovernanceReadModel {
    pub records: Vec<SkillGovernanceRecord>,
    pub provenance_events: Vec<SkillProvenanceEventRecord>,
    pub aliases: Vec<SkillAliasRecord>,
    pub proposals: Vec<SkillExperienceProposalRecord>,
    pub curation_runs: Vec<SkillCurationRunRecord>,
    pub snapshot_refs: Vec<SkillGovernanceSnapshotRefRecord>,
    pub rollback_refs: Vec<SkillRollbackRefRecord>,
    pub replayed_events: usize,
    pub captured_at: DateTime<Utc>,
}

impl SkillGovernanceReadModel {
    /// Rebuild the read model by replaying append-only governance events.
    pub fn from_events(events: impl IntoIterator<Item = SkillGovernanceEventRecord>) -> Self {
        let mut records = BTreeMap::<String, SkillGovernanceRecord>::new();
        let mut provenance_events = Vec::<SkillProvenanceEventRecord>::new();
        let mut aliases = BTreeMap::<String, SkillAliasRecord>::new();
        let mut proposals = BTreeMap::<String, SkillExperienceProposalRecord>::new();
        let mut curation_runs = BTreeMap::<String, SkillCurationRunRecord>::new();
        let mut snapshot_refs = BTreeMap::<String, SkillGovernanceSnapshotRefRecord>::new();
        let mut rollback_refs = BTreeMap::<String, SkillRollbackRefRecord>::new();
        let mut replayed_events = 0usize;

        for event in events {
            replayed_events = replayed_events.saturating_add(1);
            provenance_events.push(event.provenance.clone());
            match event.payload {
                SkillGovernanceEventPayload::UsageRecorded(observation)
                | SkillGovernanceEventPayload::LifecycleApplied(observation) => {
                    let key = observation.key();
                    records
                        .entry(key)
                        .and_modify(|record| record.apply(&observation, event.occurred_at))
                        .or_insert_with(|| {
                            SkillGovernanceRecord::from_observation(&observation, event.occurred_at)
                        });
                }
                SkillGovernanceEventPayload::AliasUpserted(record) => {
                    aliases.insert(record.key(), record);
                }
                SkillGovernanceEventPayload::ProposalCreated(record) => {
                    proposals.insert(record.proposal_id.clone(), record);
                }
                SkillGovernanceEventPayload::CurationRunRecorded(record) => {
                    curation_runs.insert(record.run_id.clone(), record);
                }
                SkillGovernanceEventPayload::SnapshotRefRecorded(record) => {
                    snapshot_refs.insert(record.snapshot_ref.clone(), record);
                }
                SkillGovernanceEventPayload::RollbackRefRecorded(record) => {
                    rollback_refs.insert(record.rollback_ref.clone(), record);
                }
            }
        }

        Self {
            records: sorted_values(records),
            provenance_events,
            aliases: sorted_values(aliases),
            proposals: sorted_values(proposals),
            curation_runs: sorted_values(curation_runs),
            snapshot_refs: sorted_values(snapshot_refs),
            rollback_refs: sorted_values(rollback_refs),
            replayed_events,
            captured_at: Utc::now(),
        }
    }
}

fn sorted_values<T>(map: BTreeMap<String, T>) -> Vec<T> {
    map.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::governance::{SkillAuthorKind, SkillLifecycleState};

    #[test]
    fn durable_governance_dtos_capture_store_backed_skill_state_without_bodies() {
        let now = Utc::now();
        let provenance = SkillProvenance {
            skill_id: "skill://agent/refactor".into(),
            version: Some("1.0.0".into()),
            author_kind: SkillAuthorKind::Agent,
            author_agent_id: Some("agent-maintainer".into()),
            application_id: Some("app-alpha".into()),
            session_id: Some("session-1".into()),
            task_id: Some("task-1".into()),
            trace_id: "trace-1".into(),
            evidence_refs: vec!["store://evidence/task-1".into()],
            source_scope: "workspace".into(),
            trust_level: Some("verified".into()),
            created_at: now,
            updated_at: now,
        };
        let telemetry = SkillUsageTelemetry {
            view_count: 1,
            activation_count: 2,
            resource_read_count: 3,
            use_count: 2,
            patch_count: 4,
            successful_task_count: 5,
            failed_task_count: 6,
            last_viewed_at: Some(now),
            last_resource_read_at: Some(now),
            last_used_at: Some(now),
            last_patched_at: Some(now),
            last_successful_task_at: Some(now),
            last_failed_task_at: Some(now),
            last_lifecycle_event_at: Some(now),
            last_observed_at: Some(now),
        };
        let record = DurableSkillGovernanceRecord {
            skill_id: provenance.skill_id.clone(),
            lifecycle: SkillLifecycleState::Active,
            pinned: false,
            source_scope: provenance.source_scope.clone(),
            ownership: "agent-private".into(),
            trust_level: provenance.trust_level.clone(),
            evidence_refs: provenance.evidence_refs.clone(),
            policy_decision_refs: vec!["policy://decision/1".into()],
            audit_event_ids: vec!["audit://event/1".into()],
            provenance,
            telemetry,
            created_at: now,
            updated_at: now,
        };
        let alias = SkillAliasMap {
            source_skill_id: "skill://agent/old".into(),
            target_skill_id: "skill://agent/refactor".into(),
            reason: "superseded by a broader maintained skill".into(),
            run_id: Some("curation-run-1".into()),
            valid_from: now,
            valid_until: Some(now),
            resolution_policy: SkillAliasResolutionPolicy::WarnAndRedirect,
        };

        let serialized = serde_json::to_string(&(record, alias))
            .expect("durable governance DTOs should serialize");

        assert!(serialized.contains("activation_count"));
        assert!(serialized.contains("WarnAndRedirect"));
        assert!(!serialized.contains("SKILL.md body"));
    }

    #[tokio::test]
    async fn unavailable_governance_store_strategy_returns_structured_unavailable() {
        let store = UnavailableSkillGovernanceStore::new("Store/EventLog provider is absent");
        let trace = TraceContext::new("trace-governance-store-unavailable");
        let event = SkillGovernanceEventRecord::new(
            "event-unavailable-1",
            &trace,
            SkillServiceScope::default(),
            Utc::now(),
            SkillGovernanceEventPayload::UsageRecorded(SkillUsageObservation {
                skill_id: "skill://agent/missing-store".into(),
                name: "missing-store".into(),
                source: "test".into(),
                source_scope: "workspace".into(),
                event: crate::governance::SkillUsageEventKind::Viewed,
                author_kind: SkillAuthorKind::Agent,
                created_by: None,
                pinned: None,
                evidence_id: Some("evidence-1".into()),
                metadata: BTreeMap::new(),
            }),
        );

        let append_err = store
            .append_event(event)
            .await
            .expect_err("unavailable store must not fake append success");
        let replay_err = store
            .read_model()
            .await
            .expect_err("unavailable store must not fake replay success");

        assert_eq!(
            append_err.kind,
            SkillGovernanceStoreUnavailableKind::Unavailable
        );
        assert_eq!(
            replay_err.kind,
            SkillGovernanceStoreUnavailableKind::Unavailable
        );
        assert!(!append_err.retryable_without_reconfiguration);
        assert!(append_err
            .reason
            .contains("Store/EventLog provider is absent"));
    }
}
