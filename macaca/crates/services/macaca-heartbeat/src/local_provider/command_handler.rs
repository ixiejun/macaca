//! Service command handler for the local Heartbeat provider.
//!
//! This module is the Adapter layer between the `HeartbeatService` trait and
//! the provider memento/gate internals. It keeps command handling separate from
//! cadence setup so the local provider remains small and reviewable.

use async_trait::async_trait;
use chrono::Utc;
use macaca_proto::{
    AutonomyAuditCorrelation, AutonomyServiceErrorKind, AutonomyStructuredError,
    HeartbeatCadencePolicy, HeartbeatCancelWakeCommand, HeartbeatCommandResult,
    HeartbeatCompleteRunCommand, HeartbeatProfileMutationResult, HeartbeatQueryCommand,
    HeartbeatRunState, HeartbeatRunSummary, HeartbeatServiceSnapshot,
    HeartbeatUpdateProfileCommand, HeartbeatWakeCommand, HeartbeatWakeDisposition, MacacaResult,
    ServiceDescriptor, TraceContext, HEARTBEAT_SERVICE_ID,
};
use tracing::{info, warn};

use super::{memento::StoredHeartbeatRun, InProcessHeartbeatProvider, LOCAL_PROVIDER_ID};
use crate::service_contract::HeartbeatService;

