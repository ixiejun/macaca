//! In-memory provider for the Autonomy Evolution Control Plane contract.
//!
//! This provider is intentionally small and deterministic. It proves the
//! service-owned State machine, bounded read model, and structured logs without
//! introducing a production Store/EventLog dependency in the first slice. A
//! later provider can replace this Memento with durable replay while preserving
//! the same service contract.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use macaca_proto::{MacacaResult, ServiceHealth, TraceContext};
use tracing::{info, warn};

use crate::{
    autonomy_evolution_service_descriptor, validate_transition, AutonomyEvolutionService,
    DefaultEvolutionAdmissionSpecification, DefaultEvolutionBenchmarkScoringStrategy,
    DefaultEvolutionLiveOrchestrator, DefaultEvolutionReleaseSafetyStrategy,
    EvolutionAdmissionCommand, EvolutionAdmissionResult, EvolutionAdmissionSpecification,
    EvolutionBenchmarkCommand, EvolutionBenchmarkResult, EvolutionBenchmarkScoringStrategy,
    EvolutionGovernanceLedgerRecord, EvolutionGovernanceLedgerRecordKind,
    EvolutionLiveAuditCommand, EvolutionLiveAuditResult, EvolutionLiveOrchestrator,
    EvolutionLiveTickCommand, EvolutionLiveTickResult, EvolutionReleaseCommand,
    EvolutionReleaseResult, EvolutionReleaseSafetyStrategy, EvolutionRunRecord,
    EvolutionServiceSnapshot, EvolutionSnapshotCommand, EvolutionTransitionCommand,
    EvolutionTransitionResult, AUTONOMY_EVOLUTION_SERVICE_ID,
};

#[derive(Debug, Default, Clone)]
pub struct InMemoryAutonomyEvolutionProvider {
    records: Arc<Mutex<BTreeMap<String, EvolutionRunRecord>>>,
    live_checkpoints: Arc<Mutex<BTreeMap<String, EvolutionLiveTickResult>>>,
    live_ledger: Arc<Mutex<Vec<EvolutionGovernanceLedgerRecord>>>,
    live_sequence: Arc<Mutex<u64>>,
}

#[async_trait]
impl AutonomyEvolutionService for InMemoryAutonomyEvolutionProvider {
    fn descriptor(&self) -> macaca_proto::ServiceDescriptor {
        autonomy_evolution_service_descriptor(ServiceHealth::Healthy)
    }

    async fn health(&self, _trace: TraceContext) -> MacacaResult<EvolutionServiceSnapshot> {
        self.snapshot(EvolutionSnapshotCommand {
            trace: TraceContext::new("autonomy-evolution-health"),
            scope: Default::default(),
            run_id: None,
        })
        .await
    }

    async fn snapshot(
        &self,
        command: EvolutionSnapshotCommand,
    ) -> MacacaResult<EvolutionServiceSnapshot> {
        info!(
            service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
            trace_id = command.trace.trace_id.as_str(),
            "autonomy evolution snapshot requested"
        );
        let records = self
            .records
            .lock()
            .expect("evolution records mutex poisoned");
        let filtered = records
            .values()
            .filter(|record| {
                command
                    .run_id
                    .as_ref()
                    .map(|run_id| &record.run_id == run_id)
                    .unwrap_or(true)
                    && record.scope.matches_filter(&command.scope)
            })
            .cloned()
            .collect();
        Ok(EvolutionServiceSnapshot {
            service_id: AUTONOMY_EVOLUTION_SERVICE_ID.into(),
            healthy: true,
            unavailable_reason: None,
            records: filtered,
            captured_at: Utc::now(),
        })
    }

