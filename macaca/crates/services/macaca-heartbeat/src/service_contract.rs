//! Service contract and Null Object provider for `service.heartbeat`.
//!
//! The Heartbeat service coordinates wake requests, gate evaluation, and
//! heartbeat run evidence.  It is intentionally separated from Scheduler so
//! cron semantics remain owned by `service.scheduler` while wake-loop semantics
//! remain owned by `service.heartbeat`.

use async_trait::async_trait;
use macaca_proto::{
    AutonomyStructuredError, CapabilityId, CleanupPolicy, HeartbeatCancelWakeCommand,
    HeartbeatCommandResult, HeartbeatCompleteRunCommand, HeartbeatProfileMutationResult,
    HeartbeatQueryCommand, HeartbeatRunId, HeartbeatRunState, HeartbeatRunSummary,
    HeartbeatServiceSnapshot, HeartbeatUpdateProfileCommand, HeartbeatWakeCommand,
    HeartbeatWakeDisposition, KernelServiceId, MacacaResult, ServiceCapability, ServiceDescriptor,
    ServiceHealth, ServiceLifecycleState, ServiceScope, ServiceType, TraceContext, TraceSchemaRef,
    HEARTBEAT_CANCEL_WAKE_COMMAND, HEARTBEAT_COMPLETE_RUN_COMMAND, HEARTBEAT_GET_RUN_COMMAND,
    HEARTBEAT_HEALTH_COMMAND, HEARTBEAT_LIST_RUNS_COMMAND, HEARTBEAT_SERVICE_ID,
    HEARTBEAT_SNAPSHOT_COMMAND, HEARTBEAT_UPDATE_PROFILE_COMMAND, HEARTBEAT_WAKE_COMMAND,
};
use tracing::{info, warn};

/// Provider-neutral Heartbeat service interface.
///
/// Runtime decorators should perform trace, policy, resource, entitlement, and
/// audit checks before invoking a provider.  Providers own generic wake
/// coalescing and gate decisions only; task execution and product behavior must
/// stay behind their own service boundaries.
#[async_trait]
pub trait HeartbeatService: Send + Sync {
    /// Return the service descriptor used by registry, facade, and diagnostics.
    fn descriptor(&self) -> ServiceDescriptor;

    /// Return a lightweight health/snapshot view for diagnostics surfaces.
    async fn health(&self, trace: TraceContext) -> MacacaResult<HeartbeatServiceSnapshot>;

    /// Return a bounded snapshot of heartbeat state.
    async fn snapshot(
        &self,
        command: HeartbeatQueryCommand,
    ) -> MacacaResult<HeartbeatServiceSnapshot>;

    /// Request a wake for a provider-neutral scope.
    async fn wake(&self, command: HeartbeatWakeCommand) -> MacacaResult<HeartbeatCommandResult>;

    /// Cancel a pending wake or active heartbeat run when provider policy allows it.
    async fn cancel_wake(
        &self,
        command: HeartbeatCancelWakeCommand,
    ) -> MacacaResult<HeartbeatCommandResult>;

    /// Record the terminal outcome of runtime work delegated from a wake.
    ///
    /// Providers must treat this as memento state only. Agent execution,
    /// planning, delivery, and product-specific behavior stay behind their own
    /// services; Heartbeat only records the sanitized state transition that lets
    /// operators audit whether a wake's delegated work actually finished.
    async fn complete_run(
        &self,
        command: HeartbeatCompleteRunCommand,
    ) -> MacacaResult<HeartbeatCommandResult>;

    /// Read one heartbeat run if the provider supports lookup.
    async fn get_run(&self, command: HeartbeatQueryCommand)
        -> MacacaResult<HeartbeatCommandResult>;

    /// List bounded heartbeat history for diagnostics and audit replay.
    async fn list_runs(
        &self,
        command: HeartbeatQueryCommand,
    ) -> MacacaResult<Vec<HeartbeatRunSummary>>;

    /// Update native Heartbeat profile policy.
    ///
    /// Implementations must treat this as runtime profile state, not as an
    /// application manifest mutation. Profile edits are auditable operator
    /// policy changes for cadence and visibility only.
    async fn update_profile(
        &self,
        command: HeartbeatUpdateProfileCommand,
    ) -> MacacaResult<HeartbeatProfileMutationResult>;
}

/// Fail-closed Heartbeat provider used when no concrete provider is installed.
///
/// The Null Object keeps the service boundary observable even when heartbeat is
/// unavailable.  Every command returns a structured unavailable result with
/// trace correlation, so autonomous wake requests cannot disappear or fake
/// success when no provider is configured.
#[derive(Debug, Clone)]
pub struct UnavailableHeartbeatProvider {
    provider_id: String,
    reason: String,
}

