//! Read-model and telemetry helpers for Skill provider governance state.
//!
//! This module owns usage recording, filtered governance snapshots, lightweight
//! telemetry rollups, and bounded autonomous materialization operator evidence.
//! Mutations delegate to the shared append-only event bridge in `proposal_store`.

use macaca_skill::{
    SkillAutonomousMaterializationRunResult, SkillAutonomousMaterializationSnapshotCommand,
    SkillAutonomousMaterializationSnapshotResult, SkillAutonomousMaterializationStatusCounts,
    SkillGovernanceEventPayload, SkillGovernanceEventRecord, SkillGovernanceRecordUsageCommand,
    SkillGovernanceRecordUsageResult, SkillGovernanceSnapshotResult, SkillTelemetryAggregate,
};

use super::event_vocabulary::snapshot_includes_lifecycle;
use super::{event_id, SkillProviderGovernanceState};

impl SkillProviderGovernanceState {
    /// Record one sanitized usage observation and return the updated record.
    pub(crate) async fn record_usage(
        &self,
        command: SkillGovernanceRecordUsageCommand,
    ) -> SkillGovernanceRecordUsageResult {
        let observed_at = chrono::Utc::now();
        let observation = command.observation;
        let key = observation.key();
        let mut records = self.records.lock().await;
        let record = records
            .entry(key)
            .and_modify(|record| record.apply(&observation, observed_at))
            .or_insert_with(|| macaca_skill::SkillGovernanceRecord::from_observation(&observation, observed_at))
            .clone();
        drop(records);
        self.append_event(SkillGovernanceEventRecord::new(
            event_id("skill-governance-usage", observed_at),
            &command.trace,
            command.scope,
            observed_at,
            SkillGovernanceEventPayload::UsageRecorded(observation),
        ))
        .await;

        SkillGovernanceRecordUsageResult {
            record,
            captured_at: observed_at,
        }
    }

    /// Return a sorted, sanitized governance snapshot.
    pub(crate) async fn governance_snapshot(
        &self,
        include_archived: bool,
        lifecycle_filters: Vec<macaca_skill::SkillLifecycleState>,
    ) -> SkillGovernanceSnapshotResult {
        let all_records = self
            .records
            .lock()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        let filtered_out = all_records
            .iter()
            .filter(|record| {
                !snapshot_includes_lifecycle(
                    &record.lifecycle,
                    include_archived,
                    &lifecycle_filters,
                )
            })
            .count();
        let mut records: Vec<_> = all_records
            .into_iter()
            .filter(|record| {
                snapshot_includes_lifecycle(&record.lifecycle, include_archived, &lifecycle_filters)
            })
            .collect();
        records.sort_by(|left, right| left.provenance.skill_id.cmp(&right.provenance.skill_id));
        let telemetry_aggregate = SkillTelemetryAggregate::from_records(records.iter());
        tracing::info!(
            include_archived = include_archived,
            lifecycle_filters = lifecycle_filters.len(),
            records = records.len(),
            filtered_records = filtered_out,
            successful_tasks = telemetry_aggregate.successful_task_count,
            failed_tasks = telemetry_aggregate.failed_task_count,
            "skill governance snapshot built through local compatibility adapter"
        );

        SkillGovernanceSnapshotResult {
            records,
            telemetry_aggregate,
            captured_at: chrono::Utc::now(),
        }
    }

    /// Return a bounded telemetry rollup for lightweight status surfaces.
    pub(crate) async fn telemetry_aggregate(&self) -> SkillTelemetryAggregate {
        let records = self
            .records
            .lock()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        SkillTelemetryAggregate::from_records(records.iter())
    }

    /// Persist a bounded, body-free Memento for the most recent operator runs.
    ///
    /// This does not make the in-memory provider durable. It gives SDK/Web
    /// surfaces a service-owned evidence source for the current process while
    /// preserving the future Store/EventLog migration point: a durable provider
    /// can replace this vector with append-only event persistence without
    /// changing the command/result contract.
    pub(crate) async fn record_materialization_operator_run(
        &self,
        result: SkillAutonomousMaterializationRunResult,
    ) {
        const MAX_OPERATOR_MEMENTOS: usize = 20;

        let mut runs = self.materialization_operator_runs.lock().await;
        runs.push(result);
        if runs.len() > MAX_OPERATOR_MEMENTOS {
            let overflow = runs.len() - MAX_OPERATOR_MEMENTOS;
            runs.drain(0..overflow);
        }
    }

    /// Return recent autonomous materialization operator evidence.
    ///
    /// Shells use this read model to display P3 materialization attempts without
    /// recomputing lifecycle semantics or reading package files directly.
    pub(crate) async fn materialization_operator_snapshot(
        &self,
        command: SkillAutonomousMaterializationSnapshotCommand,
    ) -> SkillAutonomousMaterializationSnapshotResult {
        let limit = if command.limit == 0 {
            20usize
        } else {
            command.limit.min(20) as usize
        };
        let runs = self.materialization_operator_runs.lock().await;
        let recent_runs = runs.iter().rev().take(limit).cloned().collect::<Vec<_>>();
        let status_counts = SkillAutonomousMaterializationStatusCounts::from_runs(&recent_runs);
        let last_run_ref = recent_runs.first().map(|run| run.run_id.clone());

        tracing::info!(
            trace_id = %command.trace.trace_id,
            recent_runs = recent_runs.len(),
            applied = status_counts.applied,
            previewed = status_counts.previewed,
            denied = status_counts.denied,
            "skill autonomous materialization operator snapshot emitted"
        );

        SkillAutonomousMaterializationSnapshotResult {
            recent_runs,
            status_counts,
            last_run_ref,
            captured_at: chrono::Utc::now(),
        }
    }
}
