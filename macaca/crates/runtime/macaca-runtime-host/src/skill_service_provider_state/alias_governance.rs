//! Alias governance for Skill provider state.
//!
//! Owns alias upsert, resolution with loop prevention, and deterministic
//! diagnostic snapshots.  Resolution outcomes are logged with trace identifiers
//! so audit replay can reconstruct redirect decisions without package reads.

use macaca_skill::{
    SkillAliasResolutionStatus, SkillAliasResolveCommand, SkillAliasResolveResult,
    SkillAliasSnapshotResult, SkillAliasUpsertCommand, SkillAliasUpsertResult,
    SkillGovernanceEventPayload, SkillGovernanceEventRecord,
};
use tracing::{debug, info, warn};

use super::{event_id, SkillProviderGovernanceState};

impl SkillProviderGovernanceState {
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
}
