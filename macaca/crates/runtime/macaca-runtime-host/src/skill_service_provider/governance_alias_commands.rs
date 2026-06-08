//! Governance read-model and alias command handlers.
//!
//! These handlers mutate or query `SkillProviderGovernanceState` while emitting
//! trace-friendly audit logs that never include raw skill bodies or payloads.

use macaca_proto::{ServiceCallResult, ServiceCommand, ServiceResult, TraceContext};
use macaca_skill::{
    SkillAliasResolveCommand, SkillAliasSnapshotCommand, SkillAliasUpsertCommand,
    SkillGovernanceRecordUsageCommand, SkillGovernanceSnapshotCommand,
};

use crate::skill_service_codec::{decode, service_result, to_value};

use super::SkillSystemServiceProvider;

impl SkillSystemServiceProvider {
    /// Record a bounded usage observation against the governance read model.
    pub(crate) async fn handle_governance_record_usage(
        &self,
        command: ServiceCommand,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: SkillGovernanceRecordUsageCommand = decode(command.payload)?;
        let key = typed.observation.key();
        let event = typed.observation.event.clone();
        let result = self.governance_state.record_usage(typed).await;
        tracing::info!(
            trace_id = %trace.trace_id,
            skill_id = %key,
            event = ?event,
            "skill governance usage observation recorded"
        );
        Ok(service_result(to_value(result)?, trace))
    }

    /// Emit a governance snapshot filtered by lifecycle state.
    pub(crate) async fn handle_governance_snapshot(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: SkillGovernanceSnapshotCommand = decode(command.payload)?;
        let result = self
            .governance_state
            .governance_snapshot(typed.include_archived, typed.lifecycle_filters)
            .await;
        tracing::info!(
            trace_id = %typed.trace.trace_id,
            records = result.records.len(),
            "skill governance snapshot emitted"
        );
        Ok(service_result(to_value(result)?, typed.trace))
    }

    /// Upsert a skill alias record for absorption or deprecation routing.
    pub(crate) async fn handle_alias_upsert(
        &self,
        command: ServiceCommand,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: SkillAliasUpsertCommand = decode(command.payload)?;
        let source_skill_id = typed.record.source_skill_id.clone();
        let target_skill_id = typed.record.target_skill_id.clone();
        let result = self.governance_state.upsert_alias(typed).await;
        tracing::info!(
            trace_id = %trace.trace_id,
            source_skill_id = %source_skill_id,
            target_skill_id = %target_skill_id,
            "skill alias record upserted"
        );
        Ok(service_result(to_value(result)?, trace))
    }

    /// Resolve a skill id through the alias graph without fabricating fallbacks.
    pub(crate) async fn handle_alias_resolve(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: SkillAliasResolveCommand = decode(command.payload)?;
        let result = self.governance_state.resolve_alias(&typed).await;
        tracing::info!(
            trace_id = %typed.trace.trace_id,
            skill_id = %typed.skill_id,
            resolved = result.resolved,
            "skill alias resolution completed"
        );
        Ok(service_result(to_value(result)?, typed.trace))
    }

    /// Emit a diagnostic alias snapshot for governance operators.
    pub(crate) async fn handle_alias_snapshot(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: SkillAliasSnapshotCommand = decode(command.payload)?;
        let result = self.governance_state.alias_snapshot().await;
        tracing::info!(
            trace_id = %typed.trace.trace_id,
            aliases = result.aliases.len(),
            "skill alias snapshot emitted"
        );
        Ok(service_result(to_value(result)?, typed.trace))
    }
}
