//! Curation command handlers for the runtime-host Skill provider.
//!
//! Keeping curation orchestration in this focused module prevents the generic
//! Skill service adapter and state holder from becoming hidden curation engines.

use std::sync::Arc;

use macaca_proto::{ServiceCallResult, ServiceError, ServiceResult, TraceContext};
use macaca_skill::{
    SkillCurationDryRunCommand, SkillCurationDryRunResult, SkillCurationRunCommand,
    SkillCurationRunResult, SkillCurationSnapshotCommand, SkillCurationSnapshotResult,
    SkillCurationStatusCommand, SkillCurationStatusResult, SkillGovernanceEventPayload,
    SkillGovernanceEventRecord, SkillGovernanceReadModel, SkillGovernanceSnapshotRefRecord,
    SkillGovernanceStoreStrategy,
};
use serde_json::Value;

use crate::skill_service_codec::{decode, service_result, to_value};
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

impl SkillProviderGovernanceState {
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

    /// Return read-only curation runner readiness and the latest durable run.
    pub(crate) async fn curation_status(
        &self,
        command: &SkillCurationStatusCommand,
    ) -> SkillCurationStatusResult {
        let captured_at = chrono::Utc::now();
        let last_run = self
            .event_log
            .lock()
            .await
            .iter()
            .filter_map(|event| match &event.payload {
                SkillGovernanceEventPayload::CurationRunRecorded(record) => Some(record.clone()),
                _ => None,
            })
            .max_by_key(|record| record.finished_at.unwrap_or(record.started_at));
        let last_finished_at = last_run.as_ref().and_then(|run| run.finished_at);
        let idle_for_ms = last_finished_at
            .and_then(|finished_at| captured_at.signed_duration_since(finished_at).to_std().ok())
            .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64);
        let next_eligible_run_at = last_finished_at.and_then(|finished_at| {
            chrono::Duration::from_std(std::time::Duration::from_millis(command.interval_ms))
                .ok()
                .map(|interval| finished_at + interval)
        });

        SkillCurationStatusResult {
            provider_id: "local-skill-governance".into(),
            available: true,
            interval_ms: command.interval_ms,
            idle_budget_ms: command.idle_budget_ms,
            idle_for_ms,
            last_run_id: last_run.map(|run| run.run_id),
            last_run_finished_at: last_finished_at,
            next_eligible_run_at,
            unavailable_reason: None,
            captured_at,
        }
    }

    /// Execute a governed curation run through the local Strategy.
    pub(crate) async fn curation_run(
        &self,
        command: SkillCurationRunCommand,
    ) -> Result<SkillCurationRunResult, String> {
        let started_at = chrono::Utc::now();
        let records = self
            .records
            .lock()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        let finished_at = chrono::Utc::now();
        let result =
            SkillCurationRunResult::from_records(records, &command, started_at, finished_at);
        let mut event = SkillGovernanceEventRecord::new(
            event_id("skill-governance-curation-run", finished_at),
            &command.trace,
            command.scope,
            finished_at,
            SkillGovernanceEventPayload::CurationRunRecorded(result.run.clone()),
        );
        event.policy_decision_ids = result.run.policy_decision_ids.clone();
        event.audit_event_ids = result.run.audit_event_ids.clone();
        let _ = <Self as SkillGovernanceStoreStrategy>::append_event(self, event).await;
        tracing::info!(
            trace_id = %command.trace.trace_id,
            run_id = %result.run.run_id,
            dry_run = result.run.dry_run,
            candidates = result.run.candidate_count,
            policy_decision_refs = result.run.policy_decision_ids.len(),
            audit_event_ids = result.run.audit_event_ids.len(),
            "skill curation run recorded through local governance strategy"
        );
        Ok(result)
    }

    /// Record a bounded curation snapshot ref and return replayable memento ids.
    pub(crate) async fn curation_snapshot(
        &self,
        command: SkillCurationSnapshotCommand,
    ) -> SkillCurationSnapshotResult {
        let captured_at = chrono::Utc::now();
        let governance = self
            .governance_snapshot(command.include_archived, command.lifecycle_filters)
            .await;
        let read_model = <Self as SkillGovernanceStoreStrategy>::read_model(self)
            .await
            .unwrap_or_else(|_| empty_read_model(captured_at));
        let snapshot = SkillGovernanceSnapshotRefRecord {
            snapshot_ref: format!(
                "store://skill-curation/{}/snapshot-{}",
                command.trace.trace_id,
                captured_at.timestamp_nanos_opt().unwrap_or_default()
            ),
            trace_id: command.trace.trace_id.clone(),
            record_count: governance.records.len() as u64,
            captured_at,
            report_ref: None,
        };
        let package_memento_refs = if command.include_package_mementos {
            read_model
                .rollback_refs
                .iter()
                .map(|rollback| rollback.rollback_ref.clone())
                .collect()
        } else {
            Vec::new()
        };
        let result = SkillCurationSnapshotResult {
            snapshot: snapshot.clone(),
            curation_run_refs: read_model
                .curation_runs
                .iter()
                .map(|run| run.run_id.clone())
                .collect(),
            rollback_refs: read_model.rollback_refs,
            package_memento_refs,
            mutated: false,
            captured_at,
        };
        let event = SkillGovernanceEventRecord::new(
            event_id("skill-governance-curation-snapshot", captured_at),
            &command.trace,
            command.scope,
            captured_at,
            SkillGovernanceEventPayload::SnapshotRefRecorded(snapshot),
        );
        let _ = <Self as SkillGovernanceStoreStrategy>::append_event(self, event).await;
        result
    }
}

fn empty_read_model(captured_at: chrono::DateTime<chrono::Utc>) -> SkillGovernanceReadModel {
    SkillGovernanceReadModel {
        records: Vec::new(),
        provenance_events: Vec::new(),
        aliases: Vec::new(),
        proposals: Vec::new(),
        curation_runs: Vec::new(),
        snapshot_refs: Vec::new(),
        rollback_refs: Vec::new(),
        replayed_events: 0,
        captured_at,
    }
}
