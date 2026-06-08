//! Scheduler lane for the local autonomy supervisor.
//!
//! The lane owns only Scheduler-specific loop work: lease expiry, due-run lease
//! acquisition, and provider-neutral target dispatch through runtime-host
//! strategies. It is deliberately separate from HeartbeatLane so Scheduler can
//! continue operating without becoming the owner of heartbeat cadence.

use std::sync::Arc;

use macaca_proto::{
    KernelServiceId, MacacaResult, RecordScheduledAgentTaskResultCommand, ScheduledAgentTaskId,
    SchedulerJobId, SchedulerRunId, SchedulerTargetCommand, ServiceBusSource, TraceContext,
    SCHEDULED_AGENT_TASK_SERVICE_ID,
};
use macaca_scheduler::InProcessSchedulerProvider;
use tracing::{debug, info, warn};

use crate::autonomy_dispatch::AutonomyDispatchStrategies;
use crate::autonomy_runtime_config::AutonomyRuntimeConfig;
use crate::ServiceRuntime;

/// Runtime-host Strategy object for one bounded Scheduler supervisor tick.
pub(crate) struct SchedulerLane {
    runtime: Arc<ServiceRuntime>,
    scheduler: Arc<InProcessSchedulerProvider>,
    config: AutonomyRuntimeConfig,
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use chrono::{Duration, Utc};
    use macaca_kernel::SystemService;
    use macaca_proto::{
        AgentExecutionCommand, AgentExecutionResult, ApplicationId, AutonomyScope,
        CreateScheduledAgentTaskCommand, ScheduledAgentTaskQueryCommand,
        ScheduledAgentTaskSchedule, ServiceDescriptor, ServiceResult,
    };
    use macaca_scheduled_agent_task::{LocalScheduledAgentTaskProvider, ScheduledAgentTaskService};

    use crate::agent_execution_service_provider::{
        AgentExecutionBackend, AgentExecutionSystemServiceProvider,
    };
    use crate::autonomy_service_provider::ScheduledAgentTaskSystemServiceProvider;
    use crate::{
        ServiceProviderFactoryContext, ServiceProviderInstance, ServiceRuntimeConfig,
        StaticServiceProviderFactory,
    };

    #[derive(Default)]
    struct RecordingAgentExecutionBackend {
        commands: Mutex<Vec<AgentExecutionCommand>>,
    }

    #[async_trait]
    impl AgentExecutionBackend for RecordingAgentExecutionBackend {
        async fn execute(
            &self,
            command: AgentExecutionCommand,
        ) -> ServiceResult<AgentExecutionResult> {
            self.commands.lock().unwrap().push(command.clone());
            let mut result = AgentExecutionResult::completed(
                &command,
                serde_json::json!({"scheduler_lane": "ok"}),
            );
            result.metadata.insert(
                "result_evidence_ref".into(),
                "event/scheduler-result/1".into(),
            );
            Ok(result)
        }
    }

