//! Mutable state helpers for the built-in Skill system service provider.
//!
//! The public provider remains a thin service adapter: it decodes commands,
//! emits trace-friendly logs, and delegates state transitions here.  Keeping
//! this state module focused prevents the provider adapter from becoming a
//! hidden curation engine while still allowing future Store/EventLog-backed
//! providers to replace the in-memory implementation.

use std::collections::BTreeMap;

use async_trait::async_trait;
use macaca_skill::{
    SkillAliasRecord, SkillAliasResolveCommand, SkillAliasResolveResult, SkillAliasSnapshotResult,
    SkillAliasUpsertCommand, SkillAliasUpsertResult, SkillCurationDryRunCommand,
    SkillCurationDryRunResult, SkillCurationLifecycleAction, SkillCurationLifecycleCommand,
    SkillCurationLifecycleResult, SkillExperienceProposalCommand, SkillExperienceProposalRecord,
    SkillExperienceProposalResult, SkillExperienceProposalSnapshotResult,
    SkillGovernanceEventPayload, SkillGovernanceEventRecord, SkillGovernanceReadModel,
    SkillGovernanceRecord, SkillGovernanceRecordUsageCommand, SkillGovernanceRecordUsageResult,
    SkillGovernanceSnapshotResult, SkillGovernanceStoreStrategy, SkillGovernanceStoreUnavailable,
    SkillLifecycleState, SkillLifecycleStateMachine, SkillUsageObservation,
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
    event_log: Mutex<Vec<SkillGovernanceEventRecord>>,
}

