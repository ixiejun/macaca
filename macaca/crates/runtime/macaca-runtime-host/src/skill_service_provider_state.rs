//! Mutable state helpers for the built-in Skill system service provider.
//!
//! The public provider remains a thin service adapter: it decodes commands,
//! emits trace-friendly logs, and delegates state transitions here.  Keeping
//! this state module focused prevents the provider adapter from becoming a
//! hidden curation engine while still allowing future Store/EventLog-backed
//! providers to replace the in-memory implementation.

use std::collections::BTreeMap;

use macaca_skill::{
    SkillAliasRecord, SkillAliasResolveCommand, SkillAliasResolveResult, SkillAliasSnapshotResult,
    SkillAliasUpsertResult, SkillCurationDryRunCommand, SkillCurationDryRunResult,
    SkillCurationLifecycleAction, SkillCurationLifecycleCommand, SkillCurationLifecycleResult,
    SkillExperienceProposalCommand, SkillExperienceProposalRecord, SkillExperienceProposalResult,
    SkillExperienceProposalSnapshotResult, SkillGovernanceRecord, SkillGovernanceRecordUsageResult,
    SkillGovernanceSnapshotResult, SkillLifecycleState, SkillUsageObservation,
};
use tokio::sync::Mutex;

/// In-memory governance state for the built-in provider.
///
/// This type is intentionally small and deterministic.  It is not the final
/// persistence layer; it is the local provider's Strategy implementation for
/// tests, development hosts, and future migration to a durable governance store.
#[derive(Default)]
pub(crate) struct SkillProviderGovernanceState {
    records: Mutex<BTreeMap<String, SkillGovernanceRecord>>,
    aliases: Mutex<BTreeMap<String, SkillAliasRecord>>,
    proposals: Mutex<BTreeMap<String, SkillExperienceProposalRecord>>,
}

impl SkillProviderGovernanceState {
    /// Record one sanitized usage observation and return the updated record.
    pub(crate) async fn record_usage(
        &self,
        observation: SkillUsageObservation,
    ) -> SkillGovernanceRecordUsageResult {
        let observed_at = chrono::Utc::now();
        let key = observation.key();
        let mut records = self.records.lock().await;
        let record = records
            .entry(key)
            .and_modify(|record| record.apply(&observation, observed_at))
            .or_insert_with(|| SkillGovernanceRecord::from_observation(&observation, observed_at))
            .clone();

        SkillGovernanceRecordUsageResult {
            record,
            captured_at: observed_at,
        }
    }

    /// Return a sorted, sanitized governance snapshot.
    pub(crate) async fn governance_snapshot(
        &self,
        include_archived: bool,
    ) -> SkillGovernanceSnapshotResult {
        let mut records: Vec<_> = self
            .records
            .lock()
            .await
            .values()
            .filter(|record| include_archived || record.lifecycle != SkillLifecycleState::Archived)
            .cloned()
            .collect();
        records.sort_by(|left, right| left.provenance.skill_id.cmp(&right.provenance.skill_id));

        SkillGovernanceSnapshotResult {
            records,
            captured_at: chrono::Utc::now(),
        }
    }

    /// Build a deterministic curation report without mutating state.
    pub(crate) async fn curation_dry_run(
        &self,
        command: &SkillCurationDryRunCommand,
    ) -> SkillCurationDryRunResult {
        let records = self
            .records
            .lock()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        SkillCurationDryRunResult::from_records(records, command, chrono::Utc::now())
    }

