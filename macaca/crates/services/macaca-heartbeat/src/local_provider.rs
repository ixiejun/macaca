//! In-memory local Heartbeat provider.
//!
//! The local provider coordinates wake requests, coalescing, gate evaluation,
//! and bounded run history.  It deliberately does not execute tasks, send
//! notifications, call application workflows, or inspect business payloads.
//! Future dispatch can be layered behind service-runtime decorators after
//! policy, resource, entitlement, trace, and audit boundaries are installed.

use std::collections::{BTreeMap, VecDeque};
use std::sync::RwLock;

use async_trait::async_trait;
use chrono::{DateTime, Duration, Timelike, Utc};
use macaca_proto::{
    AutonomyAuditCorrelation, AutonomyServiceErrorKind, AutonomyStructuredError,
    HeartbeatCancelWakeCommand, HeartbeatCommandResult, HeartbeatGateDecision,
    HeartbeatGateKind, HeartbeatQueryCommand, HeartbeatRunId, HeartbeatRunState,
    HeartbeatRunSummary, HeartbeatServiceSnapshot, HeartbeatWakeCommand,
    HeartbeatWakeDisposition, MacacaError, MacacaResult, ServiceDescriptor, ServiceHealth,
    ServiceLifecycleState, TraceContext, HEARTBEAT_SERVICE_ID,
};
use tracing::{info, warn};

use crate::service_contract::HeartbeatService;

const LOCAL_PROVIDER_ID: &str = "local.in_memory";
const DEFAULT_COOLDOWN_MS: i64 = 30_000;
const DEFAULT_ACTIVE_START_HOUR_UTC: u32 = 0;
const DEFAULT_ACTIVE_END_HOUR_UTC: u32 = 24;
const RESOURCE_UNITS_KEY: &str = "resource_units";
const BUDGET_UNITS_KEY: &str = "budget_units";
const DEFAULT_RESOURCE_CAPACITY_UNITS: u64 = 1_000;
const DEFAULT_BUDGET_CAPACITY_UNITS: u64 = 1_000;

/// Local Heartbeat provider backed by an in-memory memento store.
///
/// The provider uses State for run lifecycle, Memento for snapshots/history,
/// Strategy for gate evaluation, and Observer-style tracing for key execution
/// nodes.  The gate strategy is conservative and generic; it never branches on
/// application, workflow, driver, provider, model, chain, payment, or business
/// names.
pub struct LocalHeartbeatProvider {
    descriptor: ServiceDescriptor,
    store: InMemoryHeartbeatStore,
    gates: DefaultHeartbeatGateStrategy,
}

impl LocalHeartbeatProvider {
    /// Create an empty local provider with the standard Heartbeat descriptor.
    pub fn new() -> Self {
        let unavailable = crate::UnavailableHeartbeatProvider::default();
        let mut descriptor = unavailable.descriptor();
        descriptor.health = ServiceHealth::Healthy;
        descriptor.lifecycle_state = ServiceLifecycleState::Running;
        descriptor
            .metadata
            .insert("provider_id".into(), LOCAL_PROVIDER_ID.into());
        Self {
            descriptor,
            store: InMemoryHeartbeatStore::default(),
            gates: DefaultHeartbeatGateStrategy::default(),
        }
    }

    fn snapshot_inner(&self) -> HeartbeatServiceSnapshot {
        self.store.read(|state| {
            let pending_wakes = state
                .runs
                .values()
                .filter(|run| {
                    matches!(
                        run.summary.state,
                        HeartbeatRunState::Requested | HeartbeatRunState::Coalesced
                    )
                })
                .count();
            let active_runs = state
                .runs
                .values()
                .filter(|run| run.summary.state == HeartbeatRunState::Running)
                .count();
            let recent_runs = state
                .history
                .iter()
                .rev()
                .take(25)
                .filter_map(|run_id| state.runs.get(run_id).map(|run| run.summary.clone()))
                .collect();
            HeartbeatServiceSnapshot {
                service_id: HEARTBEAT_SERVICE_ID.into(),
                provider_id: LOCAL_PROVIDER_ID.into(),
                healthy: true,
                lifecycle_state: "running".into(),
                pending_wakes,
                active_runs,
                scheduler_ticks_active: state.scheduler_ticks_active,
                recent_runs,
                last_gate_decisions: state.last_gate_decisions.clone(),
                last_audit_ids: Vec::new(),
                captured_at: Utc::now(),
            }
        })
    }