    async fn transition(
        &self,
        command: EvolutionTransitionCommand,
    ) -> MacacaResult<EvolutionTransitionResult> {
        info!(
            service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
            run_id = command.run_id.as_str(),
            trace_id = command.trace.trace_id.as_str(),
            transition = ?command.transition,
            "autonomy evolution transition requested"
        );

        let mut records = self
            .records
            .lock()
            .expect("evolution records mutex poisoned");
        let current = records.get(&command.run_id).map(|record| &record.state);
        let decision = match validate_transition(current, &command) {
            Ok(decision) => decision,
            Err(error) => {
                let current_state = current
                    .cloned()
                    .or_else(|| command.from_state.clone())
                    .unwrap_or(crate::EvolutionRunState::Observed);
                warn!(
                    service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
                    run_id = command.run_id.as_str(),
                    trace_id = command.trace.trace_id.as_str(),
                    reason = %error,
                    "autonomy evolution transition denied"
                );
                return Ok(EvolutionTransitionResult::denied(
                    &command,
                    current_state,
                    error.to_string(),
                ));
            }
        };

        let previous_state = current.cloned();
        let record = EvolutionRunRecord {
            run_id: command.run_id.clone(),
            state: decision.next_state.clone(),
            target_type: command.target_type.clone(),
            scope: command.scope.clone(),
            last_trace_id: command.trace.trace_id.clone(),
            evidence_count: command.evidence_refs.len(),
            audit_count: command.audit_refs.len(),
            rollback_count: command.rollback_refs.len(),
            updated_at: Utc::now(),
        };
        records.insert(command.run_id.clone(), record);

        if decision.adapter_dispatch_required {
            info!(
                service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
                run_id = command.run_id.as_str(),
                trace_id = command.trace.trace_id.as_str(),
                target_type = ?command.target_type,
                "autonomy evolution target adapter dispatch required"
            );
        }
        info!(
            service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
            run_id = command.run_id.as_str(),
            trace_id = command.trace.trace_id.as_str(),
            next_state = ?decision.next_state,
            "autonomy evolution transition accepted"
        );

        Ok(EvolutionTransitionResult::accepted(
            &command,
            previous_state,
            decision.next_state,
            decision.adapter_dispatch_required,
        ))
    }

    async fn admit_candidate(
        &self,
        command: EvolutionAdmissionCommand,
    ) -> MacacaResult<EvolutionAdmissionResult> {
        info!(
            service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
            candidate_id = command.candidate.candidate_id.as_str(),
            trace_id = command.trace.trace_id.as_str(),
            "autonomy evolution admission command received"
        );
        DefaultEvolutionAdmissionSpecification.evaluate(&command)
    }

    async fn run_paired_benchmark(
        &self,
        command: EvolutionBenchmarkCommand,
    ) -> MacacaResult<EvolutionBenchmarkResult> {
        info!(
            service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
            benchmark_id = command.benchmark_id.as_str(),
            run_id = command.run_id.as_str(),
            trace_id = command.trace.trace_id.as_str(),
            "autonomy evolution paired benchmark command received"
        );
        DefaultEvolutionBenchmarkScoringStrategy.score(&command)
    }

    async fn evaluate_release(
        &self,
        command: EvolutionReleaseCommand,
    ) -> MacacaResult<EvolutionReleaseResult> {
        info!(
            service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
            release_id = command.release_id.as_str(),
            run_id = command.run_id.as_str(),
            trace_id = command.trace.trace_id.as_str(),
            action = ?command.action,
            "autonomy evolution release safety command received"
        );
        DefaultEvolutionReleaseSafetyStrategy.evaluate(&command)
    }

