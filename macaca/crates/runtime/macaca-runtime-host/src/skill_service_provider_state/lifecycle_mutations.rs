//! Lifecycle mutation State machine for Skill governance records.
//!
//! Applies metadata-only lifecycle transitions through the shared
//! `SkillLifecycleStateMachine` validator.  Package files, aliases, and
//! executable scripts are intentionally out of scope here; supersede composes
//! alias upsert with lifecycle mutation via the alias governance module.

use std::collections::BTreeMap;

use macaca_skill::{
    SkillCurationLifecycleAction, SkillCurationLifecycleCommand, SkillCurationLifecycleResult,
    SkillCurationSupersedeCommand, SkillGovernanceEventPayload, SkillGovernanceEventRecord,
    SkillGovernanceRecord, SkillLifecycleState, SkillLifecycleStateMachine,
    SkillPinnedMutationGuard, SkillPinnedMutationOperation, SkillUsageObservation,
};

use super::{event_id, SkillProviderGovernanceState};

impl SkillProviderGovernanceState {
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
        let alias_command = macaca_skill::SkillAliasUpsertCommand {
            trace: command.lifecycle.trace.clone(),
            scope: command.lifecycle.scope.clone(),
            record: command.alias,
        };
        self.upsert_alias(alias_command).await;
        self.apply_lifecycle(command.lifecycle, SkillCurationLifecycleAction::Supersede)
            .await
    }
}
