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
    SkillLifecycleState, SkillUsageObservation,
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

        match &action {
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
            "skill governance event appended through local compatibility adapter"
        );
        self.event_log.lock().await.push(event.clone());
        Ok(event)
    }

    async fn read_model(
        &self,
    ) -> Result<SkillGovernanceReadModel, SkillGovernanceStoreUnavailable> {
        let events = self.event_log.lock().await.clone();
        tracing::info!(
            events = events.len(),
            "skill governance read model replayed through local compatibility adapter"
        );
        Ok(SkillGovernanceReadModel::from_events(events))
    }
}

fn event_id(prefix: &str, captured_at: chrono::DateTime<chrono::Utc>) -> String {
    let nanos = captured_at
        .timestamp_nanos_opt()
        .unwrap_or_else(|| captured_at.timestamp_micros() * 1_000);
    format!("{prefix}-{nanos}")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use macaca_proto::TraceContext;
    use macaca_skill::{
        SkillAliasKind, SkillAliasRecord, SkillAliasUpsertCommand, SkillAuthorKind,
        SkillCurationLifecycleAction, SkillCurationLifecycleCommand,
        SkillEvolutionCandidateClassification, SkillEvolutionProposalAction,
        SkillExperienceCandidate, SkillExperienceProposalCommand,
        SkillGovernanceRecordUsageCommand, SkillGovernanceStoreStrategy, SkillServicePolicyHints,
        SkillServiceScope, SkillUsageEventKind, SkillUsageObservation,
    };

    use super::SkillProviderGovernanceState;

    #[tokio::test]
    async fn local_governance_state_replays_through_store_strategy_adapter() {
        let state = SkillProviderGovernanceState::default();
        let trace = TraceContext::new("trace-local-governance-store-adapter");
        let scope = SkillServiceScope::default();

        state
            .record_usage(SkillGovernanceRecordUsageCommand {
                trace: trace.clone(),
                scope: scope.clone(),
                observation: SkillUsageObservation {
                    skill_id: "skill://agent/local".into(),
                    name: "local-skill".into(),
                    source: "test".into(),
                    source_scope: "workspace".into(),
                    event: SkillUsageEventKind::Used,
                    author_kind: SkillAuthorKind::Agent,
                    created_by: Some("agent".into()),
                    pinned: None,
                    evidence_id: Some("usage-evidence-1".into()),
                    metadata: BTreeMap::new(),
                },
            })
            .await;
        state
            .apply_lifecycle(
                SkillCurationLifecycleCommand {
                    trace: trace.clone(),
                    scope: scope.clone(),
                    skill_id: "skill://agent/local".into(),
                    name: "local-skill".into(),
                    source: "test".into(),
                    source_scope: "workspace".into(),
                    author_kind: SkillAuthorKind::Agent,
                    reason: "verified stale lifecycle metadata".into(),
                    evidence_ids: vec!["lifecycle-evidence-1".into()],
                    policy: SkillServicePolicyHints::default(),
                },
                SkillCurationLifecycleAction::Archive,
            )
            .await
            .expect("local lifecycle adapter should accept evidence");
        let now = chrono::Utc::now();
        state
            .upsert_alias(SkillAliasUpsertCommand {
                trace: trace.clone(),
                scope: scope.clone(),
                record: SkillAliasRecord {
                    source_skill_id: "skill://agent/old-local".into(),
                    source_name: "old-local".into(),
                    target_skill_id: "skill://agent/local".into(),
                    target_name: "local-skill".into(),
                    kind: SkillAliasKind::SupersededBy,
                    rationale: "local test alias".into(),
                    created_at: now,
                    updated_at: now,
                    evidence_ids: vec!["alias-evidence-1".into()],
                },
            })
            .await;
        state
            .propose_experience(SkillExperienceProposalCommand {
                trace: trace.clone(),
                scope,
                candidate: SkillExperienceCandidate {
                    task_id: "task-local-1".into(),
                    session_id: Some("session-local-1".into()),
                    application_id: None,
                    agent_name: Some("agent".into()),
                    bounded_summary: "Verified local governance replay evidence.".into(),
                    reusable_procedure: "Replay append-only local governance events through the store strategy adapter.".into(),
                    classification: SkillEvolutionCandidateClassification::ReusableProcedure,
                    recommended_action: SkillEvolutionProposalAction::CreateDraft,
                    target_skill_id: None,
                    target_skill_name: Some("local-governance-replay".into()),
                    evidence_ids: vec!["proposal-evidence-1".into()],
                    metadata: BTreeMap::new(),
                },
            })
            .await;

        let read_model = state
            .read_model()
            .await
            .expect("local in-memory state should replay through the store strategy");

        assert_eq!(read_model.records.len(), 1);
        assert_eq!(read_model.records[0].telemetry.activation_count, 1);
        assert_eq!(read_model.aliases.len(), 1);
        assert_eq!(read_model.proposals.len(), 1);
        assert_eq!(read_model.replayed_events, 4);
    }
}