impl UnavailableHeartbeatProvider {
    /// Create an unavailable provider with a safe diagnostic reason.
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            provider_id: "unavailable".into(),
            reason: reason.into(),
        }
    }

    /// Return the provider id exposed in snapshots.
    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    fn unavailable_result(
        &self,
        command_name: &'static str,
        trace: TraceContext,
        run_id: Option<HeartbeatRunId>,
    ) -> MacacaResult<HeartbeatCommandResult> {
        warn!(
            service_id = HEARTBEAT_SERVICE_ID,
            provider_id = self.provider_id.as_str(),
            command = command_name,
            trace_id = trace.trace_id.as_str(),
            "heartbeat command rejected because no heartbeat provider is available"
        );
        let error = AutonomyStructuredError::unavailable(trace.clone(), self.reason.clone())?;
        Ok(HeartbeatCommandResult {
            run_id,
            state: Some(HeartbeatRunState::Skipped),
            disposition: HeartbeatWakeDisposition::Skipped,
            gates: Vec::new(),
            accepted: false,
            error: Some(error),
            trace,
            audit_id: None,
            metadata: Default::default(),
        })
    }

    fn unavailable_profile_result(
        &self,
        command: HeartbeatUpdateProfileCommand,
    ) -> MacacaResult<HeartbeatProfileMutationResult> {
        warn!(
            service_id = HEARTBEAT_SERVICE_ID,
            provider_id = self.provider_id.as_str(),
            command = HEARTBEAT_UPDATE_PROFILE_COMMAND,
            profile_id = command.profile_id.as_str(),
            trace_id = command.trace.trace_id.as_str(),
            "heartbeat profile update rejected because no heartbeat provider is available"
        );
        Ok(HeartbeatProfileMutationResult {
            accepted: false,
            profile: None,
            trace: command.trace.clone(),
            audit_id: None,
            error: Some(AutonomyStructuredError::unavailable(
                command.trace,
                self.reason.clone(),
            )?),
            metadata: Default::default(),
        })
    }
}

impl Default for UnavailableHeartbeatProvider {
    fn default() -> Self {
        Self::new("heartbeat provider is not installed")
    }
}

#[async_trait]
impl HeartbeatService for UnavailableHeartbeatProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        let mut descriptor = ServiceDescriptor::new(
            KernelServiceId::new(HEARTBEAT_SERVICE_ID),
            ServiceType::new("autonomy.heartbeat"),
            TraceSchemaRef::new("macaca.trace.heartbeat.v1"),
        );
        descriptor.lifecycle_state = ServiceLifecycleState::Registered;
        descriptor.health = ServiceHealth::Unavailable {
            reason: self.reason.clone(),
        };
        descriptor.supported_scopes = vec![ServiceScope::Global];
        descriptor.cleanup_policy = CleanupPolicy::None;
        descriptor.capabilities = vec![
            ServiceCapability::new(
                CapabilityId::new(HEARTBEAT_WAKE_COMMAND),
                "Request provider-neutral heartbeat wakes",
            ),
            ServiceCapability::new(
                CapabilityId::new(HEARTBEAT_SNAPSHOT_COMMAND),
                "Read sanitized heartbeat snapshots",
            ),
            ServiceCapability::new(
                CapabilityId::new(HEARTBEAT_HEALTH_COMMAND),
                "Read heartbeat health",
            ),
            ServiceCapability::new(
                CapabilityId::new(HEARTBEAT_UPDATE_PROFILE_COMMAND),
                "Update native heartbeat profile policy",
            ),
            ServiceCapability::new(
                CapabilityId::new(HEARTBEAT_COMPLETE_RUN_COMMAND),
                "Record sanitized heartbeat dispatch completion",
            ),
        ];
        descriptor
    }

    async fn health(&self, trace: TraceContext) -> MacacaResult<HeartbeatServiceSnapshot> {
        info!(
            service_id = HEARTBEAT_SERVICE_ID,
            provider_id = self.provider_id.as_str(),
            trace_id = trace.trace_id.as_str(),
            "heartbeat health requested from unavailable provider"
        );
        Ok(HeartbeatServiceSnapshot::unavailable(self.reason.clone()))
    }

    async fn snapshot(
        &self,
        command: HeartbeatQueryCommand,
    ) -> MacacaResult<HeartbeatServiceSnapshot> {
        info!(
            service_id = HEARTBEAT_SERVICE_ID,
            provider_id = self.provider_id.as_str(),
            trace_id = command.trace.trace_id.as_str(),
            "heartbeat snapshot requested from unavailable provider"
        );
        Ok(HeartbeatServiceSnapshot::unavailable(self.reason.clone()))
    }

    async fn wake(&self, command: HeartbeatWakeCommand) -> MacacaResult<HeartbeatCommandResult> {
        self.unavailable_result(HEARTBEAT_WAKE_COMMAND, command.trace, None)
    }

    async fn cancel_wake(
        &self,
        command: HeartbeatCancelWakeCommand,
    ) -> MacacaResult<HeartbeatCommandResult> {
        self.unavailable_result(
            HEARTBEAT_CANCEL_WAKE_COMMAND,
            command.trace,
            Some(command.run_id),
        )
    }

    async fn complete_run(
        &self,
        command: HeartbeatCompleteRunCommand,
    ) -> MacacaResult<HeartbeatCommandResult> {
        self.unavailable_result(
            HEARTBEAT_COMPLETE_RUN_COMMAND,
            command.trace,
            Some(command.run_id),
        )
    }

    async fn get_run(
        &self,
        command: HeartbeatQueryCommand,
    ) -> MacacaResult<HeartbeatCommandResult> {
        self.unavailable_result(HEARTBEAT_GET_RUN_COMMAND, command.trace, command.run_id)
    }

    async fn list_runs(
        &self,
        command: HeartbeatQueryCommand,
    ) -> MacacaResult<Vec<HeartbeatRunSummary>> {
        warn!(
            service_id = HEARTBEAT_SERVICE_ID,
            provider_id = self.provider_id.as_str(),
            command = HEARTBEAT_LIST_RUNS_COMMAND,
            trace_id = command.trace.trace_id.as_str(),
            "heartbeat run history requested but no heartbeat provider is available"
        );
        Ok(Vec::new())
    }

    async fn update_profile(
        &self,
        command: HeartbeatUpdateProfileCommand,
    ) -> MacacaResult<HeartbeatProfileMutationResult> {
        self.unavailable_profile_result(command)
    }
}