    async fn run_live_tick(
        &self,
        command: EvolutionLiveTickCommand,
    ) -> MacacaResult<EvolutionLiveTickResult> {
        info!(
            service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
            run_id = command.run_id.as_str(),
            lease_id = command.lease_id.as_str(),
            idempotency_key = command.idempotency_key.as_str(),
            trace_id = command.trace.trace_id.as_str(),
            "autonomy evolution live tick command received"
        );
        if let Some(previous) = self
            .live_checkpoints
            .lock()
            .expect("live checkpoints mutex poisoned")
            .get(&command.idempotency_key)
            .cloned()
        {
            let mut duplicate = previous;
            duplicate.adapter_dispatch_performed = false;
            if !duplicate
                .reason_codes
                .iter()
                .any(|reason| reason == "duplicate_idempotency_key")
            {
                duplicate
                    .reason_codes
                    .push("duplicate_idempotency_key".into());
            }
            warn!(
                service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
                run_id = command.run_id.as_str(),
                idempotency_key = command.idempotency_key.as_str(),
                trace_id = command.trace.trace_id.as_str(),
                "autonomy evolution live tick returned existing idempotent checkpoint"
            );
            return Ok(duplicate);
        }

        let sequence = {
            let mut sequence = self
                .live_sequence
                .lock()
                .expect("live sequence mutex poisoned");
            *sequence += 1;
            *sequence
        };
        let result = match DefaultEvolutionLiveOrchestrator.run_tick(command.clone(), sequence) {
            Ok(result) => result,
            Err(error) => EvolutionLiveTickResult::denied(
                &command,
                sequence,
                crate::EvolutionLivePhaseStatus::Denied,
                error.to_string(),
            ),
        };
        self.live_checkpoints
            .lock()
            .expect("live checkpoints mutex poisoned")
            .insert(command.idempotency_key.clone(), result.clone());
        self.live_ledger
            .lock()
            .expect("live ledger mutex poisoned")
            .push(live_ledger_record(&command, &result));
        Ok(result)
    }

    async fn audit_live_run(
        &self,
        command: EvolutionLiveAuditCommand,
    ) -> MacacaResult<EvolutionLiveAuditResult> {
        info!(
            service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
            run_id = command.run_id.as_str(),
            trace_id = command.trace.trace_id.as_str(),
            "autonomy evolution live audit command received"
        );
        let limit = command.limit.min(100);
        let live_keys = self
            .live_ledger
            .lock()
            .expect("live ledger mutex poisoned")
            .iter()
            .filter(|record| {
                record.run_id == command.run_id
                    && command
                        .after_sequence
                        .map(|sequence| record.sequence > sequence)
                        .unwrap_or(true)
            })
            .map(|record| record.record_ref.clone())
            .collect::<Vec<_>>();
        let checkpoint_map = self
            .live_checkpoints
            .lock()
            .expect("live checkpoints mutex poisoned");
        let mut checkpoints = live_keys
            .into_iter()
            .filter_map(|key| checkpoint_map.get(&key).cloned())
            .collect::<Vec<_>>();
        checkpoints.sort_by_key(|checkpoint| checkpoint.sequence);
        checkpoints.truncate(limit);
        let next_after_sequence = checkpoints.last().map(|checkpoint| checkpoint.sequence);
        Ok(EvolutionLiveAuditResult {
            service_id: AUTONOMY_EVOLUTION_SERVICE_ID.into(),
            run_id: command.run_id,
            trace: command.trace,
            checkpoints,
            next_after_sequence,
            diagnostics: Vec::new(),
            captured_at: Utc::now(),
        })
    }
}

fn live_ledger_record(
    command: &EvolutionLiveTickCommand,
    result: &EvolutionLiveTickResult,
) -> EvolutionGovernanceLedgerRecord {
    EvolutionGovernanceLedgerRecord {
        schema_version: 1,
        sequence: result.sequence,
        record_id: format!("live-checkpoint-{}", result.sequence),
        run_id: result.run_id.clone(),
        node_id: command.lease_id.clone(),
        kind: EvolutionGovernanceLedgerRecordKind::Other("live_orchestrator_checkpoint".into()),
        target_type: Some(result.target_type.clone()),
        trace_id: result.trace.trace_id.clone(),
        scope: command.scope.clone(),
        record_ref: result.idempotency_key.clone(),
        evidence_refs: result.evidence_refs.clone(),
        policy_decision_refs: result.policy_decision_refs.clone(),
        audit_refs: result.audit_refs.clone(),
        rollback_refs: result.rollback_refs.clone(),
        captured_at: result.captured_at,
    }
}
