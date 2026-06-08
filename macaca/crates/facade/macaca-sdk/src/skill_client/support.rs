//! Shared service-call bridge for the SDK Skill client facade.
//!
//! Every runtime-backed Skill command crosses the generic `SystemServiceClient`
//! boundary through this helper.  Centralizing serialization keeps the
//! service-backed implementation a thin Adapter over command DTOs.

use std::sync::Arc;

use macaca_proto::{MacacaError, MacacaResult};
use macaca_skill::{SkillCurationLifecycleAction, SKILL_SERVICE_ID,
    SKILL_CURATION_ARCHIVE_COMMAND, SKILL_CURATION_PIN_COMMAND, SKILL_CURATION_QUARANTINE_COMMAND,
    SKILL_CURATION_REJECT_COMMAND, SKILL_CURATION_RELEASE_QUARANTINE_COMMAND,
    SKILL_CURATION_RESTORE_COMMAND, SKILL_CURATION_UNPIN_COMMAND,
};

use crate::service_client::{ServiceCallCommand, SystemServiceClient};

/// Map a curation lifecycle action to the canonical Skill service command name.
///
/// The Strategy keeps lifecycle routing out of the Adapter impl so new actions can
/// be added in one place without touching every trait method.
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
///
/// The helper wraps the payload in a [`ServiceCallCommand`], forwards the trace
/// context for audit correlation, and deserializes the service output envelope.
pub(super) async fn call<T, R>(
    service: &Arc<dyn SystemServiceClient>,
    command_name: &str,
    trace: macaca_proto::TraceContext,
    payload: T,
) -> MacacaResult<R>
where
    T: serde::Serialize,
    R: serde::de::DeserializeOwned,
{
    let service_command = ServiceCallCommand::new(
        SKILL_SERVICE_ID,
        command_name,
        serde_json::to_value(payload)?,
    )?
    .with_trace(trace);
    let result = service.call_service(&service_command).await?;
    serde_json::from_value(result.output).map_err(MacacaError::from)
}