    fn result(
        &self,
        trace: TraceContext,
        run_id: Option<HeartbeatRunId>,
        state: Option<HeartbeatRunState>,
        disposition: HeartbeatWakeDisposition,
        gates: Vec<HeartbeatGateDecision>,
        accepted: bool,
    ) -> HeartbeatCommandResult {
        HeartbeatCommandResult {
            run_id,
            state,
            disposition,
            gates,
            accepted,
            error: None,
            trace,
            audit_id: None,
            metadata: BTreeMap::new(),
        }
    }

    fn error_result(
        &self,
        trace: TraceContext,
        kind: AutonomyServiceErrorKind,
        reason_code: &'static str,
        safe_message: impl Into<String>,
    ) -> HeartbeatCommandResult {
        let correlation = AutonomyAuditCorrelation::from_trace(trace.clone()).ok();
        HeartbeatCommandResult {
            run_id: None,
            state: None,
            disposition: HeartbeatWakeDisposition::Skipped,
            gates: Vec::new(),
            accepted: false,
            error: correlation.map(|correlation| AutonomyStructuredError {
                kind,
                reason_code: reason_code.into(),
                safe_message: safe_message.into(),
                correlation,
                metadata: BTreeMap::new(),
            }),
            trace,
            audit_id: None,
            metadata: BTreeMap::new(),
        }
    }

    /// Mark an accepted wake as entering the future dispatch boundary.
    ///
    /// The method records lifecycle evidence only. It does not call task,
    /// application, plugin, notification, or external provider code.
    pub fn mark_run_running(
        &self,
        trace: TraceContext,
        run_id: HeartbeatRunId,
    ) -> MacacaResult<HeartbeatCommandResult> {
        self.transition_run(trace, run_id, HeartbeatRunState::Running, "running", |run, now| {
            run.summary.started_at = Some(now);
            run.summary.safe_status = "heartbeat dispatch boundary entered".into();
        })
    }

    /// Mark a heartbeat run as completed successfully.
    pub fn mark_run_succeeded(
        &self,
        trace: TraceContext,
        run_id: HeartbeatRunId,
    ) -> MacacaResult<HeartbeatCommandResult> {
        self.transition_run(trace, run_id, HeartbeatRunState::Succeeded, "succeeded", |run, now| {
            run.summary.finished_at = Some(now);
            run.summary.safe_status = "heartbeat run completed".into();
        })
    }

    /// Mark a heartbeat run as failed with a sanitized reason code.
    pub fn mark_run_failed(
        &self,
        trace: TraceContext,
        run_id: HeartbeatRunId,
        reason_code: impl Into<String>,
    ) -> MacacaResult<HeartbeatCommandResult> {
        let reason_code =
            normalize_label(reason_code.into(), "heartbeat failure reason is required")?;
        self.transition_run(trace, run_id, HeartbeatRunState::Failed, "failed", |run, now| {
            run.summary.finished_at = Some(now);
            run.summary.safe_status = "heartbeat run failed".into();
            run.summary
                .metadata
                .insert("failure_reason_code".into(), reason_code.clone());
        })
    }

    /// Mark a heartbeat run as intentionally skipped.
    pub fn mark_run_skipped(
        &self,
        trace: TraceContext,
        run_id: HeartbeatRunId,
        reason_code: impl Into<String>,
    ) -> MacacaResult<HeartbeatCommandResult> {
        let reason_code = normalize_label(reason_code.into(), "heartbeat skip reason is required")?;
        self.transition_run(trace, run_id, HeartbeatRunState::Skipped, "skipped", |run, now| {
            run.summary.finished_at = Some(now);
            run.summary.disposition = HeartbeatWakeDisposition::Skipped;
            run.summary.safe_status = "heartbeat run skipped".into();
            run.summary
                .metadata
                .insert("skip_reason_code".into(), reason_code.clone());
        })
    }

