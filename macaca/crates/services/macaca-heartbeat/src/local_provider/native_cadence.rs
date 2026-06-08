//! Native Heartbeat cadence tick orchestration.
//!
//! Heartbeat profiles carry their own cadence policy independent of Scheduler
//! jobs. This module implements the single-tick **Command** entrypoint used by
//! runtime-host HeartbeatLane: discover due profiles, materialize wake commands,
//! invoke the normal wake/gate path, and record profile memento updates.
//!
//! No application names, workflow identifiers, or driver-specific branches
//! appear here; scope identity is entirely provider-neutral.

use chrono::Utc;
use macaca_proto::{
    HeartbeatCommandResult, HeartbeatWakeCommand, HeartbeatWakeIntent, MacacaResult,
    TraceContext, HEARTBEAT_SERVICE_ID,
};
use tracing::info;

use crate::service_contract::HeartbeatService;

use super::{
    InProcessHeartbeatProvider, LOCAL_PROVIDER_ID, PROFILE_COOLDOWN_MS_KEY,
    PROFILE_ID_METADATA_KEY,
};

impl InProcessHeartbeatProvider {
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
            let mut wake = HeartbeatWakeCommand::new(
                trace.clone(),
                profile.scope_identity.scope.clone(),
                profile.scope_identity.scope_key.clone(),
                HeartbeatWakeIntent::NativeCadence {
                    profile_id: profile_id.clone(),
                },
            )?;
            // Copy bounded profile metadata into the wake command before gate
            // evaluation. This is the only place where profile policy crosses
            // from Heartbeat memento state into a wake decision.
            wake.metadata = profile.metadata.clone();
            wake.metadata.insert(
                PROFILE_ID_METADATA_KEY.into(),
                profile.profile_id.as_str().to_string(),
            );
            if let Some(cooldown_ms) = profile.cooldown_ms {
                wake.metadata
                    .insert(PROFILE_COOLDOWN_MS_KEY.into(), cooldown_ms.to_string());
            }
            let result = self.wake(wake).await?;
            let mut result = result;
            for (key, value) in &profile.metadata {
                result.metadata.insert(key.clone(), value.clone());
            }
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
}
