//! Runtime-host adapters for Scheduler and Heartbeat autonomy services.
//!
//! This module is the approved composition-root bridge for autonomy services.
//! It wraps service-contract providers behind the kernel-visible
//! [`macaca_kernel::SystemService`] boundary and registers fail-closed
//! unavailable providers when no concrete scheduler or heartbeat implementation
//! is installed.  The adapter never parses cron expressions, evaluates
//! application workflows, or branches on provider/application/business names.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_heartbeat::{HeartbeatService, LocalHeartbeatProvider, UnavailableHeartbeatProvider};
use macaca_kernel::SystemService;
use macaca_proto::{
    CancelScheduledAgentTaskCommand, CreateScheduledAgentTaskCommand, HeartbeatCancelWakeCommand,
    HeartbeatQueryCommand, HeartbeatWakeCommand, KernelServiceId, MacacaError, MacacaResult,
    RecordScheduledAgentTaskResultCommand, ResolveScheduledAgentTaskPayloadCommand,
    ScheduledAgentTaskQueryCommand, SchedulerDeleteJobCommand, SchedulerGetJobCommand,
    SchedulerJobCommand, SchedulerLifecycleJobCommand, SchedulerListJobsCommand,
    SchedulerQueryCommand, SchedulerRegisterJobCommand, SchedulerUpdateJobCommand,
    ServiceCallResult, ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth,
    ServiceResult, TraceContext, HEARTBEAT_CANCEL_WAKE_COMMAND, HEARTBEAT_GET_RUN_COMMAND,
    HEARTBEAT_HEALTH_COMMAND, HEARTBEAT_LIST_RUNS_COMMAND, HEARTBEAT_SERVICE_ID,
    HEARTBEAT_SNAPSHOT_COMMAND, HEARTBEAT_WAKE_COMMAND, SCHEDULED_AGENT_TASK_CANCEL_COMMAND,
    SCHEDULED_AGENT_TASK_CREATE_COMMAND, SCHEDULED_AGENT_TASK_GET_COMMAND,
    SCHEDULED_AGENT_TASK_HEALTH_COMMAND, SCHEDULED_AGENT_TASK_LIST_COMMAND,
    SCHEDULED_AGENT_TASK_RECORD_RESULT_COMMAND, SCHEDULED_AGENT_TASK_RESOLVE_PAYLOAD_COMMAND,
    SCHEDULED_AGENT_TASK_SERVICE_ID, SCHEDULER_DELETE_JOB_COMMAND, SCHEDULER_GET_JOB_COMMAND,
    SCHEDULER_GET_RUN_COMMAND, SCHEDULER_HEALTH_COMMAND, SCHEDULER_LIST_JOBS_COMMAND,
    SCHEDULER_LIST_RUNS_COMMAND, SCHEDULER_PAUSE_JOB_COMMAND, SCHEDULER_REGISTER_JOB_COMMAND,
    SCHEDULER_RESUME_JOB_COMMAND, SCHEDULER_SERVICE_ID, SCHEDULER_SNAPSHOT_COMMAND,
    SCHEDULER_TRIGGER_JOB_COMMAND, SCHEDULER_UPDATE_JOB_COMMAND,
};
use macaca_scheduled_agent_task::{
    LocalScheduledAgentTaskProvider, ScheduledAgentTaskService,
    UnavailableScheduledAgentTaskProvider,
};
use macaca_scheduler::{LocalSchedulerProvider, SchedulerService, UnavailableSchedulerProvider};
use tracing::info;

use crate::{
    autonomy_runtime_config::{AutonomyProviderMode, AutonomyRuntimeConfig},
    autonomy_supervisor::AutonomySupervisor,
    ServiceProviderFactoryContext, ServiceProviderInstance, ServiceRuntime,
    StaticServiceProviderFactory,
};

/// Runtime-host adapter for a Scheduler service provider.
///
/// The adapter implements the Adapter/Bridge pattern.  `ServiceRuntime` speaks
/// generic `ServiceCommand`, while the Scheduler provider speaks typed command
/// DTOs.  The adapter only decodes commands and delegates; actual scheduling
/// semantics remain owned by the injected provider.
pub struct SchedulerSystemServiceProvider {
    provider: Arc<dyn SchedulerService>,
}

