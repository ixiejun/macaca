//! In-memory local Heartbeat provider.
//!
//! The local provider coordinates wake requests, coalescing, gate evaluation,
//! and bounded run history.  It deliberately does not execute tasks, send
//! notifications, call application workflows, or inspect business payloads.
//! Future dispatch can be layered behind service-runtime decorators after
//! policy, resource, entitlement, trace, and audit boundaries are installed.

mod command_handler;
mod gates;
mod memento;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use chrono::{DateTime, Duration, Utc};
use macaca_proto::{
    AutonomyAuditCorrelation, AutonomyScope, AutonomyServiceErrorKind, AutonomyStructuredError,
    HeartbeatCadencePolicy, HeartbeatCommandResult, HeartbeatGateDecision, HeartbeatProfile,
    HeartbeatProfileId, HeartbeatRunId, HeartbeatRunState, HeartbeatScopeIdentity,
    HeartbeatServiceSnapshot, HeartbeatWakeCommand, HeartbeatWakeDisposition, HeartbeatWakeIntent,
    MacacaError, MacacaResult, ServiceDescriptor, ServiceHealth, ServiceLifecycleState,
    TraceContext, HEARTBEAT_SERVICE_ID,
};
use tracing::info;

use crate::service_contract::HeartbeatService;
use gates::DefaultHeartbeatGateStrategy;
use memento::{InMemoryHeartbeatStore, StoredHeartbeatProfile, StoredHeartbeatRun};

