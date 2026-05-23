//! Lifecycle command helpers for the built-in Skill service provider.
//!
//! The provider adapter stays focused on service dispatch, while this module
//! owns the Command-to-state handoff for lifecycle curation commands.  Keeping
//! the helper separate preserves the service boundary without growing the main
//! provider file into a curation engine.

use std::sync::Arc;

use macaca_proto::{ServiceCallResult, ServiceError, ServiceResult, TraceContext};
use macaca_skill::{
    SkillCurationLifecycleAction, SkillCurationLifecycleCommand, SkillCurationSupersedeCommand,
};

use crate::skill_service_provider::{service_result, to_value};
use crate::skill_service_provider_state::SkillProviderGovernanceState;

/// Decode and apply one metadata-only lifecycle curation command.
pub(crate) async fn apply_lifecycle_command(
    governance_state: &Arc<SkillProviderGovernanceState>,
    payload: serde_json::Value,
    trace: TraceContext,
    action: SkillCurationLifecycleAction,
) -> ServiceResult<ServiceCallResult> {
    let typed: SkillCurationLifecycleCommand = crate::skill_service_provider::decode(payload)?;
    typed.validate().map_err(ServiceError::InvalidArgument)?;
    let result = governance_state
        .apply_lifecycle(typed.clone(), action.clone())
        .await
        .map_err(ServiceError::InvalidArgument)?;
    tracing::info!(
        trace_id = %typed.trace.trace_id,
        skill_id = %result.skill_id,
        action = ?action,
        lifecycle = ?result.lifecycle,
        pinned = result.pinned,
        mutated = result.mutated,
        "skill curation lifecycle metadata updated"
    );
    Ok(service_result(to_value(result)?, trace))
}

/// Decode and apply a supersede command whose alias must be durable first.
pub(crate) async fn supersede_command(
    governance_state: &Arc<SkillProviderGovernanceState>,
    payload: serde_json::Value,
) -> ServiceResult<ServiceCallResult> {
    let typed: SkillCurationSupersedeCommand = crate::skill_service_provider::decode(payload)?;
    typed.validate().map_err(ServiceError::InvalidArgument)?;
    let result = governance_state
        .supersede(typed.clone())
        .await
        .map_err(ServiceError::InvalidArgument)?;
    tracing::info!(
        trace_id = %typed.lifecycle.trace.trace_id,
        skill_id = %result.skill_id,
        target_skill_id = %typed.alias.target_skill_id,
        "skill curation supersede metadata updated with alias"
    );
    Ok(service_result(to_value(result)?, typed.lifecycle.trace))
}
