//! Autonomy service bootstrap and composition-root registration.
//!
//! **Pattern:** Abstract Factory — runtime-host owns provider construction and
//! `ServiceRuntime` registration.  Shells, SDK clients, applications, and the
//! microkernel observe or call these services but must not construct providers.

use std::sync::Arc;

use macaca_heartbeat::InProcessHeartbeatProvider;
use macaca_kernel::SystemService;
use macaca_proto::{KernelServiceId, MacacaResult, TraceContext};
use macaca_scheduled_agent_task::LocalScheduledAgentTaskProvider;
use macaca_scheduler::InProcessSchedulerProvider;
use tracing::info;

use crate::{
    autonomy_runtime_config::{AutonomyProviderMode, AutonomyRuntimeConfig},
    autonomy_supervisor::AutonomyLifecycleCoordinator,
    ServiceProviderFactoryContext, ServiceProviderInstance, ServiceRuntime,
    StaticServiceProviderFactory,
};

use super::heartbeat_adapter::HostHeartbeatServiceAdapter;
use super::scheduled_agent_task_adapter::ScheduledAgentTaskSystemServiceProvider;
use super::scheduler_adapter::HostSchedulerServiceAdapter;
use super::support::runtime_error;

/// Started autonomy service ids returned by runtime-host bootstrap.
#[derive(Clone, Default)]
pub struct AutonomyRuntimeBundle {
    /// Kernel service ids successfully registered and started during bootstrap.
    pub started_services: Vec<KernelServiceId>,
    /// Provider mode label recorded for observability (`unavailable` or `local`).
    pub provider_mode: String,
    /// Optional supervisor handle when local mode enables background autonomy loops.
    pub supervisor: Option<AutonomyLifecycleCoordinator>,
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
        Arc::new(HostSchedulerServiceAdapter::unavailable()),
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
        Arc::new(HostHeartbeatServiceAdapter::unavailable()),
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
    let scheduler = Arc::new(InProcessSchedulerProvider::new());
    let scheduled_agent_task = Arc::new(LocalScheduledAgentTaskProvider::new(scheduler.clone()));
    let heartbeat = Arc::new(InProcessHeartbeatProvider::new());
    let mut bundle = AutonomyRuntimeBundle {
        provider_mode: "local".into(),
        ..AutonomyRuntimeBundle::default()
    };

    register_and_start(
        Arc::clone(&runtime),
        Arc::new(HostSchedulerServiceAdapter::new(scheduler.clone())),
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
        Arc::new(HostHeartbeatServiceAdapter::new(heartbeat.clone())),
        format!("{trace_prefix}-heartbeat-local-service"),
        &mut bundle,
    )
    .await?;

    let supervisor = AutonomyLifecycleCoordinator::new(
        Arc::clone(&runtime),
        scheduler,
        heartbeat,
        config.clone(),
    );
    supervisor
        .start(TraceContext::new(format!(
            "{trace_prefix}-supervisor-start"
        )))
        .await?;
    if config.recovery_wake_enabled {
        supervisor
            .dispatch_recovery_wake(TraceContext::new(format!("{trace_prefix}-recovery-wake")))
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

/// Register one autonomy provider with `ServiceRuntime` and record it in the bundle.
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
