//! Curation command handlers for the runtime-host Skill provider.
//!
//! Keeping curation orchestration in this focused module prevents the generic
//! Skill service adapter and state holder from becoming hidden curation engines.

use std::sync::Arc;

use macaca_proto::{ServiceCallResult, ServiceError, ServiceResult, TraceContext};
use macaca_skill::{
    SkillCurationDryRunCommand, SkillCurationDryRunResult, SkillCurationRollbackCommand,
    SkillCurationRollbackResult, SkillCurationRunCommand, SkillCurationRunResult,
    SkillCurationSnapshotCommand, SkillCurationSnapshotResult, SkillCurationStatusCommand,
    SkillCurationStatusResult, SkillGovernanceEventPayload, SkillGovernanceEventRecord,
    SkillGovernanceReadModel, SkillGovernanceSnapshotRefRecord, SkillGovernanceStoreStrategy,
    SkillRollbackRefRecord,
};
use serde_json::Value;

use crate::skill_service_codec::{decode, service_result, to_value};
use crate::skill_service_provider_curation_log::curation_phase_counts;
use crate::skill_service_provider_state::{event_id, SkillProviderGovernanceState};

pub(crate) async fn dry_run_command(
    state: &Arc<SkillProviderGovernanceState>,
    payload: Value,
    trace: TraceContext,
) -> ServiceResult<ServiceCallResult> {
    let typed: SkillCurationDryRunCommand = decode(payload)?;
    let result = state.curation_dry_run(&typed).await;
    tracing::info!(
        trace_id = %typed.trace.trace_id,
        recommendations = result.recommendations.len(),
        mutated = result.mutated,
        "skill curation dry-run completed"
    );
    Ok(service_result(to_value(result)?, trace))
}

pub(crate) async fn status_command(
    state: &Arc<SkillProviderGovernanceState>,
    payload: Value,
    trace: TraceContext,
) -> ServiceResult<ServiceCallResult> {
    let typed: SkillCurationStatusCommand = decode(payload)?;
    let result = state.curation_status(&typed).await;
    tracing::info!(
        trace_id = %typed.trace.trace_id,
        provider_id = %result.provider_id,
        available = result.available,
        interval_ms = result.interval_ms,
        last_run_id = result.last_run_id.as_deref().unwrap_or(""),
        "skill curation status emitted"
    );
    Ok(service_result(to_value(result)?, trace))
}

pub(crate) async fn run_command(
    state: &Arc<SkillProviderGovernanceState>,
    payload: Value,
    trace: TraceContext,
) -> ServiceResult<ServiceCallResult> {
    let typed: SkillCurationRunCommand = decode(payload)?;
    typed.validate().map_err(ServiceError::InvalidArgument)?;
    let result = state
        .curation_run(typed.clone())
        .await
        .map_err(ServiceError::InvalidArgument)?;
    tracing::info!(
        trace_id = %typed.trace.trace_id,
        run_id = %result.run.run_id,
        dry_run = result.run.dry_run,
        recommendations = result.recommendations.len(),
        report_ref = result.report_ref.as_deref().unwrap_or(""),
        rollback_ref = result.rollback_ref.as_deref().unwrap_or(""),
        mutated = result.mutated,
        "skill curation run completed"
    );
    Ok(service_result(to_value(result)?, trace))
}

pub(crate) async fn snapshot_command(
    state: &Arc<SkillProviderGovernanceState>,
    payload: Value,
    trace: TraceContext,
) -> ServiceResult<ServiceCallResult> {
    let typed: SkillCurationSnapshotCommand = decode(payload)?;
    let result = state.curation_snapshot(typed).await;
    tracing::info!(
        trace_id = %trace.trace_id,
        snapshot_ref = %result.snapshot.snapshot_ref,
        record_count = result.snapshot.record_count,
        rollback_refs = result.rollback_refs.len(),
        package_memento_refs = result.package_memento_refs.len(),
        "skill curation snapshot recorded"
    );
    Ok(service_result(to_value(result)?, trace))
}

pub(crate) async fn rollback_command(
    state: &Arc<SkillProviderGovernanceState>,
    payload: Value,
    trace: TraceContext,
) -> ServiceResult<ServiceCallResult> {
    let typed: SkillCurationRollbackCommand = decode(payload)?;
    typed.validate().map_err(ServiceError::InvalidArgument)?;
    let result = state
        .curation_rollback(typed.clone())
        .await
        .map_err(ServiceError::InvalidArgument)?;
    tracing::info!(
        trace_id = %typed.trace.trace_id,
        rollback_ref = %result.rollback_ref,
        restored_records = result.restored_record_count,
        restored_aliases = result.restored_alias_count,
        restored_reports = result.restored_report_refs.len(),
        package_mementos = result.package_memento_refs.len(),
        mutated = result.mutated,
        "skill curation rollback restored governance memento"
    );
    Ok(service_result(to_value(result)?, trace))
}

mod store;