#[async_trait]
impl HeartbeatService for InProcessHeartbeatProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn health(&self, trace: TraceContext) -> MacacaResult<HeartbeatServiceSnapshot> {
        info!(
            service_id = HEARTBEAT_SERVICE_ID,
            provider_id = LOCAL_PROVIDER_ID,
            trace_id = trace.trace_id.as_str(),
            "local heartbeat health requested"
        );
        Ok(self.snapshot_inner())
    }

    async fn snapshot(
        &self,
        command: HeartbeatQueryCommand,
    ) -> MacacaResult<HeartbeatServiceSnapshot> {
        info!(
            service_id = HEARTBEAT_SERVICE_ID,
            provider_id = LOCAL_PROVIDER_ID,
            trace_id = command.trace.trace_id.as_str(),
            "local heartbeat snapshot requested"
        );
        Ok(self.snapshot_inner())
    }

    async fn wake(&self, command: HeartbeatWakeCommand) -> MacacaResult<HeartbeatCommandResult> {
        let trace = command.trace.clone();
        Ok(self.store.write(|state| {
            if let Some(existing) = state.pending_by_scope.get(&command.wake_scope_key).cloned() {
                let Some(run) = state.runs.get_mut(&existing) else {
                    state.pending_by_scope.remove(&command.wake_scope_key);
                    return self.error_result(
                        trace,
                        AutonomyServiceErrorKind::Conflict,
                        "pending_run_missing",
                        "heartbeat pending run was missing",
                    );
                };
                run.summary.state = HeartbeatRunState::Coalesced;
                run.summary.disposition = HeartbeatWakeDisposition::Coalesced;
                run.summary.safe_status = "wake coalesced by local heartbeat provider".into();
                run.summary
                    .metadata
                    .insert("coalesced_trace_id".into(), trace.trace_id.clone());
                info!(
                    service_id = HEARTBEAT_SERVICE_ID,
                    provider_id = LOCAL_PROVIDER_ID,
                    run_id = existing.as_str(),
                    wake_scope_key = command.wake_scope_key.as_str(),
                    trace_id = trace.trace_id.as_str(),
                    "local heartbeat coalesced duplicate wake"
                );
                return self.result(
                    trace,
                    Some(existing),
                    Some(HeartbeatRunState::Coalesced),
                    HeartbeatWakeDisposition::Coalesced,
                    run.summary.gates.clone(),
                    true,
                    run.summary.audit_id.clone(),
                );
            }

            let gates = self.gates.evaluate(state, &command);
            state.last_gate_decisions = gates.clone();
            let allowed = gates.iter().all(|gate| gate.allowed);
            let run_id = state.next_run_id();
            let now = Utc::now();
            let audit_id = state.record_audit(if allowed {
                "wake.accepted"
            } else {
                "wake.gated"
            });
            let disposition = if allowed {
                HeartbeatWakeDisposition::Accepted
            } else {
                HeartbeatWakeDisposition::Gated
            };
            let run_state = if allowed {
                HeartbeatRunState::Requested
            } else {
                HeartbeatRunState::Gated
            };
            let summary = HeartbeatRunSummary {
                run_id: run_id.clone(),
                wake_scope_key: command.wake_scope_key.clone(),
                intent: command.intent.clone(),
                state: run_state.clone(),
                disposition: disposition.clone(),
                requested_at: now,
                started_at: None,
                finished_at: if allowed { None } else { Some(now) },
                gates: gates.clone(),
                trace_id: trace.trace_id.clone(),
                audit_id: Some(audit_id.clone()),
                safe_status: if allowed {
                    "wake accepted by local heartbeat provider".into()
                } else {
                    "wake gated by local heartbeat provider".into()
                },
                metadata: command.metadata.clone(),
            };
            if allowed {
                state
                    .pending_by_scope
                    .insert(command.wake_scope_key.clone(), run_id.clone());
                info!(
                    service_id = HEARTBEAT_SERVICE_ID,
                    provider_id = LOCAL_PROVIDER_ID,
                    run_id = run_id.as_str(),
                    audit_id = audit_id.as_str(),
                    wake_scope_key = command.wake_scope_key.as_str(),
                    trace_id = trace.trace_id.as_str(),
                    "local heartbeat wake accepted"
                );
            } else {
                warn!(
                    service_id = HEARTBEAT_SERVICE_ID,
                    provider_id = LOCAL_PROVIDER_ID,
                    run_id = run_id.as_str(),
                    audit_id = audit_id.as_str(),
                    wake_scope_key = command.wake_scope_key.as_str(),
                    trace_id = trace.trace_id.as_str(),
                    "local heartbeat wake gated before dispatch boundary"
                );
            }
            state.history.push_back(run_id.clone());
            state
                .runs
                .insert(run_id.clone(), StoredHeartbeatRun::new(summary));
            info!(
                service_id = HEARTBEAT_SERVICE_ID,
                provider_id = LOCAL_PROVIDER_ID,
                run_id = run_id.as_str(),
                audit_id = audit_id.as_str(),
                wake_scope_key = command.wake_scope_key.as_str(),
                accepted = allowed,
                trace_id = trace.trace_id.as_str(),
                "local heartbeat processed wake request"
            );
            self.result(
                trace,
                Some(run_id),
                Some(run_state),
                disposition,
                gates,
                allowed,
                Some(audit_id),
            )
        }))
    }

    async fn cancel_wake(
        &self,
        command: HeartbeatCancelWakeCommand,
    ) -> MacacaResult<HeartbeatCommandResult> {
        let trace = command.trace.clone();
        Ok(self.store.write(|state| {
            let audit_id = state.record_audit("wake.cancelled");
            let transition = {
                let Some(run) = state.runs.get_mut(&command.run_id) else {
                    return self.error_result(
                        trace,
                        AutonomyServiceErrorKind::InvalidRequest,
                        "run_not_found",
                        "heartbeat run was not found",
                    );
                };
                run.summary.state = HeartbeatRunState::Skipped;
                run.summary.disposition = HeartbeatWakeDisposition::Skipped;
                run.summary.audit_id = Some(audit_id.clone());
                run.summary.finished_at = Some(Utc::now());
                run.summary.safe_status = "wake cancelled by local heartbeat provider".into();
                run.summary
                    .metadata
                    .insert("cancel_reason_code".into(), command.reason_code.clone());
                (
                    run.summary.wake_scope_key.clone(),
                    run.summary.gates.clone(),
                )
            };
            state.pending_by_scope.remove(&transition.0);
            warn!(
                service_id = HEARTBEAT_SERVICE_ID,
                provider_id = LOCAL_PROVIDER_ID,
                run_id = command.run_id.as_str(),
                audit_id = audit_id.as_str(),
                trace_id = trace.trace_id.as_str(),
                "local heartbeat cancelled wake"
            );
            self.result(
                trace,
                Some(command.run_id),
                Some(HeartbeatRunState::Skipped),
                HeartbeatWakeDisposition::Skipped,
                transition.1,
                false,
                Some(audit_id),
            )
        }))
    }

    async fn complete_run(
        &self,
        command: HeartbeatCompleteRunCommand,
    ) -> MacacaResult<HeartbeatCommandResult> {
        self.complete_run(command)
    }

    async fn get_run(
        &self,
        command: HeartbeatQueryCommand,
    ) -> MacacaResult<HeartbeatCommandResult> {
        let trace = command.trace.clone();
        let Some(run_id) = command.run_id else {
            return Ok(self.error_result(
                trace,
                AutonomyServiceErrorKind::InvalidRequest,
                "missing_run_id",
                "heartbeat get_run requires run_id",
            ));
        };
        let result = self.store.read(|state| {
            state.runs.get(&run_id).map(|run| {
                self.result(
                    trace.clone(),
                    Some(run_id.clone()),
                    Some(run.summary.state.clone()),
                    run.summary.disposition.clone(),
                    run.summary.gates.clone(),
                    run.summary.disposition == HeartbeatWakeDisposition::Accepted,
                    run.summary.audit_id.clone(),
                )
            })
        });
        Ok(result.unwrap_or_else(|| {
            self.error_result(
                trace,
                AutonomyServiceErrorKind::InvalidRequest,
                "run_not_found",
                "heartbeat run was not found",
            )
        }))
    }

    async fn list_runs(
        &self,
        command: HeartbeatQueryCommand,
    ) -> MacacaResult<Vec<HeartbeatRunSummary>> {
        Ok(self.store.read(|state| {
            state
                .history
                .iter()
                .rev()
                .filter_map(|run_id| state.runs.get(run_id))
                .filter(|run| {
                    command
                        .wake_scope_key
                        .as_ref()
                        .map(|scope| &run.summary.wake_scope_key == scope)
                        .unwrap_or(true)
                })
                .take(command.limit.unwrap_or(100))
                .map(|run| run.summary.clone())
                .collect()
        }))
    }

    async fn update_profile(
        &self,
        command: HeartbeatUpdateProfileCommand,
    ) -> MacacaResult<HeartbeatProfileMutationResult> {
        let trace = command.trace.clone();
        Ok(self.store.write(|state| {
            let audit_id = state.record_audit("profile.updated");
            let Some(profile) = state.profiles.get_mut(&command.profile_id) else {
                let error = AutonomyAuditCorrelation::from_trace(trace.clone())
                    .ok()
                    .map(|correlation| AutonomyStructuredError {
                        kind: AutonomyServiceErrorKind::InvalidRequest,
                        reason_code: "profile_not_found".into(),
                        safe_message: "heartbeat profile was not found".into(),
                        correlation,
                        metadata: Default::default(),
                    });
                return HeartbeatProfileMutationResult {
                    accepted: false,
                    profile: None,
                    trace,
                    audit_id: Some(audit_id),
                    error,
                    metadata: Default::default(),
                };
            };

            if let Some(enabled) = command.enabled {
                profile.profile.enabled = enabled;
            }
            if let Some(interval_ms) = command.fixed_interval_ms {
                profile.profile.cadence = HeartbeatCadencePolicy::FixedInterval {
                    interval_ms,
                    anchor: Some(Utc::now()),
                };
            }
            if let Some(cooldown_ms) = command.cooldown_ms {
                profile.profile.cooldown_ms = Some(cooldown_ms);
            }
            for (key, value) in command.metadata {
                if value.is_empty() {
                    profile.profile.metadata.remove(&key);
                } else {
                    profile.profile.metadata.insert(key, value);
                }
            }
            profile.safe_status = "native heartbeat profile policy updated".into();

            info!(
                service_id = HEARTBEAT_SERVICE_ID,
                provider_id = LOCAL_PROVIDER_ID,
                profile_id = command.profile_id.as_str(),
                audit_id = audit_id.as_str(),
                reason_code = command.reason_code.as_str(),
                trace_id = trace.trace_id.as_str(),
                "local heartbeat native profile policy updated"
            );

            HeartbeatProfileMutationResult {
                accepted: true,
                profile: Some(profile.summary()),
                trace,
                audit_id: Some(audit_id),
                error: None,
                metadata: Default::default(),
            }
        }))
    }
}
