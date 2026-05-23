//! Durable Skill governance event and read-model contracts.
//!
//! The Skill service owns governance semantics, but the durable bytes may live
//! in different Store/EventLog providers.  This module therefore defines
//! provider-neutral event DTOs and a replayable read model.  Built-in local
//! providers can keep an in-memory event vector for development, while future
//! Store-backed providers can persist the same records without changing SDK or
//! shell contracts.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use macaca_proto::TraceContext;
use serde::{Deserialize, Serialize};

use crate::alias::SkillAliasRecord;
use crate::evolution::SkillExperienceProposalRecord;
use crate::governance::{SkillGovernanceRecord, SkillUsageObservation};
use crate::service_contract::SkillServiceScope;

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
        Self {
            event_id: event_id.into(),
            trace_id: trace.trace_id.clone(),
            scope,
            occurred_at,
            policy_decision_ids: Vec::new(),
            audit_event_ids: Vec::new(),
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
        let mut aliases = BTreeMap::<String, SkillAliasRecord>::new();
        let mut proposals = BTreeMap::<String, SkillExperienceProposalRecord>::new();
        let mut curation_runs = BTreeMap::<String, SkillCurationRunRecord>::new();
        let mut snapshot_refs = BTreeMap::<String, SkillGovernanceSnapshotRefRecord>::new();
        let mut rollback_refs = BTreeMap::<String, SkillRollbackRefRecord>::new();
        let mut replayed_events = 0usize;

        for event in events {
            replayed_events = replayed_events.saturating_add(1);
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
