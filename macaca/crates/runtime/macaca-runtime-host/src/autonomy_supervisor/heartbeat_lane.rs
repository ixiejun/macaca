//! Heartbeat lane for the local autonomy supervisor.
//!
//! HeartbeatLane owns native heartbeat cadence coordination. It calls the
//! Heartbeat service's profile tick API directly through the injected local
//! provider and does not require Scheduler jobs, due-run materialization, or
//! Scheduler leases. This keeps agent/system heartbeat liveness independent
//! from application schedule management.

use std::sync::Arc;

use macaca_heartbeat::{HeartbeatService, LocalHeartbeatProvider};
use macaca_proto::{
    AutonomyScope, HeartbeatWakeCommand, HeartbeatWakeIntent, MacacaResult, TraceContext,
};
use tracing::info;

use super::heartbeat_agent_dispatch::{
    HeartbeatAgentDispatchStrategy, HeartbeatAgentDispatchSummary,
};
use crate::ServiceRuntime;

/// Runtime-host Strategy object for one bounded Heartbeat supervisor tick.
pub(crate) struct HeartbeatLane {
    runtime: Arc<ServiceRuntime>,
    heartbeat: Arc<LocalHeartbeatProvider>,
    recovery_wake_enabled: bool,
}

impl HeartbeatLane {
    /// Build a Heartbeat lane from approved runtime-host composition inputs.
    pub(crate) fn new(
        runtime: Arc<ServiceRuntime>,
        heartbeat: Arc<LocalHeartbeatProvider>,
        recovery_wake_enabled: bool,
    ) -> Self {
        Self {
            runtime,
            heartbeat,
            recovery_wake_enabled,
        }
    }

    /// Execute one native Heartbeat cadence tick.
    ///
    /// The tick evaluates Heartbeat-owned profiles and gates. It never creates
    /// Scheduler jobs and never interprets application business payloads.
    pub(crate) async fn tick_once(&self, trace: TraceContext) -> MacacaResult<bool> {
        let results = self
            .heartbeat
            .tick_native_profiles_once(trace.clone())
            .await?;
        let accepted = results.iter().any(|result| result.accepted);
        let dispatcher = HeartbeatAgentDispatchStrategy::new(Arc::clone(&self.runtime));
        let mut dispatch_summary = HeartbeatAgentDispatchSummary::default();
        for result in results.iter().filter(|result| result.accepted) {
            let summary = dispatcher.dispatch_after_accepted_wake(result).await?;
            dispatch_summary.queried += summary.queried;
            dispatch_summary.enabled += summary.enabled;
            dispatch_summary.dispatched += summary.dispatched;
            dispatch_summary.skipped += summary.skipped;
            dispatch_summary.failed += summary.failed;
        }
        info!(
            trace_id = trace.trace_id.as_str(),
            accepted,
            processed = results.len(),
            declaration_count = dispatch_summary.queried,
            enabled_declaration_count = dispatch_summary.enabled,
            dispatched_count = dispatch_summary.dispatched,
            skipped_count = dispatch_summary.skipped,
            failed_count = dispatch_summary.failed,
            "autonomy heartbeat lane native cadence tick completed"
        );
        Ok(accepted)
    }

    /// Emit one provider-neutral recovery wake when enabled.
    pub(crate) async fn recovery_wake_once(&self, trace: TraceContext) -> MacacaResult<bool> {
        if !self.recovery_wake_enabled {
            info!(
                trace_id = trace.trace_id.as_str(),
                "autonomy heartbeat lane recovery wake skipped by config"
            );
            return Ok(false);
        }
        let wake = HeartbeatWakeCommand::new(
            trace.clone(),
            AutonomyScope::global(),
            "runtime.autonomy_supervisor.recovery",
            HeartbeatWakeIntent::Recovery {
                reason_code: "runtime_host_startup".into(),
            },
        )?;
        let result = self.heartbeat.wake(wake).await?;
        info!(
            trace_id = trace.trace_id.as_str(),
            accepted = result.accepted,
            "autonomy heartbeat lane recovery wake completed"
        );
        Ok(result.accepted)
    }
}
