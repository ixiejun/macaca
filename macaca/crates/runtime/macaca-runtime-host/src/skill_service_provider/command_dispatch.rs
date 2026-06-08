//! Command Router for the built-in Skill system service provider.
//!
//! Uses the Command pattern: each `SKILL_*` command name maps to a focused handler
//! method on `SkillSystemServiceProvider`.  The router itself stays free of business
//! logic so new commands can be added without growing a monolithic `call` body.

use macaca_proto::{ServiceCallResult, ServiceCommand, ServiceError, ServiceResult, TraceContext};
use macaca_skill::{
    SKILL_ALIAS_RESOLVE_COMMAND, SKILL_ALIAS_SNAPSHOT_COMMAND, SKILL_ALIAS_UPSERT_COMMAND,
    SKILL_CLEANUP_COMMAND, SKILL_CONTENT_MUTATE_COMMAND, SKILL_CURATION_ARCHIVE_COMMAND,
    SKILL_CURATION_DRY_RUN_COMMAND, SKILL_CURATION_MERGE_APPLY_COMMAND, SKILL_CURATION_PIN_COMMAND,
    SKILL_CURATION_QUARANTINE_COMMAND, SKILL_CURATION_REJECT_COMMAND,
    SKILL_CURATION_RELEASE_QUARANTINE_COMMAND, SKILL_CURATION_RESTORE_COMMAND,
    SKILL_CURATION_ROLLBACK_COMMAND, SKILL_CURATION_RUN_COMMAND, SKILL_CURATION_SNAPSHOT_COMMAND,
    SKILL_CURATION_STATUS_COMMAND, SKILL_CURATION_SUPERSEDE_COMMAND, SKILL_CURATION_UNPIN_COMMAND,
    SKILL_EVALUATION_CHECKPOINT_APPEND_COMMAND, SKILL_EVALUATION_REPORT_COMMAND,
    SKILL_EVALUATION_SCORE_COMMAND, SKILL_EVOLUTION_MATERIALIZATION_APPLY_COMMAND,
    SKILL_EVOLUTION_MATERIALIZATION_OPERATOR_RUN_COMMAND,
    SKILL_EVOLUTION_MATERIALIZATION_OPERATOR_SNAPSHOT_COMMAND,
    SKILL_EVOLUTION_PROCESSING_RUN_COMMAND, SKILL_EVOLUTION_PROCESSING_SNAPSHOT_COMMAND,
    SKILL_EVOLUTION_PROMOTE_DRAFT_COMMAND, SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
    SKILL_EVOLUTION_PROPOSE_PATCH_COMMAND, SKILL_EVOLUTION_REJECT_DRAFT_COMMAND,
    SKILL_EVOLUTION_SNAPSHOT_COMMAND, SKILL_EXECUTABLE_LOAD_COMMAND,
    SKILL_GOVERNANCE_RECORD_USAGE_COMMAND, SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
    SKILL_OPERATOR_CATALOG_LIST_COMMAND, SKILL_OPERATOR_CHANGED_COMMAND,
    SKILL_OPERATOR_CONFIG_WRITE_COMMAND, SKILL_OPERATOR_ENABLEMENT_COMMAND,
    SKILL_OPERATOR_MARKDOWN_READ_COMMAND, SKILL_OPERATOR_UNWATCH_COMMAND,
    SKILL_OPERATOR_WATCH_COMMAND, SKILL_SERVICE_SNAPSHOT_COMMAND, SKILL_SNAPSHOT_COMMAND,
    SKILL_STATUS_COMMAND, SKILL_TOOL_CATALOG_COMMAND, SKILL_TOOL_INVOKE_COMMAND,
};

use super::SkillSystemServiceProvider;

