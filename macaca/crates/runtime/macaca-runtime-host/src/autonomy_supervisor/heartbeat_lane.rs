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
    AutonomyScope, HeartbeatCommandResult, HeartbeatCompleteRunCommand, HeartbeatRunState,
    HeartbeatWakeCommand, HeartbeatWakeIntent, MacacaResult, TraceContext,
};
use tracing::{info, warn};

use super::heartbeat_agent_dispatch::{
    HeartbeatAgentDispatchStrategy, HeartbeatAgentDispatchSummary,
};
use crate::ServiceRuntime;

/// Runtime-host Strategy object for one bounded Heartbeat supervisor tick.
pub(crate) struct HeartbeatLane {
    runtime: Arc<ServiceRuntime>,
    heartbeat: Arc<LocalHeartbeatProvider>,
    recovery_wake_enabled: bool,
    dispatch_timeout_ms: u64,
}

#[cfg(test)]
mod tests;

impl HeartbeatLane {
    /// Build a Heartbeat lane from approved runtime-host composition inputs.
    pub(crate) fn new(
        runtime: Arc<ServiceRuntime>,
        heartbeat: Arc<LocalHeartbeatProvider>,
        recovery_wake_enabled: bool,
        dispatch_timeout_ms: u64,
    ) -> Self {
        Self {
            runtime,
            heartbeat,
            recovery_wake_enabled,
            dispatch_timeout_ms: dispatch_timeout_ms.max(1),
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
        let mut dispatch_summary = HeartbeatAgentDispatchSummary::default();
        for result in results.iter().filter(|result| result.accepted) {
            let runtime = Arc::clone(&self.runtime);
            let wake = result.clone();
            let timeout_ms = self.dispatch_timeout_ms;
            dispatch_summary.dispatched += 1;
            info!(
                trace_id = trace.trace_id.as_str(),
                heartbeat_run_id = result
                    .run_id
                    .as_ref()
                    .map(|run_id| run_id.as_str())
                    .unwrap_or("none"),
                "autonomy heartbeat lane handing off accepted wake to background agent dispatch"
            );
            tokio::spawn(async move {
                let dispatcher = HeartbeatAgentDispatchStrategy::with_timeout(runtime, timeout_ms);
                match dispatcher.dispatch_after_accepted_wake(&wake).await {
                    Ok(summary) => {
                        if let Err(error) = record_dispatch_completion(
                            &dispatcher,
                            &wake,
                            summary.completion_state.clone(),
                            summary.reason_code.clone(),
                            summary.metadata.clone(),
                        )
                        .await
                        {
                            warn!(
                                trace_id = wake.trace.trace_id.as_str(),
                                error = %error,
                                "heartbeat dispatch completion memento record failed"
                            );
                        }
                        info!(
                            trace_id = wake.trace.trace_id.as_str(),
                            queried = summary.queried,
                            enabled = summary.enabled,
                            dispatched = summary.dispatched,
                            skipped = summary.skipped,
                            failed = summary.failed,
                            "heartbeat background agent dispatch completed"
                        );
                    }
                    Err(error) => {
                        let completion = record_dispatch_completion(
                            &dispatcher,
                            &wake,
                            Some(HeartbeatRunState::Failed),
                            Some("heartbeat_dispatch_failed".into()),
                            Default::default(),
                        )
                        .await;
                        if let Err(record_error) = completion {
                            warn!(
                                trace_id = wake.trace.trace_id.as_str(),
                                error = %record_error,
                                "heartbeat dispatch failure memento record failed"
                            );
                        }
                        warn!(
                            trace_id = wake.trace.trace_id.as_str(),
                            error = %error,
                            "heartbeat background agent dispatch failed"
                        );
                    }
                }
            });
        }
        info!(
            trace_id = trace.trace_id.as_str(),
            accepted,
            processed = results.len(),
            handoff_count = dispatch_summary.dispatched,
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

async fn record_dispatch_completion(
    dispatcher: &HeartbeatAgentDispatchStrategy,
    wake: &HeartbeatCommandResult,
    state: Option<HeartbeatRunState>,
    reason_code: Option<String>,
    metadata: std::collections::BTreeMap<String, String>,
) -> MacacaResult<()> {
    let Some(run_id) = wake.run_id.clone() else {
        return Ok(());
    };
    let state = state.unwrap_or(HeartbeatRunState::Skipped);
    let reason_code = reason_code.unwrap_or_else(|| "heartbeat_dispatch_noop".into());
    let mut command =
        HeartbeatCompleteRunCommand::new(wake.trace.clone(), run_id, state, reason_code)?;
    command.metadata = metadata;
    dispatcher.record_completion(command).await
}