const LOCAL_PROVIDER_ID: &str = "local.in_memory";
const DEFAULT_COOLDOWN_MS: i64 = 30_000;
const DEFAULT_ACTIVE_START_HOUR_UTC: u32 = 0;
const DEFAULT_ACTIVE_END_HOUR_UTC: u32 = 24;
const DEFAULT_NATIVE_PROFILE_ID: &str = "profile.system.autonomy";
const DEFAULT_NATIVE_SCOPE_KEY: &str = "system.autonomy";

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
        let store = InMemoryHeartbeatStore::default();
        store.write(|state| {
            let profile = HeartbeatProfile::new(
                HeartbeatProfileId::new(DEFAULT_NATIVE_PROFILE_ID)
                    .expect("default heartbeat profile id is non-empty"),
                HeartbeatScopeIdentity::new(AutonomyScope::global(), DEFAULT_NATIVE_SCOPE_KEY)
                    .expect("default heartbeat scope key is non-empty"),
                HeartbeatCadencePolicy::FixedInterval {
                    interval_ms: DEFAULT_COOLDOWN_MS as u64,
                    anchor: Some(Utc::now() - Duration::milliseconds(DEFAULT_COOLDOWN_MS)),
                },
            )
            .expect("default heartbeat profile is valid");
            state.profiles.insert(
                profile.profile_id.clone(),
                StoredHeartbeatProfile::new(profile),
            );
        });
        Self {
            descriptor,
            store,
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
                native_profiles_active: state
                    .profiles
                    .values()
                    .filter(|profile| profile.profile.enabled)
                    .count(),
                native_profiles: state
                    .profiles
                    .values()
                    .map(StoredHeartbeatProfile::summary)
                    .collect(),
                recent_runs,
                last_gate_decisions: state.last_gate_decisions.clone(),
                last_audit_ids: state.audit_ids.iter().rev().take(25).cloned().collect(),
                captured_at: Utc::now(),
            }
        })
    }

    /// Register or replace a native heartbeat profile.
    ///
    /// This is the Heartbeat service's own cadence API. It intentionally does
    /// not create a Scheduler job, so agent and system heartbeat liveness can
    /// continue independently of Scheduler due-run materialization.
    pub fn register_native_profile(&self, profile: HeartbeatProfile) -> MacacaResult<()> {
        profile.cadence.validate()?;
        let profile_id = profile.profile_id.clone();
        let scope_key = profile.scope_identity.scope_key.clone();
        self.store.write(|state| {
            state
                .profiles
                .insert(profile_id.clone(), StoredHeartbeatProfile::new(profile));
        });
        info!(
            service_id = HEARTBEAT_SERVICE_ID,
            provider_id = LOCAL_PROVIDER_ID,
            profile_id = profile_id.as_str(),
            scope_key = scope_key.as_str(),
            "local heartbeat native profile registered"
        );
        Ok(())
    }

    /// Evaluate native heartbeat profiles once and process due profiles.
    ///
    /// The method is a deterministic single-tick entrypoint used by
    /// runtime-host HeartbeatLane and tests. It records mementos through the
    /// normal wake path and dispatches no application-specific behavior.
    pub async fn tick_native_profiles_once(
        &self,
        trace: TraceContext,
    ) -> MacacaResult<Vec<HeartbeatCommandResult>> {
        let due_profiles = self.store.write(|state| {
            let now = Utc::now();
            state
                .profiles
                .values_mut()
                .filter_map(|profile| profile.due_at(now).map(|_| profile.profile.clone()))
                .collect::<Vec<_>>()
        });
        let mut results = Vec::new();
        for profile in due_profiles {
            let profile_id = profile.profile_id.clone();
            let wake = HeartbeatWakeCommand::new(
                trace.clone(),
                profile.scope_identity.scope.clone(),
                profile.scope_identity.scope_key.clone(),
                HeartbeatWakeIntent::NativeCadence {
                    profile_id: profile_id.clone(),
                },
            )?;
            let result = self.wake(wake).await?;
            let mut result = result;
            if let Some(app_id) = profile.scope_identity.scope.application_id {
                result
                    .metadata
                    .insert("application_id".into(), app_id.to_string());
            }
            if let Some(session_id) = profile.scope_identity.scope.session_id.as_ref() {
                result
                    .metadata
                    .insert("session_id".into(), session_id.clone());
            }
            result
                .metadata
                .insert("scope_key".into(), profile.scope_identity.scope_key.clone());
            if let Some(run_id) = result.run_id.clone() {
                self.store.write(|state| {
                    if let Some(stored) = state.profiles.get_mut(&profile_id) {
                        stored.last_tick_at = Some(Utc::now());
                        stored.last_run_id = Some(run_id.clone());
                        stored.safe_status = if result.accepted {
                            "native heartbeat cadence accepted".into()
                        } else {
                            "native heartbeat cadence gated".into()
                        };
                    }
                });
                if result.accepted {
                    self.mark_run_running(trace.clone(), run_id.clone())?;
                    self.mark_run_succeeded(trace.clone(), run_id)?;
                }
            }
            info!(
                service_id = HEARTBEAT_SERVICE_ID,
                provider_id = LOCAL_PROVIDER_ID,
                profile_id = profile_id.as_str(),
                accepted = result.accepted,
                trace_id = trace.trace_id.as_str(),
                "local heartbeat native profile tick processed"
            );
            results.push(result);
        }
        Ok(results)
    }

    fn result(
        &self,
        trace: TraceContext,
        run_id: Option<HeartbeatRunId>,
        state: Option<HeartbeatRunState>,
        disposition: HeartbeatWakeDisposition,
        gates: Vec<HeartbeatGateDecision>,
        accepted: bool,
        audit_id: Option<String>,
    ) -> HeartbeatCommandResult {
        HeartbeatCommandResult {
            run_id,
            state,
            disposition,
            gates,
            accepted,
            error: None,
            trace,
            audit_id,
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
        self.transition_run(
            trace,
            run_id,
            HeartbeatRunState::Running,
            "running",
            |run, now| {
                run.summary.started_at = Some(now);
                run.summary.safe_status = "heartbeat dispatch boundary entered".into();
            },
        )
    }

    /// Mark a heartbeat run as completed successfully.
    pub fn mark_run_succeeded(
        &self,
        trace: TraceContext,
        run_id: HeartbeatRunId,
    ) -> MacacaResult<HeartbeatCommandResult> {
        self.transition_run(
            trace,
            run_id,
            HeartbeatRunState::Succeeded,
            "succeeded",
            |run, now| {
                run.summary.finished_at = Some(now);
                run.summary.safe_status = "heartbeat run completed".into();
            },
        )
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
        self.transition_run(
            trace,
            run_id,
            HeartbeatRunState::Failed,
            "failed",
            |run, now| {
                run.summary.finished_at = Some(now);
                run.summary.safe_status = "heartbeat run failed".into();
                run.summary
                    .metadata
                    .insert("failure_reason_code".into(), reason_code.clone());
            },
        )
    }

    /// Mark a heartbeat run as intentionally skipped.
    pub fn mark_run_skipped(
        &self,
        trace: TraceContext,
        run_id: HeartbeatRunId,
        reason_code: impl Into<String>,
    ) -> MacacaResult<HeartbeatCommandResult> {
        let reason_code = normalize_label(reason_code.into(), "heartbeat skip reason is required")?;
        self.transition_run(
            trace,
            run_id,
            HeartbeatRunState::Skipped,
            "skipped",
            |run, now| {
                run.summary.finished_at = Some(now);
                run.summary.disposition = HeartbeatWakeDisposition::Skipped;
                run.summary.safe_status = "heartbeat run skipped".into();
                run.summary
                    .metadata
                    .insert("skip_reason_code".into(), reason_code.clone());
            },
        )
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
            let audit_id = state.record_audit(match next_state {
                HeartbeatRunState::Running => "run.running",
                HeartbeatRunState::Succeeded => "run.succeeded",
                HeartbeatRunState::Failed => "run.failed",
                HeartbeatRunState::Skipped => "run.skipped",
                HeartbeatRunState::Requested => "run.requested",
                HeartbeatRunState::Coalesced => "run.coalesced",
                HeartbeatRunState::Gated => "run.gated",
            });
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
                run.summary.audit_id = Some(audit_id.clone());
                mutate(run, now);
                (
                    run.summary.wake_scope_key.clone(),
                    run.summary.disposition.clone(),
                    run.summary.gates.clone(),
                )
            };
            if matches!(
                next_state,
                HeartbeatRunState::Succeeded
                    | HeartbeatRunState::Failed
                    | HeartbeatRunState::Skipped
            ) {
                state.pending_by_scope.remove(&transition.0);
            }
            info!(
                service_id = HEARTBEAT_SERVICE_ID,
                provider_id = LOCAL_PROVIDER_ID,
                run_id = run_id.as_str(),
                action,
                audit_id = audit_id.as_str(),
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
                Some(audit_id),
            )
        }))
    }
}

impl Default for LocalHeartbeatProvider {
    fn default() -> Self {
        Self::new()
    }
}

fn normalize_label(value: String, message: &'static str) -> MacacaResult<String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(MacacaError::Config(message.into()));
    }
    Ok(value)
}