    fn transition_run<F>(
        &self,
        trace: TraceContext,
        run_id: HeartbeatRunId,
        next_state: HeartbeatRunState,
        action: &'static str,
        mutate: F,
    ) -> MacacaResult<HeartbeatCommandResult>
    where
        F: FnOnce(&mut StoredHeartbeatRun, DateTime<Utc>),
    {
        Ok(self.store.write(|state| {
            let transition = {
                let Some(run) = state.runs.get_mut(&run_id) else {
                    return self.error_result(
                        trace,
                        AutonomyServiceErrorKind::InvalidRequest,
                        "run_not_found",
                        "heartbeat run was not found",
                    );
                };
                let now = Utc::now();
                run.summary.state = next_state.clone();
                mutate(run, now);
                (
                    run.summary.wake_scope_key.clone(),
                    run.summary.disposition.clone(),
                    run.summary.gates.clone(),
                )
            };
            if matches!(
                next_state,
                HeartbeatRunState::Succeeded | HeartbeatRunState::Failed | HeartbeatRunState::Skipped
            ) {
                state.pending_by_scope.remove(&transition.0);
            }
            info!(
                service_id = HEARTBEAT_SERVICE_ID,
                provider_id = LOCAL_PROVIDER_ID,
                run_id = run_id.as_str(),
                action,
                trace_id = trace.trace_id.as_str(),
                "local heartbeat run lifecycle transition completed"
            );
            self.result(
                trace,
                Some(run_id),
                Some(next_state),
                transition.1,
                transition.2,
                false,
            )
        }))
    }
}

impl Default for LocalHeartbeatProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl HeartbeatService for LocalHeartbeatProvider {
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

    async fn snapshot(&self, command: HeartbeatQueryCommand) -> MacacaResult<HeartbeatServiceSnapshot> {
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
                );
            }

            let gates = self.gates.evaluate(state, &command);
            state.last_gate_decisions = gates.clone();
            let allowed = gates.iter().all(|gate| gate.allowed);
            let run_id = state.next_run_id();
            let now = Utc::now();
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
                audit_id: None,
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
                    wake_scope_key = command.wake_scope_key.as_str(),
                    trace_id = trace.trace_id.as_str(),
                    "local heartbeat wake accepted"
                );
            } else {
                warn!(
                    service_id = HEARTBEAT_SERVICE_ID,
                    provider_id = LOCAL_PROVIDER_ID,
                    run_id = run_id.as_str(),
                    wake_scope_key = command.wake_scope_key.as_str(),
                    trace_id = trace.trace_id.as_str(),
                    "local heartbeat wake gated before dispatch boundary"
                );
            }
            state.history.push_back(run_id.clone());
            state.runs.insert(run_id.clone(), StoredHeartbeatRun { summary });
            info!(
                service_id = HEARTBEAT_SERVICE_ID,
                provider_id = LOCAL_PROVIDER_ID,
                run_id = run_id.as_str(),
                wake_scope_key = command.wake_scope_key.as_str(),
                accepted = allowed,
                trace_id = trace.trace_id.as_str(),
                "local heartbeat processed wake request"
            );
            self.result(trace, Some(run_id), Some(run_state), disposition, gates, allowed)
        }))
    }

    async fn cancel_wake(
        &self,
        command: HeartbeatCancelWakeCommand,
    ) -> MacacaResult<HeartbeatCommandResult> {
        let trace = command.trace.clone();
        Ok(self.store.write(|state| {
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
            )
        }))
    }

    async fn get_run(&self, command: HeartbeatQueryCommand) -> MacacaResult<HeartbeatCommandResult> {
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
}

#[derive(Default)]
struct InMemoryHeartbeatStore {
    state: RwLock<LocalHeartbeatState>,
}

impl InMemoryHeartbeatStore {
    fn read<R>(&self, f: impl FnOnce(&LocalHeartbeatState) -> R) -> R {
        let guard = self
            .state
            .read()
            .expect("local heartbeat state lock poisoned");
        f(&guard)
    }

    fn write<R>(&self, f: impl FnOnce(&mut LocalHeartbeatState) -> R) -> R {
        let mut guard = self
            .state
            .write()
            .expect("local heartbeat state lock poisoned");
        f(&mut guard)
    }
}

#[derive(Default)]
struct LocalHeartbeatState {
    next_run_sequence: u64,
    runs: BTreeMap<HeartbeatRunId, StoredHeartbeatRun>,
    history: VecDeque<HeartbeatRunId>,
    pending_by_scope: BTreeMap<String, HeartbeatRunId>,
    last_gate_decisions: Vec<HeartbeatGateDecision>,
    scheduler_ticks_active: bool,
    last_accepted_by_scope: BTreeMap<String, DateTime<Utc>>,
}