impl SkillProviderGovernanceState {
    /// Record one sanitized usage observation and return the updated record.
    pub(crate) async fn record_usage(
        &self,
        command: SkillGovernanceRecordUsageCommand,
    ) -> SkillGovernanceRecordUsageResult {
        let observed_at = chrono::Utc::now();
        let observation = command.observation;
        let key = observation.key();
        let mut records = self.records.lock().await;
        let record = records
            .entry(key)
            .and_modify(|record| record.apply(&observation, observed_at))
            .or_insert_with(|| SkillGovernanceRecord::from_observation(&observation, observed_at))
            .clone();
        drop(records);
        self.append_event(SkillGovernanceEventRecord::new(
            event_id("skill-governance-usage", observed_at),
            &command.trace,
            command.scope,
            observed_at,
            SkillGovernanceEventPayload::UsageRecorded(observation),
        ))
        .await;

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
        let all_records = self
            .records
            .lock()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        let archived_records = all_records
            .iter()
            .filter(|record| record.lifecycle == SkillLifecycleState::Archived)
            .count();
        let filtered_out = if include_archived {
            0
        } else {
            archived_records
        };
        let mut records: Vec<_> = all_records
            .into_iter()
            .filter(|record| include_archived || record.lifecycle != SkillLifecycleState::Archived)
            .collect();
        records.sort_by(|left, right| left.provenance.skill_id.cmp(&right.provenance.skill_id));
        tracing::info!(
            include_archived = include_archived,
            records = records.len(),
            filtered_records = filtered_out,
            "skill governance snapshot built through local compatibility adapter"
        );

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

        let target_lifecycle = match &action {
            SkillCurationLifecycleAction::Pin => {
                record.pinned = true;
                record.lifecycle.clone()
            }
            SkillCurationLifecycleAction::Unpin => {
                record.pinned = false;
                record.lifecycle.clone()
            }
            SkillCurationLifecycleAction::Archive => SkillLifecycleState::Archived,
            SkillCurationLifecycleAction::Restore => SkillLifecycleState::Active,
        };
        SkillLifecycleStateMachine::validate_transition(&record.lifecycle, &target_lifecycle)?;
        record.lifecycle = target_lifecycle;
        record.updated_at = captured_at;
        for evidence_id in &sanitized_evidence {
            if !record.evidence_ids.contains(evidence_id) {
                record.evidence_ids.push(evidence_id.clone());
            }
        }
        let result_skill_id = record.provenance.skill_id.clone();
        let result_name = record.provenance.name.clone();
        let result_lifecycle = record.lifecycle.clone();
        let result_pinned = record.pinned;
        drop(records);
        self.append_event(SkillGovernanceEventRecord::new(
            event_id("skill-governance-lifecycle", captured_at),
            &command.trace,
            command.scope.clone(),
            captured_at,
            SkillGovernanceEventPayload::LifecycleApplied(SkillUsageObservation {
                skill_id: command.skill_id.clone(),
                name: command.name.clone(),
                source: command.source.clone(),
                source_scope: command.source_scope.clone(),
                event: match &action {
                    SkillCurationLifecycleAction::Pin => macaca_skill::SkillUsageEventKind::Pinned,
                    SkillCurationLifecycleAction::Unpin => {
                        macaca_skill::SkillUsageEventKind::Unpinned
                    }
                    SkillCurationLifecycleAction::Archive => {
                        macaca_skill::SkillUsageEventKind::Archived
                    }
                    SkillCurationLifecycleAction::Restore => {
                        macaca_skill::SkillUsageEventKind::Restored
                    }
                },
                author_kind: command.author_kind.clone(),
                created_by: None,
                pinned: match &action {
                    SkillCurationLifecycleAction::Pin => Some(true),
                    SkillCurationLifecycleAction::Unpin => Some(false),
                    SkillCurationLifecycleAction::Archive
                    | SkillCurationLifecycleAction::Restore => None,
                },
                evidence_id: sanitized_evidence.first().cloned(),
                metadata: BTreeMap::new(),
            }),
        ))
        .await;

        Ok(SkillCurationLifecycleResult {
            skill_id: result_skill_id,
            name: result_name,
            action,
            lifecycle: result_lifecycle,
            pinned: result_pinned,
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
        command: SkillAliasUpsertCommand,
    ) -> SkillAliasUpsertResult {
        let captured_at = chrono::Utc::now();
        let mut record = command.record;
        if record.created_at > captured_at {
            record.created_at = captured_at;
        }
        record.updated_at = captured_at;
        let mut aliases = self.aliases.lock().await;
        aliases.insert(record.key(), record.clone());
        drop(aliases);
        self.append_event(SkillGovernanceEventRecord::new(
            event_id("skill-governance-alias", captured_at),
            &command.trace,
            command.scope,
            captured_at,
            SkillGovernanceEventPayload::AliasUpserted(record.clone()),
        ))
        .await;

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
        self.append_event(SkillGovernanceEventRecord::new(
            event_id("skill-governance-proposal", captured_at),
            &command.trace,
            command.scope,
            captured_at,
            SkillGovernanceEventPayload::ProposalCreated(proposal.clone()),
        ))
        .await;

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

    async fn append_event(&self, event: SkillGovernanceEventRecord) {
        let _ = <Self as SkillGovernanceStoreStrategy>::append_event(self, event).await;
    }
}

#[async_trait]
impl SkillGovernanceStoreStrategy for SkillProviderGovernanceState {
    async fn append_event(
        &self,
        event: SkillGovernanceEventRecord,
    ) -> Result<SkillGovernanceEventRecord, SkillGovernanceStoreUnavailable> {
        tracing::info!(
            event_id = %event.event_id,
            trace_id = %event.trace_id,
            event_kind = %governance_event_kind(&event.payload),
            application_id = ?event.scope.application_id,
            session_id = ?event.scope.session_id,
            agent_name = ?event.scope.agent_name,
            "skill governance event appended through local compatibility adapter"
        );
        self.event_log.lock().await.push(event.clone());
        Ok(event)
    }

    async fn read_model(
        &self,
    ) -> Result<SkillGovernanceReadModel, SkillGovernanceStoreUnavailable> {
        let events = self.event_log.lock().await.clone();
        let read_model = SkillGovernanceReadModel::from_events(events);
        tracing::info!(
            events = read_model.replayed_events,
            records = read_model.records.len(),
            aliases = read_model.aliases.len(),
            proposals = read_model.proposals.len(),
            curation_runs = read_model.curation_runs.len(),
            rollback_refs = read_model.rollback_refs.len(),
            "skill governance read model replayed through local compatibility adapter"
        );
        Ok(read_model)
    }
}

fn governance_event_kind(payload: &SkillGovernanceEventPayload) -> &'static str {
    match payload {
        SkillGovernanceEventPayload::UsageRecorded(_) => "usage_recorded",
        SkillGovernanceEventPayload::LifecycleApplied(_) => "lifecycle_applied",
        SkillGovernanceEventPayload::AliasUpserted(_) => "alias_upserted",
        SkillGovernanceEventPayload::ProposalCreated(_) => "proposal_created",
        SkillGovernanceEventPayload::CurationRunRecorded(_) => "curation_run_recorded",
        SkillGovernanceEventPayload::SnapshotRefRecorded(_) => "snapshot_ref_recorded",
        SkillGovernanceEventPayload::RollbackRefRecorded(_) => "rollback_ref_recorded",
    }
}
fn event_id(prefix: &str, captured_at: chrono::DateTime<chrono::Utc>) -> String {
    let nanos = captured_at
        .timestamp_nanos_opt()
        .unwrap_or_else(|| captured_at.timestamp_micros() * 1_000);
    format!("{prefix}-{nanos}")
}