    /// Apply one metadata-only lifecycle transition to a governance record.
    ///
    /// This helper is deliberately limited to governance state.  It does not
    /// touch skill package files, aliases, executable scripts, or scheduler
    /// references; future durable providers can wrap the same transition in
    /// policy decisions, mementos, and audit event persistence.
    pub(crate) async fn apply_lifecycle(
        &self,
        command: SkillCurationLifecycleCommand,
        action: SkillCurationLifecycleAction,
    ) -> Result<SkillCurationLifecycleResult, String> {
        let captured_at = chrono::Utc::now();
        let key = command.key();
        let sanitized_evidence = command
            .evidence_ids
            .iter()
            .filter(|id| !id.trim().is_empty())
            .cloned()
            .collect::<Vec<_>>();
        let mut records = self.records.lock().await;
        let record = records.entry(key).or_insert_with(|| {
            SkillGovernanceRecord::from_lifecycle_command(&command, captured_at)
        });

        if action == SkillCurationLifecycleAction::Archive && record.pinned {
            return Err("pinned skill cannot be archived without an approval override".into());
        }

        match action {
            SkillCurationLifecycleAction::Pin => {
                record.pinned = true;
            }
            SkillCurationLifecycleAction::Unpin => {
                record.pinned = false;
            }
            SkillCurationLifecycleAction::Archive => {
                record.lifecycle = SkillLifecycleState::Archived;
            }
            SkillCurationLifecycleAction::Restore => {
                record.lifecycle = SkillLifecycleState::Active;
            }
        }
        record.updated_at = captured_at;
        for evidence_id in &sanitized_evidence {
            if !record.evidence_ids.contains(evidence_id) {
                record.evidence_ids.push(evidence_id.clone());
            }
        }

        Ok(SkillCurationLifecycleResult {
            skill_id: record.provenance.skill_id.clone(),
            name: record.provenance.name.clone(),
            action,
            lifecycle: record.lifecycle.clone(),
            pinned: record.pinned,
            mutated: true,
            reason: command.reason,
            evidence_ids: sanitized_evidence,
            trace_id: command.trace.trace_id,
            captured_at,
        })
    }

    /// Insert or replace one alias record.
    pub(crate) async fn upsert_alias(
        &self,
        mut record: SkillAliasRecord,
    ) -> SkillAliasUpsertResult {
        let captured_at = chrono::Utc::now();
        if record.created_at > captured_at {
            record.created_at = captured_at;
        }
        record.updated_at = captured_at;
        let mut aliases = self.aliases.lock().await;
        aliases.insert(record.key(), record.clone());

        SkillAliasUpsertResult {
            record,
            captured_at,
        }
    }

    /// Resolve a skill id/name through the alias map.
    pub(crate) async fn resolve_alias(
        &self,
        command: &SkillAliasResolveCommand,
    ) -> SkillAliasResolveResult {
        let captured_at = chrono::Utc::now();
        let aliases = self.aliases.lock().await;
        aliases
            .get(&command.key())
            .cloned()
            .map(|record| SkillAliasResolveResult::resolved(command, record, captured_at))
            .unwrap_or_else(|| SkillAliasResolveResult::unresolved(command, captured_at))
    }

    /// Return a deterministic alias snapshot for diagnostics and audit replay.
    pub(crate) async fn alias_snapshot(&self) -> SkillAliasSnapshotResult {
        let mut aliases = self
            .aliases
            .lock()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        aliases.sort_by(|left, right| left.source_skill_id.cmp(&right.source_skill_id));

        SkillAliasSnapshotResult {
            aliases,
            captured_at: chrono::Utc::now(),
        }
    }

    /// Store a draft-only experience proposal without changing active skills.
    pub(crate) async fn propose_experience(
        &self,
        command: SkillExperienceProposalCommand,
    ) -> SkillExperienceProposalResult {
        let captured_at = chrono::Utc::now();
        let proposal = SkillExperienceProposalRecord::from_candidate(
            &command.trace,
            command.candidate,
            captured_at,
        );
        self.proposals
            .lock()
            .await
            .insert(proposal.proposal_id.clone(), proposal.clone());

        SkillExperienceProposalResult {
            proposal,
            mutated: false,
            captured_at,
        }
    }

    /// Return a deterministic, read-only snapshot of stored draft proposals.
    pub(crate) async fn experience_snapshot(&self) -> SkillExperienceProposalSnapshotResult {
        let captured_at = chrono::Utc::now();
        let mut proposals = self
            .proposals
            .lock()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        proposals.sort_by(|left, right| left.proposal_id.cmp(&right.proposal_id));

        SkillExperienceProposalSnapshotResult {
            proposals,
            mutated: false,
            captured_at,
        }
    }
}
