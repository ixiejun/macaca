//! Shared service-call bridge for the host Skill client facade.

use std::sync::Arc;

use macaca_proto::{MacacaError, MacacaResult};

use crate::runtime_host::{
    SkillCurationLifecycleAction, SKILL_CURATION_ARCHIVE_COMMAND, SKILL_CURATION_PIN_COMMAND,
    SKILL_CURATION_QUARANTINE_COMMAND, SKILL_CURATION_REJECT_COMMAND,
    SKILL_CURATION_RELEASE_QUARANTINE_COMMAND, SKILL_CURATION_RESTORE_COMMAND,
    SKILL_CURATION_UNPIN_COMMAND, SKILL_SERVICE_ID,
};

/// Map curation lifecycle actions to the canonical Skill service command name.
pub(super) fn curation_lifecycle_command_name(
    action: &SkillCurationLifecycleAction,
) -> MacacaResult<&'static str> {
    Ok(match action {
        SkillCurationLifecycleAction::Pin => SKILL_CURATION_PIN_COMMAND,
        SkillCurationLifecycleAction::Unpin => SKILL_CURATION_UNPIN_COMMAND,
        SkillCurationLifecycleAction::Archive => SKILL_CURATION_ARCHIVE_COMMAND,
        SkillCurationLifecycleAction::Restore => SKILL_CURATION_RESTORE_COMMAND,
        SkillCurationLifecycleAction::Quarantine => SKILL_CURATION_QUARANTINE_COMMAND,
        SkillCurationLifecycleAction::ReleaseQuarantine => {
            SKILL_CURATION_RELEASE_QUARANTINE_COMMAND
        }
        SkillCurationLifecycleAction::Supersede => {
            return Err(MacacaError::Config(
                "supersede requires a SkillCurationSupersedeCommand with alias evidence".into(),
            ));
        }
        SkillCurationLifecycleAction::Reject => SKILL_CURATION_REJECT_COMMAND,
    })
}

/// Dispatch a typed Skill command through the generic SDK service client.
pub(super) async fn call<T, R>(
    service: &Arc<dyn macaca_sdk::SystemServiceClient>,
    command_name: &str,
    trace: macaca_proto::TraceContext,
    payload: T,
) -> MacacaResult<R>
where
    T: serde::Serialize,
    R: serde::de::DeserializeOwned,
{
    // The outer service command carries the same trace as the typed payload so
    // trace-required middleware can audit the boundary before provider dispatch.
    let service_command = macaca_sdk::ServiceCallCommand::new(
        SKILL_SERVICE_ID,
        command_name,
        serde_json::to_value(payload)?,
    )?
    .with_trace(trace);
    let result = service.call_service(&service_command).await?;
    serde_json::from_value(result.output).map_err(MacacaError::from)
}
