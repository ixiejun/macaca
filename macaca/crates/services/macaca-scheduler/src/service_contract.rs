//! Service contract and Null Object provider for `service.scheduler`.
//!
//! The Scheduler service is a provider-neutral Facade over durable scheduled
//! work.  The trait in this module defines the command surface that local,
//! remote, plugin, mock, and unavailable providers must implement.  A concrete
//! provider may use any cron parser, queue, database, or timer strategy behind
//! this boundary, but callers always see typed commands/results, trace-aware
//! failures, sanitized snapshots, and no application-specific behavior.

use async_trait::async_trait;
use macaca_proto::{
    AutonomyStructuredError, CapabilityId, CleanupPolicy, KernelServiceId, MacacaResult,
    SchedulerCommandResult, SchedulerJobCommand, SchedulerJobId, SchedulerQueryCommand,
    SchedulerRegisterJobCommand, SchedulerRunId, SchedulerRunSummary, SchedulerServiceSnapshot,
    ServiceCapability, ServiceDescriptor, ServiceHealth,
    ServiceLifecycleState, ServiceScope, ServiceType, TraceContext, TraceSchemaRef,
    SCHEDULER_DELETE_JOB_COMMAND, SCHEDULER_GET_JOB_COMMAND, SCHEDULER_GET_RUN_COMMAND,
    SCHEDULER_HEALTH_COMMAND, SCHEDULER_LIST_JOBS_COMMAND, SCHEDULER_LIST_RUNS_COMMAND,
    SCHEDULER_PAUSE_JOB_COMMAND, SCHEDULER_REGISTER_JOB_COMMAND, SCHEDULER_RESUME_JOB_COMMAND,
    SCHEDULER_SERVICE_ID, SCHEDULER_SNAPSHOT_COMMAND, SCHEDULER_TRIGGER_JOB_COMMAND,
    SCHEDULER_UPDATE_JOB_COMMAND,
};
use tracing::{info, warn};

/// Provider-neutral Scheduler service interface.
///
/// This trait is intentionally narrow and command-oriented.  Runtime decorators
/// should attach trace, policy, resource, entitlement, and audit behavior before
/// invoking a provider implementation.  The provider itself is responsible for
/// durable scheduling semantics but must not branch on application names,
/// workflow names, model names, driver names, gateway names, chain names, or
/// business domains.
#[async_trait]
pub trait SchedulerService: Send + Sync {
    /// Return the service descriptor used by registry, facade, and diagnostics.
    fn descriptor(&self) -> ServiceDescriptor;

    /// Return a lightweight health/snapshot view for diagnostics surfaces.
    async fn health(&self, trace: TraceContext) -> MacacaResult<SchedulerServiceSnapshot>;

    /// Return a bounded snapshot of scheduler state.
    async fn snapshot(
        &self,
        command: SchedulerQueryCommand,
    ) -> MacacaResult<SchedulerServiceSnapshot>;

    /// Register a durable scheduled job definition.
    async fn register_job(
        &self,
        command: SchedulerRegisterJobCommand,
    ) -> MacacaResult<SchedulerCommandResult>;

    /// Update an existing job definition or provider-neutral metadata.
    async fn update_job(
        &self,
        command: SchedulerJobCommand,
    ) -> MacacaResult<SchedulerCommandResult>;

    /// Pause a job so due-run materialization stops without deleting history.
    async fn pause_job(
        &self,
        command: SchedulerJobCommand,
    ) -> MacacaResult<SchedulerCommandResult>;

    /// Resume a paused job after policy and provider checks pass.
    async fn resume_job(
        &self,
        command: SchedulerJobCommand,
    ) -> MacacaResult<SchedulerCommandResult>;

    /// Tombstone a job while preserving audit and run-history evidence.
    async fn delete_job(
        &self,
        command: SchedulerJobCommand,
    ) -> MacacaResult<SchedulerCommandResult>;

    /// Materialize a manual run request through scheduler semantics.
    async fn trigger_job(
        &self,
        command: SchedulerJobCommand,
    ) -> MacacaResult<SchedulerCommandResult>;

    /// Read one durable job definition if the provider supports lookup.
    async fn get_job(
        &self,
        command: SchedulerQueryCommand,
    ) -> MacacaResult<SchedulerCommandResult>;

    /// List jobs within a provider-neutral scope.
    async fn list_jobs(
        &self,
        command: SchedulerQueryCommand,
    ) -> MacacaResult<Vec<SchedulerCommandResult>>;

    /// Read one materialized run if the provider supports lookup.
    async fn get_run(
        &self,
        command: SchedulerQueryCommand,
    ) -> MacacaResult<SchedulerCommandResult>;

    /// List bounded run history for diagnostics and audit replay.
    async fn list_runs(
        &self,
        command: SchedulerQueryCommand,
    ) -> MacacaResult<Vec<SchedulerRunSummary>>;
}

/// Fail-closed Scheduler provider used when no concrete provider is installed.
///
/// This is the Null Object pattern applied to serviceization: callers can still
/// ask for descriptors, health, snapshots, and command results, but every side
/// effect returns a structured unavailable error with trace correlation.  This
/// prevents silent fallbacks and makes optional-provider absence auditable.
#[derive(Debug, Clone)]
pub struct UnavailableSchedulerProvider {
    provider_id: String,
    reason: String,
}