impl SkillSystemServiceProvider {
    /// Route a decoded Skill service command to its handler after trace admission.
    pub(crate) async fn dispatch_command(
        &self,
        command: ServiceCommand,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        match command.name.as_str() {
            SKILL_SNAPSHOT_COMMAND => self.handle_skill_snapshot(command).await,
            SKILL_EXECUTABLE_LOAD_COMMAND => self.handle_executable_load(command).await,
            SKILL_TOOL_CATALOG_COMMAND => self.handle_tool_catalog(command).await,
            SKILL_TOOL_INVOKE_COMMAND => self.handle_tool_invoke(command).await,
            SKILL_STATUS_COMMAND => self.handle_status(command).await,
            SKILL_OPERATOR_CATALOG_LIST_COMMAND => self.handle_operator_catalog_list(command).await,
            SKILL_OPERATOR_MARKDOWN_READ_COMMAND => self.handle_operator_markdown_read(command).await,
            SKILL_OPERATOR_CONFIG_WRITE_COMMAND => self.handle_operator_config_write(command).await,
            SKILL_OPERATOR_WATCH_COMMAND => self.handle_operator_watch(command).await,
            SKILL_OPERATOR_UNWATCH_COMMAND => self.handle_operator_unwatch(command).await,
            SKILL_OPERATOR_CHANGED_COMMAND => self.handle_operator_changed(command).await,
            SKILL_OPERATOR_ENABLEMENT_COMMAND => self.handle_operator_enablement(command).await,
            SKILL_SERVICE_SNAPSHOT_COMMAND => self.handle_service_snapshot(command).await,
            SKILL_GOVERNANCE_RECORD_USAGE_COMMAND => {
                self.handle_governance_record_usage(command, trace).await
            }
            SKILL_GOVERNANCE_SNAPSHOT_COMMAND => self.handle_governance_snapshot(command).await,
            SKILL_CURATION_DRY_RUN_COMMAND => {
                self.handle_curation_dry_run(command, trace).await
            }
            SKILL_CURATION_STATUS_COMMAND => self.handle_curation_status(command, trace).await,
            SKILL_CURATION_RUN_COMMAND => self.handle_curation_run(command, trace).await,
            SKILL_CURATION_SNAPSHOT_COMMAND => self.handle_curation_snapshot(command, trace).await,
            SKILL_CURATION_ROLLBACK_COMMAND => self.handle_curation_rollback(command, trace).await,
            SKILL_CURATION_MERGE_APPLY_COMMAND => {
                self.handle_curation_merge_apply(command, trace).await
            }
            SKILL_CURATION_PIN_COMMAND => self.handle_curation_pin(command, trace).await,
            SKILL_CURATION_UNPIN_COMMAND => self.handle_curation_unpin(command, trace).await,
            SKILL_CURATION_ARCHIVE_COMMAND => self.handle_curation_archive(command, trace).await,
            SKILL_CURATION_RESTORE_COMMAND => self.handle_curation_restore(command, trace).await,
            SKILL_CURATION_QUARANTINE_COMMAND => {
                self.handle_curation_quarantine(command, trace).await
            }
            SKILL_CURATION_RELEASE_QUARANTINE_COMMAND => {
                self.handle_curation_release_quarantine(command, trace).await
            }
            SKILL_CURATION_REJECT_COMMAND => self.handle_curation_reject(command, trace).await,
            SKILL_CURATION_SUPERSEDE_COMMAND => self.handle_curation_supersede(command).await,
            SKILL_ALIAS_UPSERT_COMMAND => self.handle_alias_upsert(command, trace).await,
            SKILL_ALIAS_RESOLVE_COMMAND => self.handle_alias_resolve(command).await,
            SKILL_ALIAS_SNAPSHOT_COMMAND => self.handle_alias_snapshot(command).await,
            SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND => {
                self.handle_evolution_propose_from_task(command).await
            }
            SKILL_EVOLUTION_PROPOSE_PATCH_COMMAND => {
                self.handle_evolution_propose_patch(command).await
            }
            SKILL_EVOLUTION_PROMOTE_DRAFT_COMMAND => {
                self.handle_evolution_promote_draft(command).await
            }
            SKILL_EVOLUTION_REJECT_DRAFT_COMMAND => {
                self.handle_evolution_reject_draft(command).await
            }
            SKILL_EVOLUTION_SNAPSHOT_COMMAND => self.handle_evolution_snapshot(command).await,
            SKILL_EVOLUTION_PROCESSING_RUN_COMMAND => {
                self.handle_evolution_processing_run(command, trace).await
            }
            SKILL_EVOLUTION_PROCESSING_SNAPSHOT_COMMAND => {
                self.handle_evolution_processing_snapshot(command, trace).await
            }
            SKILL_EVOLUTION_MATERIALIZATION_APPLY_COMMAND => {
                self.handle_evolution_materialization_apply(command, trace).await
            }
            SKILL_EVOLUTION_MATERIALIZATION_OPERATOR_RUN_COMMAND => {
                self.handle_evolution_materialization_operator_run(command, trace).await
            }
            SKILL_EVOLUTION_MATERIALIZATION_OPERATOR_SNAPSHOT_COMMAND => {
                self.handle_evolution_materialization_operator_snapshot(command, trace).await
            }
            SKILL_EVALUATION_CHECKPOINT_APPEND_COMMAND => {
                self.handle_evaluation_checkpoint_append(command).await
            }
            SKILL_EVALUATION_SCORE_COMMAND => self.handle_evaluation_score(command).await,
            SKILL_EVALUATION_REPORT_COMMAND => self.handle_evaluation_report(command).await,
            SKILL_CONTENT_MUTATE_COMMAND => self.handle_content_mutate(command).await,
            SKILL_CLEANUP_COMMAND => self.handle_cleanup(trace).await,
            other => Err(ServiceError::UnsupportedCommand(format!(
                "unsupported Skill service command '{other}'"
            ))),
        }
    }
}
