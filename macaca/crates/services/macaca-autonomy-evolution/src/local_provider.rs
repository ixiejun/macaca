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
    DefaultEvolutionAdmissionSpecification, EvolutionAdmissionCommand, EvolutionAdmissionResult,
    EvolutionAdmissionSpecification, EvolutionRunRecord, EvolutionServiceSnapshot,
    EvolutionSnapshotCommand, EvolutionTransitionCommand, EvolutionTransitionResult,
    AUTONOMY_EVOLUTION_SERVICE_ID,
};

#[derive(Debug, Default, Clone)]
pub struct InMemoryAutonomyEvolutionProvider {
    records: Arc<Mutex<BTreeMap<String, EvolutionRunRecord>>>,
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
}