impl SchedulerSystemServiceProvider {
    /// Wrap a concrete or remote Scheduler provider.
    pub fn new(provider: Arc<dyn SchedulerService>) -> Self {
        Self { provider }
    }

    /// Build the fail-closed Null Object provider used by default.
    pub fn unavailable() -> Self {
        Self::new(Arc::new(UnavailableSchedulerProvider::default()))
    }
}

#[async_trait]
impl SystemService for SchedulerSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.provider.descriptor()
    }

    async fn start(&self) -> ServiceResult<()> {
        let descriptor = self.provider.descriptor();
        info!(
            service_id = %descriptor.id,
            "scheduler system service provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = command_trace(&command)?;
        info!(
            service_id = SCHEDULER_SERVICE_ID,
            command = %command.name,
            trace_id = %trace.trace_id,
            "scheduler system service command accepted"
        );
        match command.name.as_str() {
            SCHEDULER_REGISTER_JOB_COMMAND => {
                let typed: SchedulerRegisterJobCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .register_job(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            SCHEDULER_UPDATE_JOB_COMMAND => {
                let typed: SchedulerUpdateJobCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .update_job(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            SCHEDULER_PAUSE_JOB_COMMAND => {
                let typed: SchedulerLifecycleJobCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .pause_job(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            SCHEDULER_RESUME_JOB_COMMAND => {
                let typed: SchedulerLifecycleJobCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .resume_job(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            SCHEDULER_DELETE_JOB_COMMAND => {
                let typed: SchedulerDeleteJobCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .delete_job(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            SCHEDULER_TRIGGER_JOB_COMMAND => {
                let typed: SchedulerJobCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .trigger_job(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            SCHEDULER_GET_JOB_COMMAND => {
                let typed: SchedulerGetJobCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .get_job(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            SCHEDULER_LIST_JOBS_COMMAND => {
                let typed: SchedulerListJobsCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .list_jobs(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            SCHEDULER_GET_RUN_COMMAND => {
                let typed: SchedulerQueryCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .get_run(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            SCHEDULER_LIST_RUNS_COMMAND => {
                let typed: SchedulerQueryCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .list_runs(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            SCHEDULER_HEALTH_COMMAND => service_result(
                self.provider
                    .health(trace.clone())
                    .await
                    .map_err(service_adapter_error)?,
                trace,
            ),
            SCHEDULER_SNAPSHOT_COMMAND => {
                let typed: SchedulerQueryCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .snapshot(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            other => Err(ServiceError::UnsupportedCommand(format!(
                "unsupported Scheduler service command '{other}'"
            ))),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(
            service_id = SCHEDULER_SERVICE_ID,
            "scheduler system service provider stopped"
        );
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        info!(
            service_id = SCHEDULER_SERVICE_ID,
            "scheduler system service provider cleanup completed"
        );
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(self.provider.descriptor().health)
    }
}

/// Runtime-host adapter for a Scheduled Agent Task service provider.
///
/// The adapter is the service-runtime Bridge for scheduled task intent.  It
/// decodes generic `ServiceCommand` envelopes into typed DTOs and delegates to
/// the injected provider.  It does not store prompts, compute schedules, or
/// execute agents; those responsibilities stay in the scheduled-task service,
/// Scheduler service, and Agent Execution service respectively.
pub struct ScheduledAgentTaskSystemServiceProvider {
    provider: Arc<dyn ScheduledAgentTaskService>,
}

impl ScheduledAgentTaskSystemServiceProvider {
    /// Wrap a concrete, remote, plugin, mock, or unavailable provider.
    pub fn new(provider: Arc<dyn ScheduledAgentTaskService>) -> Self {
        Self { provider }
    }

    /// Build the fail-closed Null Object provider used by default.
    pub fn unavailable() -> Self {
        Self::new(Arc::new(UnavailableScheduledAgentTaskProvider::default()))
    }
}

#[async_trait]
impl SystemService for ScheduledAgentTaskSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.provider.descriptor()
    }

    async fn start(&self) -> ServiceResult<()> {
        let descriptor = self.provider.descriptor();
        info!(
            service_id = %descriptor.id,
            "scheduled agent task system service provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = command_trace(&command)?;
        info!(
            service_id = SCHEDULED_AGENT_TASK_SERVICE_ID,
            command = %command.name,
            trace_id = %trace.trace_id,
            "scheduled agent task system service command accepted"
        );
        match command.name.as_str() {
            SCHEDULED_AGENT_TASK_CREATE_COMMAND => {
                let typed: CreateScheduledAgentTaskCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .create_task(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            SCHEDULED_AGENT_TASK_GET_COMMAND => {
                let typed: ScheduledAgentTaskQueryCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .get_task(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            SCHEDULED_AGENT_TASK_LIST_COMMAND => {
                let typed: ScheduledAgentTaskQueryCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .list_tasks(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            SCHEDULED_AGENT_TASK_CANCEL_COMMAND => {
                let typed: CancelScheduledAgentTaskCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .cancel_task(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            SCHEDULED_AGENT_TASK_RESOLVE_PAYLOAD_COMMAND => {
                let typed: ResolveScheduledAgentTaskPayloadCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .resolve_payload(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            SCHEDULED_AGENT_TASK_RECORD_RESULT_COMMAND => {
                let typed: RecordScheduledAgentTaskResultCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .record_result(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            SCHEDULED_AGENT_TASK_HEALTH_COMMAND => service_result(
                self.provider
                    .health(trace.clone())
                    .await
                    .map_err(service_adapter_error)?,
                trace,
            ),
            other => Err(ServiceError::UnsupportedCommand(format!(
                "unsupported Scheduled Agent Task service command '{other}'"
            ))),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(
            service_id = SCHEDULED_AGENT_TASK_SERVICE_ID,
            "scheduled agent task system service provider stopped"
        );
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        info!(
            service_id = SCHEDULED_AGENT_TASK_SERVICE_ID,
            "scheduled agent task system service provider cleanup completed"
        );
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(self.provider.descriptor().health)
    }
}

/// Runtime-host adapter for a Heartbeat service provider.
///
/// This adapter decodes generic service-runtime commands into typed Heartbeat
/// commands.  Wake coalescing and gate evaluation remain provider behavior; the
/// adapter only preserves trace, structured errors, and sanitized result
/// envelopes.
pub struct HeartbeatSystemServiceProvider {
    provider: Arc<dyn HeartbeatService>,
}

impl HeartbeatSystemServiceProvider {
    /// Wrap a concrete or remote Heartbeat provider.
    pub fn new(provider: Arc<dyn HeartbeatService>) -> Self {
        Self { provider }
    }

    /// Build the fail-closed Null Object provider used by default.
    pub fn unavailable() -> Self {
        Self::new(Arc::new(UnavailableHeartbeatProvider::default()))
    }
}

#[async_trait]
impl SystemService for HeartbeatSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.provider.descriptor()
    }

    async fn start(&self) -> ServiceResult<()> {
        let descriptor = self.provider.descriptor();
        info!(
            service_id = %descriptor.id,
            "heartbeat system service provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = command_trace(&command)?;
        info!(
            service_id = HEARTBEAT_SERVICE_ID,
            command = %command.name,
            trace_id = %trace.trace_id,
            "heartbeat system service command accepted"
        );
        match command.name.as_str() {
            HEARTBEAT_WAKE_COMMAND => {
                let typed: HeartbeatWakeCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .wake(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            HEARTBEAT_CANCEL_WAKE_COMMAND => {
                let typed: HeartbeatCancelWakeCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .cancel_wake(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            HEARTBEAT_GET_RUN_COMMAND => {
                let typed: HeartbeatQueryCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .get_run(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            HEARTBEAT_LIST_RUNS_COMMAND => {
                let typed: HeartbeatQueryCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .list_runs(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            HEARTBEAT_HEALTH_COMMAND => service_result(
                self.provider
                    .health(trace.clone())
                    .await
                    .map_err(service_adapter_error)?,
                trace,
            ),
            HEARTBEAT_SNAPSHOT_COMMAND => {
                let typed: HeartbeatQueryCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .snapshot(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            other => Err(ServiceError::UnsupportedCommand(format!(
                "unsupported Heartbeat service command '{other}'"
            ))),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(
            service_id = HEARTBEAT_SERVICE_ID,
            "heartbeat system service provider stopped"
        );
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        info!(
            service_id = HEARTBEAT_SERVICE_ID,
            "heartbeat system service provider cleanup completed"
        );
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(self.provider.descriptor().health)
    }
}

/// Started autonomy service ids returned by runtime-host bootstrap.
#[derive(Clone, Default)]
pub struct AutonomyRuntimeBundle {
    pub started_services: Vec<KernelServiceId>,
    pub provider_mode: String,
    pub supervisor: Option<AutonomySupervisor>,
}

/// Register fail-closed autonomy providers into an existing service runtime.
///
/// Runtime-host owns this composition step because it is the approved Abstract
/// Factory boundary for provider construction.  Web, CLI, SDK, applications,
/// and the microkernel can observe or call these services, but they must not
/// construct the providers themselves.
pub async fn bootstrap_autonomy_unavailable_services(
    runtime: Arc<ServiceRuntime>,
    trace_prefix: impl Into<String>,
) -> MacacaResult<AutonomyRuntimeBundle> {
    let trace_prefix = trace_prefix.into();
    let mut bundle = AutonomyRuntimeBundle {
        provider_mode: "unavailable".into(),
        ..AutonomyRuntimeBundle::default()
    };

    register_and_start(
        Arc::clone(&runtime),
        Arc::new(SchedulerSystemServiceProvider::unavailable()),
        format!("{trace_prefix}-scheduler-service"),
        &mut bundle,
    )
    .await?;
    register_and_start(
        Arc::clone(&runtime),
        Arc::new(ScheduledAgentTaskSystemServiceProvider::unavailable()),
        format!("{trace_prefix}-scheduled-agent-task-service"),
        &mut bundle,
    )
    .await?;
    register_and_start(
        runtime,
        Arc::new(HeartbeatSystemServiceProvider::unavailable()),
        format!("{trace_prefix}-heartbeat-service"),
        &mut bundle,
    )
    .await?;

    info!(
        services = bundle.started_services.len(),
        "autonomy unavailable services bootstrap completed"
    );
    Ok(bundle)
}

/// Register autonomy providers according to explicit runtime-host config.
///
/// Disabled mode delegates to the existing unavailable bootstrap and starts no
/// background loop.  Local mode constructs concrete providers only inside this
/// runtime-host composition root, then returns an optional supervisor handle for
/// lifecycle control and deterministic manual ticks in tests.
pub async fn bootstrap_autonomy_services(
    runtime: Arc<ServiceRuntime>,
    trace_prefix: impl Into<String>,
    config: AutonomyRuntimeConfig,
) -> MacacaResult<AutonomyRuntimeBundle> {
    let config = config.normalized();
    let trace_prefix = trace_prefix.into();
    info!(
        provider_mode = config.mode_label(),
        supervisor_enabled = config.supervisor_enabled,
        "autonomy runtime configuration resolved"
    );
    match config.provider_mode {
        AutonomyProviderMode::Unavailable => {
            bootstrap_autonomy_unavailable_services(runtime, trace_prefix).await
        }
        AutonomyProviderMode::Local => {
            bootstrap_autonomy_local_services(runtime, trace_prefix, config).await
        }
    }
}

/// Register local Scheduler and Heartbeat providers and optionally start the supervisor.
pub async fn bootstrap_autonomy_local_services(
    runtime: Arc<ServiceRuntime>,
    trace_prefix: impl Into<String>,
    config: AutonomyRuntimeConfig,
) -> MacacaResult<AutonomyRuntimeBundle> {
    let trace_prefix = trace_prefix.into();
    let config = config.normalized();
    let scheduler = Arc::new(LocalSchedulerProvider::new());
    let scheduled_agent_task = Arc::new(LocalScheduledAgentTaskProvider::new(scheduler.clone()));
    let heartbeat = Arc::new(LocalHeartbeatProvider::new());
    let mut bundle = AutonomyRuntimeBundle {
        provider_mode: "local".into(),
        ..AutonomyRuntimeBundle::default()
    };

    register_and_start(
        Arc::clone(&runtime),
        Arc::new(SchedulerSystemServiceProvider::new(scheduler.clone())),
        format!("{trace_prefix}-scheduler-local-service"),
        &mut bundle,
    )
    .await?;
    register_and_start(
        Arc::clone(&runtime),
        Arc::new(ScheduledAgentTaskSystemServiceProvider::new(
            scheduled_agent_task,
        )),
        format!("{trace_prefix}-scheduled-agent-task-local-service"),
        &mut bundle,
    )
    .await?;
    register_and_start(
        Arc::clone(&runtime),
        Arc::new(HeartbeatSystemServiceProvider::new(heartbeat.clone())),
        format!("{trace_prefix}-heartbeat-local-service"),
        &mut bundle,
    )
    .await?;

    let supervisor =
        AutonomySupervisor::new(Arc::clone(&runtime), scheduler, heartbeat, config.clone());
    supervisor
        .start(TraceContext::new(format!(
            "{trace_prefix}-supervisor-start"
        )))
        .await?;
    if config.recovery_wake_enabled {
        supervisor
            .run_recovery_wake_once(TraceContext::new(format!("{trace_prefix}-recovery-wake")))
            .await?;
    }
    bundle.supervisor = Some(supervisor);
    info!(
        services = bundle.started_services.len(),
        supervisor_enabled = config.supervisor_enabled,
        "autonomy local services bootstrap completed"
    );
    Ok(bundle)
}

async fn register_and_start(
    runtime: Arc<ServiceRuntime>,
    service: Arc<dyn SystemService>,
    trace_id: String,
    bundle: &mut AutonomyRuntimeBundle,
) -> MacacaResult<()> {
    let descriptor = service.descriptor();
    let service_id = descriptor.id.clone();
    let trace = TraceContext::new(trace_id);
    info!(
        service_id = %service_id,
        trace_id = %trace.trace_id,
        "autonomy service registering provider"
    );
    runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(descriptor, service)),
            ServiceProviderFactoryContext::new(),
        )
        .await
        .map_err(runtime_error)?;
    runtime
        .start(&service_id, trace.clone())
        .await
        .map_err(runtime_error)?;
    info!(
        service_id = %service_id,
        trace_id = %trace.trace_id,
        "autonomy service provider started"
    );
    bundle.started_services.push(service_id);
    Ok(())
}

fn command_trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
    command
        .trace
        .clone()
        .ok_or(ServiceError::MissingTraceContext)
}

fn decode<T>(payload: serde_json::Value) -> ServiceResult<T>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_value(payload).map_err(|error| {
        ServiceError::InvalidArgument(format!("invalid autonomy service payload: {error}"))
    })
}

fn service_result<T>(output: T, trace: TraceContext) -> ServiceResult<ServiceCallResult>
where
    T: serde::Serialize,
{
    Ok(ServiceCallResult {
        output: serde_json::to_value(output).map_err(|error| {
            ServiceError::InvalidArgument(format!("autonomy result serialization failed: {error}"))
        })?,
        trace,
        status: "ok".into(),
        metadata: Default::default(),
        cleanup_hint: Some(macaca_proto::CleanupPolicy::None),
    })
}

fn service_adapter_error(error: MacacaError) -> ServiceError {
    ServiceError::ServiceUnavailable(error.to_string())
}

fn runtime_error(error: crate::ServiceRuntimeError) -> MacacaError {
    MacacaError::Config(error.to_string())
}