    async fn register_static_service(
        runtime: &ServiceRuntime,
        descriptor: ServiceDescriptor,
        service: Arc<dyn SystemService>,
    ) {
        runtime
            .register_provider(
                &StaticServiceProviderFactory::new(ServiceProviderInstance::new(
                    descriptor, service,
                )),
                ServiceProviderFactoryContext::new(),
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn tick_once_records_scheduled_agent_task_result_summary() {
        let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
        let scheduler = Arc::new(macaca_scheduler::InProcessSchedulerProvider::new());
        let scheduled_provider = Arc::new(LocalScheduledAgentTaskProvider::new(scheduler.clone()));
        let execution_backend = Arc::new(RecordingAgentExecutionBackend::default());
        let scheduled_service = Arc::new(ScheduledAgentTaskSystemServiceProvider::new(
            scheduled_provider.clone(),
        ));
        let execution_service = Arc::new(AgentExecutionSystemServiceProvider::new(
            execution_backend.clone(),
        ));

        register_static_service(&runtime, scheduled_service.descriptor(), scheduled_service).await;
        register_static_service(&runtime, execution_service.descriptor(), execution_service).await;

        let scope =
            AutonomyScope::application(ApplicationId::from_name("scheduled-agent-lane-test"));
        let create = scheduled_provider
            .create_task(
                CreateScheduledAgentTaskCommand::new(
                    TraceContext::new("trace-scheduled-agent-lane-create"),
                    scope.clone(),
                    ScheduledAgentTaskSchedule::At {
                        run_at: Utc::now() - Duration::seconds(1),
                    },
                    "worker",
                    "RAW_PROMPT_SHOULD_NOT_APPEAR_IN_SUMMARY",
                )
                .unwrap(),
            )
            .await
            .unwrap();
        let task_id = create.task_id.clone().unwrap();

        let dispatched = SchedulerLane::new(
            Arc::clone(&runtime),
            scheduler,
            AutonomyRuntimeConfig {
                max_leases_per_tick: 1,
                supervisor_enabled: false,
                ..AutonomyRuntimeConfig::local_manual()
            },
        )
        .tick_once(TraceContext::new("trace-scheduled-agent-lane-tick"))
        .await
        .unwrap();

        assert_eq!(dispatched, 1);
        let commands = execution_backend.commands.lock().unwrap();
        assert_eq!(commands.len(), 1);
        drop(commands);

        let summary = scheduled_provider
            .get_task(
                ScheduledAgentTaskQueryCommand::new(
                    TraceContext::new("trace-scheduled-agent-lane-get"),
                    scope,
                    Some(task_id),
                )
                .unwrap(),
            )
            .await
            .unwrap()
            .unwrap();
        assert!(summary.last_run_id.is_some());
        assert_eq!(summary.last_result_status.as_deref(), Some("succeeded"));
        assert!(!serde_json::to_string(&summary)
            .unwrap()
            .contains("RAW_PROMPT_SHOULD_NOT_APPEAR_IN_SUMMARY"));
    }
}

impl SchedulerLane {
    /// Build a Scheduler lane from approved runtime-host composition inputs.
    pub(crate) fn new(
        runtime: Arc<ServiceRuntime>,
        scheduler: Arc<InProcessSchedulerProvider>,
        config: AutonomyRuntimeConfig,
    ) -> Self {
        Self {
            runtime,
            scheduler,
            config,
        }
    }

    /// Execute one bounded Scheduler tick.
    ///
    /// Scheduler owns due-run state and leases; this lane only coordinates the
    /// host loop and dispatch Strategy. It never evaluates Heartbeat native
    /// cadence and never branches on application-specific semantics.
    pub(crate) async fn tick_once(&self, trace: TraceContext) -> MacacaResult<usize> {
        let mut dispatched = 0usize;
        self.scheduler.expire_leases(trace.clone())?;
        self.scheduler
            .materialize_due_runs_for_dispatch(trace.clone())?;
        for _ in 0..self.config.max_leases_per_tick {
            let Some(leased) = self
                .scheduler
                .acquire_next_run_lease_with_target(trace.clone(), "runtime.autonomy_supervisor")?
            else {
                debug!(
                    trace_id = trace.trace_id.as_str(),
                    "autonomy scheduler lane found no eligible run"
                );
                break;
            };
            self.scheduler
                .mark_run_running(trace.clone(), leased.summary.run_id.clone())?;
            let result_correlation = ResultRecordCorrelation::from_target(
                &leased.target,
                leased.summary.job_id.clone(),
                leased.summary.run_id.clone(),
            );
            let outcome = AutonomyDispatchStrategies::new(
                self.runtime.as_ref(),
                self.config.dispatch_timeout_ms,
            )
            .dispatch(trace.clone(), leased.scope, leased.target)
            .await?;
            let result = if outcome.succeeded {
                self.scheduler
                    .mark_run_succeeded(trace.clone(), leased.summary.run_id)?
            } else if outcome.retryable {
                self.scheduler.mark_run_failed(
                    trace.clone(),
                    leased.summary.run_id,
                    true,
                    outcome.reason_code,
                )?
            } else {
                self.scheduler.cancel_run(
                    trace.clone(),
                    leased.summary.run_id,
                    outcome.reason_code,
                )?
            };
            if let Some(correlation) = result_correlation {
                self.record_scheduled_agent_task_result(
                    trace.clone(),
                    correlation,
                    if outcome.succeeded {
                        "succeeded"
                    } else if outcome.retryable {
                        "failed_retryable"
                    } else {
                        "skipped"
                    },
                    result.audit_id.clone(),
                )
                .await?;
            }
            dispatched += 1;
        }
        info!(
            trace_id = trace.trace_id.as_str(),
            dispatched, "autonomy scheduler lane tick completed"
        );
        Ok(dispatched)
    }

    /// Correlate a terminal Scheduler run back into the owning task summary.
    ///
    /// Scheduler has already recorded the authoritative run transition before
    /// this method is called. Runtime Host only bridges that terminal evidence
    /// back to the Scheduled Agent Task service so user-facing summaries and
    /// replayable audit ids stay current without moving prompt or result
    /// storage into Scheduler.
    async fn record_scheduled_agent_task_result(
        &self,
        trace: TraceContext,
        correlation: ResultRecordCorrelation,
        result_status: &'static str,
        result_audit_id: Option<String>,
    ) -> MacacaResult<()> {
        let command = RecordScheduledAgentTaskResultCommand::new(
            trace.clone(),
            correlation.task_id,
            correlation.run_id,
            result_status,
        )?
        .with_scheduler_job_id(Some(correlation.job_id))
        .with_result_evidence(Some(trace.trace_id.clone()), result_audit_id);
        match self
            .runtime
            .call(
                &KernelServiceId::new(SCHEDULED_AGENT_TASK_SERVICE_ID),
                ServiceBusSource::new("runtime.autonomy_supervisor"),
                command.into_service_command()?,
            )
            .await
        {
            Ok(reply) if reply.success => {
                info!(
                    trace_id = trace.trace_id.as_str(),
                    "scheduled agent task result summary recorded"
                );
            }
            Ok(reply) => {
                warn!(
                    trace_id = trace.trace_id.as_str(),
                    status = reply.status.as_str(),
                    "scheduled agent task result summary record returned unsuccessful reply"
                );
            }
            Err(error) => {
                warn!(
                    trace_id = trace.trace_id.as_str(),
                    error = %error,
                    "scheduled agent task result summary record failed"
                );
            }
        }
        Ok(())
    }
}

/// Safe Scheduler-target correlation required for result-summary recording.
struct ResultRecordCorrelation {
    task_id: ScheduledAgentTaskId,
    job_id: SchedulerJobId,
    run_id: SchedulerRunId,
}

impl ResultRecordCorrelation {
    /// Extract the scheduled-agent-task id from safe target metadata only.
    fn from_target(
        target: &SchedulerTargetCommand,
        job_id: SchedulerJobId,
        run_id: SchedulerRunId,
    ) -> Option<Self> {
        let metadata = match target {
            SchedulerTargetCommand::AgentExecution(command) => &command.metadata,
            _ => return None,
        };
        let task_id = metadata.get("scheduled_agent_task_id")?;
        Some(Self {
            task_id: ScheduledAgentTaskId::new(task_id.clone()).ok()?,
            job_id,
            run_id,
        })
    }
}
