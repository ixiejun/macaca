//! Strategy bridge handlers for curation, lifecycle, and materialization commands.
//!
//! These commands delegate to sibling runtime-host modules that already encapsulate
//! curation engines, lifecycle State machines, and materialization operators.
//! The provider adapter only forwards typed payloads and trace contexts.

use macaca_proto::{ServiceCallResult, ServiceCommand, ServiceResult, TraceContext};
use macaca_skill::SkillCurationLifecycleAction;

use super::SkillSystemServiceProvider;

impl SkillSystemServiceProvider {
    /// Run a curation dry-run without mutating active governance state.
    pub(crate) async fn handle_curation_dry_run(
        &self,
        command: ServiceCommand,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        crate::skill_service_provider_curation::dry_run_command(
            &self.governance_state,
            command.payload,
            trace,
        )
        .await
    }

    /// Report read-only curation status from local provider state.
    pub(crate) async fn handle_curation_status(
        &self,
        command: ServiceCommand,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        crate::skill_service_provider_curation::status_command(
            &self.governance_state,
            command.payload,
            trace,
        )
        .await
    }

    /// Execute a curation run and record bounded artifacts.
    pub(crate) async fn handle_curation_run(
        &self,
        command: ServiceCommand,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        crate::skill_service_provider_curation::run_command(
            &self.governance_state,
            command.payload,
            trace,
        )
        .await
    }

    /// Emit curation snapshot references without skill bodies.
    pub(crate) async fn handle_curation_snapshot(
        &self,
        command: ServiceCommand,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        crate::skill_service_provider_curation::snapshot_command(
            &self.governance_state,
            command.payload,
            trace,
        )
        .await
    }

    /// Roll back curation state from a captured memento.
    pub(crate) async fn handle_curation_rollback(
        &self,
        command: ServiceCommand,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        crate::skill_service_provider_curation::rollback_command(
            &self.governance_state,
            command.payload,
            trace,
        )
        .await
    }

    /// Apply an approved merge curation action.
    pub(crate) async fn handle_curation_merge_apply(
        &self,
        command: ServiceCommand,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        crate::skill_service_provider_merge::merge_apply_command(
            &self.governance_state,
            command.payload,
            trace,
        )
        .await
    }

    /// Pin a skill record in the governance lifecycle state machine.
    pub(crate) async fn handle_curation_pin(
        &self,
        command: ServiceCommand,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        crate::skill_service_provider_lifecycle::apply_lifecycle_command(
            &self.governance_state,
            command.payload,
            trace,
            SkillCurationLifecycleAction::Pin,
        )
        .await
    }

    /// Unpin a skill record in the governance lifecycle state machine.
    pub(crate) async fn handle_curation_unpin(
        &self,
        command: ServiceCommand,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        crate::skill_service_provider_lifecycle::apply_lifecycle_command(
            &self.governance_state,
            command.payload,
            trace,
            SkillCurationLifecycleAction::Unpin,
        )
        .await
    }

    /// Archive a skill record through the lifecycle strategy.
    pub(crate) async fn handle_curation_archive(
        &self,
        command: ServiceCommand,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        crate::skill_service_provider_lifecycle::apply_lifecycle_command(
            &self.governance_state,
            command.payload,
            trace,
            SkillCurationLifecycleAction::Archive,
        )
        .await
    }

    /// Restore an archived skill record.
    pub(crate) async fn handle_curation_restore(
        &self,
        command: ServiceCommand,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        crate::skill_service_provider_lifecycle::apply_lifecycle_command(
            &self.governance_state,
            command.payload,
            trace,
            SkillCurationLifecycleAction::Restore,
        )
        .await
    }

    /// Quarantine a skill record pending review.
    pub(crate) async fn handle_curation_quarantine(
        &self,
        command: ServiceCommand,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        crate::skill_service_provider_lifecycle::apply_lifecycle_command(
            &self.governance_state,
            command.payload,
            trace,
            SkillCurationLifecycleAction::Quarantine,
        )
        .await
    }

    /// Release a quarantined skill record after policy approval.
    pub(crate) async fn handle_curation_release_quarantine(
        &self,
        command: ServiceCommand,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        crate::skill_service_provider_lifecycle::apply_lifecycle_command(
            &self.governance_state,
            command.payload,
            trace,
            SkillCurationLifecycleAction::ReleaseQuarantine,
        )
        .await
    }

    /// Reject a skill record through the lifecycle strategy.
    pub(crate) async fn handle_curation_reject(
        &self,
        command: ServiceCommand,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        crate::skill_service_provider_lifecycle::apply_lifecycle_command(
            &self.governance_state,
            command.payload,
            trace,
            SkillCurationLifecycleAction::Reject,
        )
        .await
    }

    /// Supersede one skill identity with another through lifecycle orchestration.
    pub(crate) async fn handle_curation_supersede(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        crate::skill_service_provider_lifecycle::supersede_command(
            &self.governance_state,
            command.payload,
        )
        .await
    }

    /// Run proposal processing for queued experience proposals.
    pub(crate) async fn handle_evolution_processing_run(
        &self,
        command: ServiceCommand,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        crate::skill_service_provider_proposal_processing::run_command(
            &self.governance_state,
            command.payload,
            trace,
        )
        .await
    }

    /// Emit a snapshot of proposal processing state.
    pub(crate) async fn handle_evolution_processing_snapshot(
        &self,
        command: ServiceCommand,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        crate::skill_service_provider_proposal_processing::snapshot_command(
            &self.governance_state,
            command.payload,
            trace,
        )
        .await
    }

    /// Apply approved materialization for a proposal.
    pub(crate) async fn handle_evolution_materialization_apply(
        &self,
        command: ServiceCommand,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        crate::skill_service_provider_proposal_materialization::apply_command(
            &self.governance_state,
            &self.content_mutation,
            command.payload,
            trace,
        )
        .await
    }

    /// Run the autonomous materialization operator.
    pub(crate) async fn handle_evolution_materialization_operator_run(
        &self,
        command: ServiceCommand,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        crate::skill_service_provider_materialization_operator::run_command(
            &self.governance_state,
            &self.content_mutation,
            command.payload,
            trace,
        )
        .await
    }

    /// Emit a snapshot of materialization operator runs.
    pub(crate) async fn handle_evolution_materialization_operator_snapshot(
        &self,
        command: ServiceCommand,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        crate::skill_service_provider_materialization_operator::snapshot_command(
            &self.governance_state,
            command.payload,
            trace,
        )
        .await
    }
}