impl LocalHeartbeatState {
    fn next_run_id(&mut self) -> HeartbeatRunId {
        self.next_run_sequence += 1;
        HeartbeatRunId::new(format!("heartbeat-run-{}", self.next_run_sequence))
            .expect("generated heartbeat run id is always non-empty")
    }
}

struct StoredHeartbeatRun {
    summary: HeartbeatRunSummary,
}

#[derive(Clone)]
struct DefaultHeartbeatGateStrategy {
    active_start_hour_utc: u32,
    active_end_hour_utc: u32,
    cooldown_ms: i64,
    resource_capacity_units: u64,
    budget_capacity_units: u64,
}

impl Default for DefaultHeartbeatGateStrategy {
    fn default() -> Self {
        Self {
            active_start_hour_utc: DEFAULT_ACTIVE_START_HOUR_UTC,
            active_end_hour_utc: DEFAULT_ACTIVE_END_HOUR_UTC,
            cooldown_ms: DEFAULT_COOLDOWN_MS,
            resource_capacity_units: DEFAULT_RESOURCE_CAPACITY_UNITS,
            budget_capacity_units: DEFAULT_BUDGET_CAPACITY_UNITS,
        }
    }
}

impl DefaultHeartbeatGateStrategy {
    fn evaluate(
        &self,
        state: &mut LocalHeartbeatState,
        command: &HeartbeatWakeCommand,
    ) -> Vec<HeartbeatGateDecision> {
        let now = Utc::now();
        let active_hours_allowed = self.in_active_hours(now);
        let cooldown_allowed = state
            .last_accepted_by_scope
            .get(&command.wake_scope_key)
            .map(|last| now.signed_duration_since(*last).num_milliseconds() >= self.cooldown_ms)
            .unwrap_or(true);
        let busy_allowed = !state.pending_by_scope.contains_key(&command.wake_scope_key);
        let provider_allowed = true;
        let resource_units = metadata_units(&command.metadata, RESOURCE_UNITS_KEY);
        let budget_units = metadata_units(&command.metadata, BUDGET_UNITS_KEY);
        let resource_allowed = resource_units <= self.resource_capacity_units;
        let budget_allowed = budget_units <= self.budget_capacity_units;
        let gates = vec![
            gate(
                HeartbeatGateKind::ActiveHours,
                active_hours_allowed,
                "active_hours",
                None,
            ),
            gate(
                HeartbeatGateKind::Cooldown,
                cooldown_allowed,
                "cooldown",
                if cooldown_allowed {
                    None
                } else {
                    state
                        .last_accepted_by_scope
                        .get(&command.wake_scope_key)
                        .map(|last| *last + Duration::milliseconds(self.cooldown_ms))
                },
            ),
            gate(HeartbeatGateKind::Busy, busy_allowed, "busy", None),
            gate(HeartbeatGateKind::Resource, resource_allowed, "resource", None),
            gate(HeartbeatGateKind::Budget, budget_allowed, "budget", None),
            gate(
                HeartbeatGateKind::ProviderHealth,
                provider_allowed,
                "provider_health",
                None,
            ),
        ];
        if gates.iter().all(|gate| gate.allowed) {
            state
                .last_accepted_by_scope
                .insert(command.wake_scope_key.clone(), now);
        }
        gates
    }

    fn in_active_hours(&self, now: DateTime<Utc>) -> bool {
        if self.active_start_hour_utc == self.active_end_hour_utc {
            return true;
        }
        let hour = now.hour();
        if self.active_start_hour_utc < self.active_end_hour_utc {
            hour >= self.active_start_hour_utc && hour < self.active_end_hour_utc
        } else {
            hour >= self.active_start_hour_utc || hour < self.active_end_hour_utc
        }
    }
}

fn gate(
    kind: HeartbeatGateKind,
    allowed: bool,
    reason_code: &'static str,
    next_eligible_at: Option<DateTime<Utc>>,
) -> HeartbeatGateDecision {
    HeartbeatGateDecision {
        gate: kind,
        allowed,
        reason_code: reason_code.into(),
        next_eligible_at,
        metadata: BTreeMap::new(),
    }
}

fn metadata_units(metadata: &BTreeMap<String, String>, key: &str) -> u64 {
    metadata
        .get(key)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0)
}

fn normalize_label(value: String, message: &'static str) -> MacacaResult<String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(MacacaError::Config(message.into()));
    }
    Ok(value)
}
