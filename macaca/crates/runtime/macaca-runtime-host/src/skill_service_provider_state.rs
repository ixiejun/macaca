//! Mutable state helpers for the built-in Skill system service provider.
//!
//! The public provider remains a thin service adapter: it decodes commands,
//! emits trace-friendly logs, and delegates state transitions here.  Keeping
//! this state module focused prevents the provider adapter from becoming a
//! hidden curation engine while still allowing future Store/EventLog-backed
//! providers to replace the in-memory implementation.

use std::collections::BTreeMap;

use macaca_skill::{
    SkillAliasRecord, SkillAliasResolutionStatus, SkillAliasResolveCommand,
    SkillAliasResolveResult, SkillAliasSnapshotResult, SkillAliasUpsertCommand,
    SkillAliasUpsertResult, SkillCurationLifecycleAction, SkillCurationLifecycleCommand,
    SkillCurationLifecycleResult, SkillCurationSupersedeCommand,
    SkillExperienceDestinationRouteResult, SkillExperienceProposalCommand,
    SkillExperienceProposalRecord, SkillExperienceProposalResult,
    SkillExperienceProposalSnapshotResult, SkillGovernanceEventPayload, SkillGovernanceEventRecord,
    SkillGovernanceRecord, SkillGovernanceRecordUsageCommand, SkillGovernanceRecordUsageResult,
    SkillGovernanceSnapshotResult, SkillGovernanceStoreStrategy, SkillLifecycleState,
    SkillLifecycleStateMachine, SkillPinnedMutationGuard, SkillPinnedMutationOperation,
    SkillTelemetryAggregate, SkillUsageObservation,
};
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

/// In-memory governance state for the built-in provider.
///
/// This type is intentionally small and deterministic.  It is not the final
/// persistence layer; it is the local provider's Strategy implementation for
/// tests, development hosts, and future migration to a durable governance store.
#[derive(Default)]
pub(crate) struct SkillProviderGovernanceState {
    pub(crate) records: Mutex<BTreeMap<String, SkillGovernanceRecord>>,
    pub(crate) aliases: Mutex<BTreeMap<String, SkillAliasRecord>>,
    pub(crate) proposals: Mutex<BTreeMap<String, SkillExperienceProposalRecord>>,
    pub(crate) event_log: Mutex<Vec<SkillGovernanceEventRecord>>,
    pub(crate) curation_mementos: Mutex<BTreeMap<String, SkillCurationRollbackMemento>>,
}