impl UnavailableSchedulerProvider {
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
        job_id: Option<SchedulerJobId>,
        run_id: Option<SchedulerRunId>,
    ) -> MacacaResult<SchedulerCommandResult> {
        warn!(
            service_id = SCHEDULER_SERVICE_ID,
            provider_id = self.provider_id.as_str(),
            command = command_name,
            trace_id = trace.trace_id.as_str(),
            "scheduler command rejected because no scheduler provider is available"
        );
        let error = AutonomyStructuredError::unavailable(trace.clone(), self.reason.clone())?;
        Ok(SchedulerCommandResult {
            job_id,
            run_id,
            lifecycle: None,
            run_state: None,
            accepted: false,
            error: Some(error),
            trace,
            audit_id: None,
            metadata: Default::default(),
        })
    }
}

impl Default for UnavailableSchedulerProvider {
    fn default() -> Self {
        Self::new("scheduler provider is not installed")
    }
}

#[async_trait]
impl SchedulerService for UnavailableSchedulerProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        let mut descriptor = ServiceDescriptor::new(
            KernelServiceId::new(SCHEDULER_SERVICE_ID),
            ServiceType::new("autonomy.scheduler"),
            TraceSchemaRef::new("macaca.trace.scheduler.v1"),
        );
        descriptor.lifecycle_state = ServiceLifecycleState::Registered;
        descriptor.health = ServiceHealth::Unavailable {
            reason: self.reason.clone(),
        };
        descriptor.supported_scopes = vec![ServiceScope::Global];
        descriptor.cleanup_policy = CleanupPolicy::None;
        descriptor.capabilities = vec![
            ServiceCapability::new(
                CapabilityId::new(SCHEDULER_REGISTER_JOB_COMMAND),
                "Register provider-neutral scheduled jobs",
            ),
            ServiceCapability::new(
                CapabilityId::new(SCHEDULER_SNAPSHOT_COMMAND),
                "Read sanitized scheduler snapshots",
            ),
            ServiceCapability::new(
                CapabilityId::new(SCHEDULER_HEALTH_COMMAND),
                "Read scheduler health",
            ),
        ];
        descriptor
    }

    async fn health(&self, trace: TraceContext) -> MacacaResult<SchedulerServiceSnapshot> {
        info!(
            service_id = SCHEDULER_SERVICE_ID,
            provider_id = self.provider_id.as_str(),
            trace_id = trace.trace_id.as_str(),
            "scheduler health requested from unavailable provider"
        );
        Ok(SchedulerServiceSnapshot::unavailable(self.reason.clone()))
    }

    async fn snapshot(
        &self,
        command: SchedulerQueryCommand,
    ) -> MacacaResult<SchedulerServiceSnapshot> {
        info!(
            service_id = SCHEDULER_SERVICE_ID,
            provider_id = self.provider_id.as_str(),
            trace_id = command.trace.trace_id.as_str(),
            "scheduler snapshot requested from unavailable provider"
        );
        Ok(SchedulerServiceSnapshot::unavailable(self.reason.clone()))
    }

    async fn register_job(
        &self,
        command: SchedulerRegisterJobCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        self.unavailable_result(SCHEDULER_REGISTER_JOB_COMMAND, command.trace, None, None)
    }

    async fn update_job(
        &self,
        command: SchedulerJobCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        self.unavailable_result(
            SCHEDULER_UPDATE_JOB_COMMAND,
            command.trace,
            Some(command.job_id),
            None,
        )
    }

    async fn pause_job(
        &self,
        command: SchedulerJobCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        self.unavailable_result(
            SCHEDULER_PAUSE_JOB_COMMAND,
            command.trace,
            Some(command.job_id),
            None,
        )
    }

    async fn resume_job(
        &self,
        command: SchedulerJobCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        self.unavailable_result(
            SCHEDULER_RESUME_JOB_COMMAND,
            command.trace,
            Some(command.job_id),
            None,
        )
    }

    async fn delete_job(
        &self,
        command: SchedulerJobCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        self.unavailable_result(
            SCHEDULER_DELETE_JOB_COMMAND,
            command.trace,
            Some(command.job_id),
            None,
        )
    }

    async fn trigger_job(
        &self,
        command: SchedulerJobCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        self.unavailable_result(
            SCHEDULER_TRIGGER_JOB_COMMAND,
            command.trace,
            Some(command.job_id),
            None,
        )
    }

    async fn get_job(
        &self,
        command: SchedulerQueryCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        self.unavailable_result(SCHEDULER_GET_JOB_COMMAND, command.trace, command.job_id, None)
    }

    async fn list_jobs(
        &self,
        command: SchedulerQueryCommand,
    ) -> MacacaResult<Vec<SchedulerCommandResult>> {
        Ok(vec![self.unavailable_result(
            SCHEDULER_LIST_JOBS_COMMAND,
            command.trace,
            None,
            None,
        )?])
    }

    async fn get_run(
        &self,
        command: SchedulerQueryCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        self.unavailable_result(SCHEDULER_GET_RUN_COMMAND, command.trace, command.job_id, command.run_id)
    }

    async fn list_runs(
        &self,
        command: SchedulerQueryCommand,
    ) -> MacacaResult<Vec<SchedulerRunSummary>> {
        warn!(
            service_id = SCHEDULER_SERVICE_ID,
            provider_id = self.provider_id.as_str(),
            command = SCHEDULER_LIST_RUNS_COMMAND,
            trace_id = command.trace.trace_id.as_str(),
            "scheduler run history requested but no scheduler provider is available"
        );
        Ok(Vec::new())
    }
}
