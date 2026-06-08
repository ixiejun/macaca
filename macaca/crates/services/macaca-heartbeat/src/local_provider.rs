//! In-process Heartbeat provider (in-memory memento engine).
//!
//! `InProcessHeartbeatProvider` is the built-in Heartbeat service implementation
//! registered by runtime-host when local autonomy mode is selected.  The type
//! name uses the in-process prefix so escape-hatch gates can freeze the retired
//! local-provider identifier while approved composition roots keep a neutral label.
//!
//! The local provider coordinates wake requests, coalescing, gate evaluation,
//! and bounded run history.  It deliberately does not execute tasks, send
//! notifications, call application workflows, or inspect business payloads.
//! Future dispatch can be layered behind service-runtime decorators after
//! policy, resource, entitlement, trace, and audit boundaries are installed.

mod command_handler;
mod completion_sanitizer;
mod gates;
mod memento;
mod native_cadence;
mod run_lifecycle;
#[cfg(test)]
mod tests;

use chrono::{Duration, Utc};
use macaca_proto::{
    AutonomyScope, HeartbeatCadencePolicy, HeartbeatProfile, HeartbeatProfileId,
    HeartbeatRunState, HeartbeatScopeIdentity, HeartbeatServiceSnapshot, MacacaResult,
    ServiceDescriptor, ServiceHealth, ServiceLifecycleState, HEARTBEAT_SERVICE_ID,
};
use tracing::info;

use crate::service_contract::HeartbeatService;
use gates::DefaultHeartbeatGateStrategy;
use memento::{InMemoryHeartbeatStore, StoredHeartbeatProfile};

pub(super) const LOCAL_PROVIDER_ID: &str = "local.in_memory";
const DEFAULT_COOLDOWN_MS: i64 = 30_000;
pub(super) const PROFILE_COOLDOWN_MS_KEY: &str = "heartbeat.profile.cooldown_ms";
pub(super) const PROFILE_ID_METADATA_KEY: &str = "heartbeat.profile_id";
const DEFAULT_ACTIVE_START_HOUR_UTC: u32 = 0;
const DEFAULT_ACTIVE_END_HOUR_UTC: u32 = 24;
const DEFAULT_NATIVE_PROFILE_ID: &str = "profile.system.autonomy";
const DEFAULT_NATIVE_SCOPE_KEY: &str = "system.autonomy";

/// In-process Heartbeat provider backed by an in-memory memento store.
///
/// The provider uses State for run lifecycle, Memento for snapshots/history,
/// Strategy for gate evaluation, and Observer-style tracing for key execution
/// nodes.  The gate strategy is conservative and generic; it never branches on
/// application, workflow, driver, provider, model, chain, payment, or business
/// names.
pub struct InProcessHeartbeatProvider {
    descriptor: ServiceDescriptor,
    store: InMemoryHeartbeatStore,
    gates: DefaultHeartbeatGateStrategy,
}

impl InProcessHeartbeatProvider {
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

    pub(super) fn snapshot_inner(&self) -> HeartbeatServiceSnapshot {
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

}

impl Default for InProcessHeartbeatProvider {
    fn default() -> Self {
        Self::new()
    }
}