/// Local rollback memento captured before an approved curation apply run.
///
/// A future Store/EventLog strategy can persist this same shape as durable
/// artifacts.  The built-in provider keeps it in memory so rollback semantics
/// can be proven without leaking skill bodies or package bytes into reports.
#[derive(Debug, Clone)]
pub(crate) struct SkillCurationRollbackMemento {
    pub(crate) rollback_ref: String,
    pub(crate) before_snapshot_ref: String,
    pub(crate) after_snapshot_ref: Option<String>,
    pub(crate) records: BTreeMap<String, SkillGovernanceRecord>,
    pub(crate) aliases: BTreeMap<String, SkillAliasRecord>,
    pub(crate) proposals: BTreeMap<String, SkillExperienceProposalRecord>,
    pub(crate) event_log: Vec<SkillGovernanceEventRecord>,
    pub(crate) report_refs: Vec<String>,
    pub(crate) package_memento_refs: Vec<String>,
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
        lifecycle_filters: Vec<SkillLifecycleState>,
    ) -> SkillGovernanceSnapshotResult {
        let all_records = self
            .records
            .lock()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        let filtered_out = all_records
            .iter()
            .filter(|record| {
                !snapshot_includes_lifecycle(
                    &record.lifecycle,
                    include_archived,
                    &lifecycle_filters,
                )
            })
            .count();
        let mut records: Vec<_> = all_records
            .into_iter()
            .filter(|record| {
                snapshot_includes_lifecycle(&record.lifecycle, include_archived, &lifecycle_filters)
            })
            .collect();
        records.sort_by(|left, right| left.provenance.skill_id.cmp(&right.provenance.skill_id));
        let telemetry_aggregate = SkillTelemetryAggregate::from_records(records.iter());
        tracing::info!(
            include_archived = include_archived,
            lifecycle_filters = lifecycle_filters.len(),
            records = records.len(),
            filtered_records = filtered_out,
            successful_tasks = telemetry_aggregate.successful_task_count,
            failed_tasks = telemetry_aggregate.failed_task_count,
            "skill governance snapshot built through local compatibility adapter"
        );

        SkillGovernanceSnapshotResult {
            records,
            telemetry_aggregate,
            captured_at: chrono::Utc::now(),
        }
    }

    /// Return a bounded telemetry rollup for lightweight status surfaces.
    pub(crate) async fn telemetry_aggregate(&self) -> SkillTelemetryAggregate {
        let records = self
            .records
            .lock()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        SkillTelemetryAggregate::from_records(records.iter())
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
        let sanitized_policy_decisions = command
            .policy_decision_refs
            .iter()
            .filter(|id| !id.trim().is_empty())
            .cloned()
            .collect::<Vec<_>>();
        let mut records = self.records.lock().await;
        let record = records.entry(key).or_insert_with(|| {
            let mut record = SkillGovernanceRecord::from_lifecycle_command(&command, captured_at);
            if action == SkillCurationLifecycleAction::Reject {
                record.lifecycle = SkillLifecycleState::Draft;
            }
            record
        });

        if action == SkillCurationLifecycleAction::Archive {
            SkillPinnedMutationGuard::ensure_not_pinned(
                record.pinned,
                &SkillPinnedMutationOperation::Archive,
            )?;
        }
        if action == SkillCurationLifecycleAction::Supersede {
            SkillPinnedMutationGuard::ensure_not_pinned(
                record.pinned,
                &SkillPinnedMutationOperation::Supersede,
            )?;
        }
        if action == SkillCurationLifecycleAction::ReleaseQuarantine
            && record.lifecycle != SkillLifecycleState::Quarantined
        {
            return Err("quarantine release requires a quarantined skill lifecycle".into());
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
            SkillCurationLifecycleAction::Quarantine => SkillLifecycleState::Quarantined,
            SkillCurationLifecycleAction::ReleaseQuarantine => SkillLifecycleState::Active,
            SkillCurationLifecycleAction::Supersede => SkillLifecycleState::Superseded,
            SkillCurationLifecycleAction::Reject => SkillLifecycleState::Rejected,
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
        tracing::info!(
            trace_id = %command.trace.trace_id,
            skill_id = %command.skill_id,
            action = ?action,
            evidence_refs = sanitized_evidence.len(),
            policy_decision_refs = sanitized_policy_decisions.len(),
            "skill governance lifecycle mutation accepted"
        );
        let mut lifecycle_metadata = BTreeMap::new();
        if let Some(task_id) = command.task_id.as_ref().filter(|id| !id.trim().is_empty()) {
            lifecycle_metadata.insert("task_id".into(), task_id.clone());
        }
        for (index, evidence_id) in sanitized_evidence.iter().enumerate().skip(1) {
            lifecycle_metadata.insert(format!("evidence_ref.{index}"), evidence_id.clone());
        }
        let mut event = SkillGovernanceEventRecord::new(
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
                    SkillCurationLifecycleAction::Quarantine => {
                        macaca_skill::SkillUsageEventKind::Quarantined
                    }
                    SkillCurationLifecycleAction::ReleaseQuarantine => {
                        macaca_skill::SkillUsageEventKind::QuarantineReleased
                    }
                    SkillCurationLifecycleAction::Supersede => {
                        macaca_skill::SkillUsageEventKind::Superseded
                    }
                    SkillCurationLifecycleAction::Reject => {
                        macaca_skill::SkillUsageEventKind::Rejected
                    }
                },
                author_kind: command.author_kind.clone(),
                created_by: command.scope.agent_name.clone(),
                pinned: match &action {
                    SkillCurationLifecycleAction::Pin => Some(true),
                    SkillCurationLifecycleAction::Unpin => Some(false),
                    SkillCurationLifecycleAction::Archive
                    | SkillCurationLifecycleAction::Restore
                    | SkillCurationLifecycleAction::Quarantine
                    | SkillCurationLifecycleAction::ReleaseQuarantine
                    | SkillCurationLifecycleAction::Supersede
                    | SkillCurationLifecycleAction::Reject => None,
                },
                evidence_id: sanitized_evidence.first().cloned(),
                metadata: lifecycle_metadata,
            }),
        );
        event.policy_decision_ids = sanitized_policy_decisions.clone();
        self.append_event(event).await;

        Ok(SkillCurationLifecycleResult {
            skill_id: result_skill_id,
            name: result_name,
            action,
            lifecycle: result_lifecycle,
            pinned: result_pinned,
            mutated: true,
            reason: command.reason,
            evidence_ids: sanitized_evidence,
            policy_decision_refs: sanitized_policy_decisions,
            trace_id: command.trace.trace_id,
            captured_at,
        })
    }

    /// Upsert the required alias before hiding the source skill as superseded.
    pub(crate) async fn supersede(
        &self,
        command: SkillCurationSupersedeCommand,
    ) -> Result<SkillCurationLifecycleResult, String> {
        let alias_command = SkillAliasUpsertCommand {
            trace: command.lifecycle.trace.clone(),
            scope: command.lifecycle.scope.clone(),
            record: command.alias,
        };
        self.upsert_alias(alias_command).await;
        self.apply_lifecycle(command.lifecycle, SkillCurationLifecycleAction::Supersede)
            .await
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
        let Some(record) = aliases.get(&command.key()).cloned() else {
            debug!(
                requested_skill_id = %command.skill_id,
                requested_name = ?command.name,
                trace_id = %command.trace.trace_id,
                "skill alias miss in governance state"
            );
            return SkillAliasResolveResult::unresolved(command, captured_at);
        };

        if !record.is_active_at(captured_at) {
            info!(
                source_skill_id = %record.source_skill_id,
                target_skill_id = %record.target_skill_id,
                trace_id = %command.trace.trace_id,
                "expired skill alias decision skipped redirect"
            );
            return SkillAliasResolveResult::blocked(
                command,
                record,
                captured_at,
                SkillAliasResolutionStatus::Expired,
            );
        }

        if aliases
            .get(&record.target_key())
            .map(|next| next.target_key() == record.key())
            .unwrap_or(false)
            || record.target_key() == record.key()
        {
            warn!(
                source_skill_id = %record.source_skill_id,
                target_skill_id = %record.target_skill_id,
                trace_id = %command.trace.trace_id,
                "skill alias loop prevented before redirect"
            );
            return SkillAliasResolveResult::blocked(
                command,
                record,
                captured_at,
                SkillAliasResolutionStatus::LoopPrevented,
            );
        }

        let result = SkillAliasResolveResult::resolved(command, record, captured_at);
        match &result.status {
            SkillAliasResolutionStatus::WarnAndRedirected => info!(
                requested_skill_id = %result.requested_skill_id,
                target_skill_id = ?result.target_skill_id,
                trace_id = %command.trace.trace_id,
                "skill alias warn-and-redirect decision resolved"
            ),
            SkillAliasResolutionStatus::Denied => warn!(
                requested_skill_id = %result.requested_skill_id,
                trace_id = %command.trace.trace_id,
                "skill alias deny decision blocked redirect"
            ),
            _ => info!(
                requested_skill_id = %result.requested_skill_id,
                target_skill_id = ?result.target_skill_id,
                status = ?result.status,
                trace_id = %command.trace.trace_id,
                "skill alias decision resolved"
            ),
        }
        result
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
        destination_route: SkillExperienceDestinationRouteResult,
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
            destination_route,
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

    pub(crate) async fn append_event(&self, event: SkillGovernanceEventRecord) {
        let _ = <Self as SkillGovernanceStoreStrategy>::append_event(self, event).await;
    }
}

fn snapshot_includes_lifecycle(
    lifecycle: &SkillLifecycleState,
    include_archived: bool,
    lifecycle_filters: &[SkillLifecycleState],
) -> bool {
    if !lifecycle_filters.is_empty() {
        return lifecycle_filters.contains(lifecycle);
    }
    include_archived || lifecycle != &SkillLifecycleState::Archived
}

pub(crate) fn event_id(prefix: &str, captured_at: chrono::DateTime<chrono::Utc>) -> String {
    let nanos = captured_at
        .timestamp_nanos_opt()
        .unwrap_or_else(|| captured_at.timestamp_micros() * 1_000);
    format!("{prefix}-{nanos}")
}
