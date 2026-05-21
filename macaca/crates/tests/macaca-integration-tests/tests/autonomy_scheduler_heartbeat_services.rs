//! Autonomy Scheduler and Heartbeat service integration gates.
//!
//! These tests keep the new autonomous runtime capabilities behind the approved
//! microkernel boundaries.  Runtime-host owns provider registration, SDK and
//! applications call typed service commands, and local providers expose only
//! sanitized snapshots and audit identifiers.

use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::{Duration, Utc};
use macaca_kernel::MockSystemService;
use macaca_proto::{
    ApplicationId, AutonomyScope, HeartbeatQueryCommand, KernelServiceId,
    SchedulerDeleteJobCommand, SchedulerGetJobCommand, SchedulerJobDefinition,
    SchedulerJobLifecycleCommand, SchedulerJobLifecycleState, SchedulerLifecycleJobCommand,
    SchedulerListJobsCommand, SchedulerMissedRunPolicy, SchedulerQueryCommand, SchedulerRunState,
    SchedulerScheduleSpec, SchedulerServiceSnapshot, SchedulerTargetCommand,
    SchedulerUpdateJobCommand, ServiceBusSource, ServiceCapability, ServiceCommand,
    ServiceCommandName, ServiceDescriptor, ServiceHealth, ServiceScope, ServiceTargetCommand,
    ServiceType, TraceContext, TraceSchemaRef, HEARTBEAT_SERVICE_ID, SCHEDULER_SERVICE_ID,
    SCHEDULER_SNAPSHOT_COMMAND,
};
use macaca_runtime_host::{
    bootstrap_autonomy_services, bootstrap_autonomy_unavailable_services, AutonomyProviderMode,
    AutonomyRuntimeConfig, InMemoryServiceRuntimeEventSink, ServiceProviderFactoryContext,
    ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig, StaticServiceProviderFactory,
};
use macaca_scheduler::{LocalSchedulerProvider, SchedulerService};

fn generic_target() -> SchedulerTargetCommand {
    SchedulerTargetCommand::Service(ServiceTargetCommand {
        service_id: KernelServiceId::new("service.generic"),
        command_name: ServiceCommandName::new("generic.dispatch"),
        payload_ref: None,
        metadata: BTreeMap::new(),
    })
}

fn mock_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new("service.autonomy.mock"),
        ServiceType::new("integration.autonomy.mock"),
        TraceSchemaRef::new("macaca.trace.integration.autonomy.mock"),
    );
    descriptor.health = ServiceHealth::Healthy;
    descriptor.supported_scopes = vec![ServiceScope::Global];
    descriptor.capabilities = vec![ServiceCapability::new(
        macaca_proto::CapabilityId::new("autonomy.mock.echo"),
        "Echo sanitized autonomy dispatch payloads",
    )];
    descriptor
}

async fn register_mock_service(runtime: Arc<ServiceRuntime>) {
    let descriptor = mock_descriptor();
    runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(
                descriptor.clone(),
                Arc::new(MockSystemService::new(descriptor)),
            )),
            ServiceProviderFactoryContext::new(),
        )
        .await
        .unwrap();
    runtime
        .start(
            &KernelServiceId::new("service.autonomy.mock"),
            TraceContext::new("trace-autonomy-mock-start"),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn runtime_host_registers_unavailable_autonomy_services() {
    let events = Arc::new(InMemoryServiceRuntimeEventSink::new());
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig {
        event_sink: Some(events.clone()),
        ..Default::default()
    }));

    let bundle = bootstrap_autonomy_unavailable_services(Arc::clone(&runtime), "trace-autonomy")
        .await
        .unwrap();
    assert!(bundle
        .started_services
        .iter()
        .any(|service_id| service_id.as_str() == SCHEDULER_SERVICE_ID));
    assert!(bundle
        .started_services
        .iter()
        .any(|service_id| service_id.as_str() == HEARTBEAT_SERVICE_ID));

    let trace = TraceContext::new("trace-autonomy-scheduler-snapshot");
    let command = SchedulerQueryCommand {
        trace: trace.clone(),
        scope: AutonomyScope::global(),
        job_id: None,
        run_id: None,
        limit: Some(5),
    };
    let reply = runtime
        .call(
            &KernelServiceId::new(SCHEDULER_SERVICE_ID),
            ServiceBusSource::new("integration.autonomy"),
            ServiceCommand::with_trace(
                ServiceCommandName::new(SCHEDULER_SNAPSHOT_COMMAND),
                serde_json::to_value(command).unwrap(),
                trace,
            ),
        )
        .await
        .unwrap();

    assert!(reply.success);
    assert!(events
        .events()
        .unwrap()
        .iter()
        .any(|event| event.operation == "service_runtime.register.completed"));
}

#[tokio::test]
async fn configured_unavailable_mode_starts_no_supervisor() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let bundle = bootstrap_autonomy_services(
        Arc::clone(&runtime),
        "trace-autonomy-disabled",
        AutonomyRuntimeConfig::default(),
    )
    .await
    .unwrap();

    assert_eq!(bundle.provider_mode, "unavailable");
    assert!(bundle.supervisor.is_none());
    assert!(bundle
        .started_services
        .iter()
        .any(|service_id| service_id.as_str() == SCHEDULER_SERVICE_ID));
    assert!(bundle
        .started_services
        .iter()
        .any(|service_id| service_id.as_str() == HEARTBEAT_SERVICE_ID));
}

#[tokio::test]
async fn configured_local_mode_registers_active_providers_and_manual_supervisor() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let bundle = bootstrap_autonomy_services(
        Arc::clone(&runtime),
        "trace-autonomy-local",
        AutonomyRuntimeConfig {
            provider_mode: AutonomyProviderMode::Local,
            supervisor_enabled: false,
            recovery_wake_enabled: true,
            ..AutonomyRuntimeConfig::local_manual()
        },
    )
    .await
    .unwrap();

    assert_eq!(bundle.provider_mode, "local");
    let supervisor = bundle.supervisor.expect("local mode returns supervisor");
    assert!(!supervisor.is_running());

    let trace = TraceContext::new("trace-autonomy-local-snapshot");
    let reply = runtime
        .call(
            &KernelServiceId::new(SCHEDULER_SERVICE_ID),
            ServiceBusSource::new("integration.autonomy"),
            ServiceCommand::with_trace(
                ServiceCommandName::new(SCHEDULER_SNAPSHOT_COMMAND),
                serde_json::to_value(SchedulerQueryCommand {
                    trace: trace.clone(),
                    scope: AutonomyScope::global(),
                    job_id: None,
                    run_id: None,
                    limit: Some(5),
                })
                .unwrap(),
                trace,
            ),
        )
        .await
        .unwrap();
    let snapshot: SchedulerServiceSnapshot = serde_json::from_value(reply.output.unwrap()).unwrap();
    assert!(snapshot.healthy);
    assert_eq!(snapshot.provider_id, "local.in_memory");
}

#[tokio::test]
async fn supervisor_tick_dispatches_generic_service_command() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    register_mock_service(Arc::clone(&runtime)).await;
    let bundle = bootstrap_autonomy_services(
        Arc::clone(&runtime),
        "trace-autonomy-dispatch",
        AutonomyRuntimeConfig {
            provider_mode: AutonomyProviderMode::Local,
            supervisor_enabled: false,
            recovery_wake_enabled: false,
            max_leases_per_tick: 4,
            ..AutonomyRuntimeConfig::local_manual()
        },
    )
    .await
    .unwrap();
    let supervisor = bundle.supervisor.expect("local mode returns supervisor");
    let trace = TraceContext::new("trace-autonomy-register-dispatch-job");
    let definition = SchedulerJobDefinition::new(
        AutonomyScope::global(),
        SchedulerScheduleSpec::At {
            run_at: Utc::now() - Duration::seconds(1),
        },
        SchedulerTargetCommand::Service(ServiceTargetCommand {
            service_id: KernelServiceId::new("service.autonomy.mock"),
            command_name: ServiceCommandName::new("autonomy.mock.echo"),
            payload_ref: None,
            metadata: BTreeMap::new(),
        }),
    )
    .unwrap();
    runtime
        .call(
            &KernelServiceId::new(SCHEDULER_SERVICE_ID),
            ServiceBusSource::new("integration.autonomy"),
            macaca_proto::SchedulerRegisterJobCommand::new(trace.clone(), definition)
                .unwrap()
                .into_service_command()
                .unwrap(),
        )
        .await
        .unwrap();

    let dispatched = supervisor
        .run_scheduler_tick_once(TraceContext::new("trace-autonomy-manual-tick"))
        .await
        .unwrap();
    assert_eq!(dispatched, 1);

    let snapshot_reply = runtime
        .call(
            &KernelServiceId::new(SCHEDULER_SERVICE_ID),
            ServiceBusSource::new("integration.autonomy"),
            ServiceCommand::with_trace(
                ServiceCommandName::new(SCHEDULER_SNAPSHOT_COMMAND),
                serde_json::to_value(SchedulerQueryCommand {
                    trace: TraceContext::new("trace-autonomy-post-dispatch-snapshot"),
                    scope: AutonomyScope::global(),
                    job_id: None,
                    run_id: None,
                    limit: Some(10),
                })
                .unwrap(),
                TraceContext::new("trace-autonomy-post-dispatch-snapshot"),
            ),
        )
        .await
        .unwrap();
    let snapshot: SchedulerServiceSnapshot =
        serde_json::from_value(snapshot_reply.output.unwrap()).unwrap();
    assert!(snapshot
        .recent_runs
        .iter()
        .any(|run| run.state == SchedulerRunState::Succeeded));
}

#[tokio::test]
async fn supervisor_runs_scheduler_lane_without_heartbeat_cadence() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    register_mock_service(Arc::clone(&runtime)).await;
    let bundle = bootstrap_autonomy_services(
        Arc::clone(&runtime),
        "trace-autonomy-scheduler-lane",
        AutonomyRuntimeConfig {
            provider_mode: AutonomyProviderMode::Local,
            supervisor_enabled: false,
            recovery_wake_enabled: false,
            max_leases_per_tick: 2,
            ..AutonomyRuntimeConfig::local_manual()
        },
    )
    .await
    .unwrap();
    let supervisor = bundle.supervisor.expect("local mode returns supervisor");
    let definition = SchedulerJobDefinition::new(
        AutonomyScope::global(),
        SchedulerScheduleSpec::At {
            run_at: Utc::now() - Duration::seconds(1),
        },
        SchedulerTargetCommand::Service(ServiceTargetCommand {
            service_id: KernelServiceId::new("service.autonomy.mock"),
            command_name: ServiceCommandName::new("autonomy.mock.echo"),
            payload_ref: None,
            metadata: BTreeMap::new(),
        }),
    )
    .unwrap();
    runtime
        .call(
            &KernelServiceId::new(SCHEDULER_SERVICE_ID),
            ServiceBusSource::new("integration.autonomy"),
            macaca_proto::SchedulerRegisterJobCommand::new(
                TraceContext::new("trace-autonomy-scheduler-lane-register"),
                definition,
            )
            .unwrap()
            .into_service_command()
            .unwrap(),
        )
        .await
        .unwrap();

    let dispatched = supervisor
        .run_scheduler_tick_once(TraceContext::new("trace-autonomy-scheduler-lane-tick"))
        .await
        .unwrap();

    assert_eq!(dispatched, 1);
}

#[tokio::test]
async fn supervisor_runs_heartbeat_lane_without_scheduler_due_runs() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let bundle = bootstrap_autonomy_services(
        Arc::clone(&runtime),
        "trace-autonomy-heartbeat-lane",
        AutonomyRuntimeConfig {
            provider_mode: AutonomyProviderMode::Local,
            supervisor_enabled: false,
            recovery_wake_enabled: false,
            ..AutonomyRuntimeConfig::local_manual()
        },
    )
    .await
    .unwrap();
    let supervisor = bundle.supervisor.expect("local mode returns supervisor");

    let accepted = supervisor
        .run_heartbeat_tick_once(TraceContext::new("trace-autonomy-heartbeat-lane-tick"))
        .await
        .unwrap();

    assert!(accepted);
    let trace = TraceContext::new("trace-autonomy-heartbeat-lane-snapshot");
    let reply = runtime
        .call(
            &KernelServiceId::new(HEARTBEAT_SERVICE_ID),
            ServiceBusSource::new("integration.autonomy"),
            ServiceCommand::with_trace(
                ServiceCommandName::new(macaca_proto::HEARTBEAT_SNAPSHOT_COMMAND),
                serde_json::to_value(HeartbeatQueryCommand {
                    trace: trace.clone(),
                    scope: AutonomyScope::global(),
                    run_id: None,
                    wake_scope_key: None,
                    limit: Some(10),
                })
                .unwrap(),
                trace,
            ),
        )
        .await
        .unwrap();
    let snapshot: macaca_proto::HeartbeatServiceSnapshot =
        serde_json::from_value(reply.output.unwrap()).unwrap();
    assert!(snapshot.native_profiles_active >= 1);
    assert!(!snapshot.recent_runs.is_empty());
}

#[tokio::test]
async fn supervisor_reports_structured_lane_degradation() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let bundle = bootstrap_autonomy_services(
        Arc::clone(&runtime),
        "trace-autonomy-lane-degradation",
        AutonomyRuntimeConfig::default(),
    )
    .await
    .unwrap();

    assert!(bundle.supervisor.is_none());
    let trace = TraceContext::new("trace-autonomy-lane-degradation-snapshot");
    let reply = runtime
        .call(
            &KernelServiceId::new(HEARTBEAT_SERVICE_ID),
            ServiceBusSource::new("integration.autonomy"),
            ServiceCommand::with_trace(
                ServiceCommandName::new(macaca_proto::HEARTBEAT_SNAPSHOT_COMMAND),
                serde_json::to_value(HeartbeatQueryCommand {
                    trace: trace.clone(),
                    scope: AutonomyScope::global(),
                    run_id: None,
                    wake_scope_key: None,
                    limit: Some(10),
                })
                .unwrap(),
                trace,
            ),
        )
        .await
        .unwrap();
    let snapshot: macaca_proto::HeartbeatServiceSnapshot =
        serde_json::from_value(reply.output.unwrap()).unwrap();
    assert!(!snapshot.healthy);
    assert_eq!(snapshot.provider_id, "unavailable");
}

#[tokio::test]
async fn supervisor_emits_heartbeat_scheduled_and_recovery_wakes() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let bundle = bootstrap_autonomy_services(
        Arc::clone(&runtime),
        "trace-autonomy-heartbeat",
        AutonomyRuntimeConfig {
            provider_mode: AutonomyProviderMode::Local,
            supervisor_enabled: false,
            recovery_wake_enabled: false,
            ..AutonomyRuntimeConfig::local_manual()
        },
    )
    .await
    .unwrap();
    let supervisor = bundle.supervisor.expect("local mode returns supervisor");

    assert!(supervisor
        .run_heartbeat_tick_once(TraceContext::new("trace-autonomy-heartbeat-tick"))
        .await
        .unwrap());
    let recovery_accepted = supervisor
        .run_recovery_wake_once(TraceContext::new("trace-autonomy-heartbeat-recovery"))
        .await
        .unwrap();
    assert!(!recovery_accepted);

    let trace = TraceContext::new("trace-autonomy-heartbeat-snapshot");
    let reply = runtime
        .call(
            &KernelServiceId::new(HEARTBEAT_SERVICE_ID),
            ServiceBusSource::new("integration.autonomy"),
            ServiceCommand::with_trace(
                ServiceCommandName::new(macaca_proto::HEARTBEAT_SNAPSHOT_COMMAND),
                serde_json::to_value(HeartbeatQueryCommand {
                    trace: trace.clone(),
                    scope: AutonomyScope::global(),
                    run_id: None,
                    wake_scope_key: None,
                    limit: Some(10),
                })
                .unwrap(),
                trace,
            ),
        )
        .await
        .unwrap();
    let snapshot: macaca_proto::HeartbeatServiceSnapshot =
        serde_json::from_value(reply.output.unwrap()).unwrap();
    assert!(!snapshot.recent_runs.is_empty());
}

#[tokio::test]
async fn supervisor_start_and_stop_controls_background_loop() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let bundle = bootstrap_autonomy_services(
        Arc::clone(&runtime),
        "trace-autonomy-background",
        AutonomyRuntimeConfig {
            provider_mode: AutonomyProviderMode::Local,
            supervisor_enabled: true,
            scheduler_tick_interval_ms: 1_000,
            heartbeat_tick_interval_ms: 1_000,
            shutdown_grace_ms: 2_000,
            recovery_wake_enabled: false,
            ..AutonomyRuntimeConfig::local_enabled()
        },
    )
    .await
    .unwrap();
    let supervisor = bundle.supervisor.expect("local mode returns supervisor");
    assert!(supervisor.is_running());

    supervisor
        .stop(TraceContext::new("trace-autonomy-background-stop"))
        .await;
    assert!(!supervisor.is_running());
}

#[tokio::test]
async fn local_scheduler_cron_timezone_snapshot_contains_sanitized_audit() {
    let provider = LocalSchedulerProvider::new();
    let trace = TraceContext::new("trace-local-scheduler-cron");
    let mut definition = SchedulerJobDefinition::new(
        AutonomyScope::global(),
        SchedulerScheduleSpec::CronExpression {
            expression: "*/1 * * * *".into(),
            timezone: "+00:00".into(),
            start_after: Some(Utc::now() - Duration::minutes(3)),
            end_before: None,
        },
        generic_target(),
    )
    .unwrap();
    definition.missed_run_policy = SchedulerMissedRunPolicy::Skip;

    let registered = provider
        .register_job(
            macaca_proto::SchedulerRegisterJobCommand::new(trace.clone(), definition).unwrap(),
        )
        .await
        .unwrap();
    assert!(registered.accepted);
    assert!(registered.audit_id.is_some());

    let snapshot = provider
        .snapshot(SchedulerQueryCommand {
            trace,
            scope: AutonomyScope::global(),
            job_id: None,
            run_id: None,
            limit: Some(5),
        })
        .await
        .unwrap();

    assert!(snapshot.healthy);
    assert!(!snapshot.recent_runs.is_empty());
    assert!(snapshot
        .last_audit_ids
        .iter()
        .all(|audit_id| audit_id.starts_with("audit.scheduler.")));
    assert!(snapshot.recent_runs.iter().all(|run| run
        .audit_id
        .as_deref()
        .unwrap_or("")
        .starts_with("audit.scheduler.")));
}

#[tokio::test]
async fn local_scheduler_job_management_is_application_scoped_and_lifecycle_aware() {
    let provider = LocalSchedulerProvider::new();
    let app_one = ApplicationId(uuid::Uuid::new_v4());
    let app_two = ApplicationId(uuid::Uuid::new_v4());
    let trace = TraceContext::new("trace-local-scheduler-job-management");
    let first = provider
        .register_job(
            macaca_proto::SchedulerRegisterJobCommand::new(
                trace.clone(),
                SchedulerJobDefinition::new(
                    AutonomyScope::application(app_one),
                    SchedulerScheduleSpec::Every {
                        interval_ms: 60_000,
                        anchor: None,
                    },
                    generic_target(),
                )
                .unwrap(),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    provider
        .register_job(
            macaca_proto::SchedulerRegisterJobCommand::new(
                TraceContext::new("trace-local-scheduler-other-app"),
                SchedulerJobDefinition::new(
                    AutonomyScope::application(app_two),
                    SchedulerScheduleSpec::Every {
                        interval_ms: 60_000,
                        anchor: None,
                    },
                    generic_target(),
                )
                .unwrap(),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let job_id = first.job_id.clone().expect("registered job id");

    let app_one_jobs = provider
        .list_jobs(
            SchedulerListJobsCommand::new(
                trace.clone(),
                AutonomyScope::application(app_one),
                Some(10),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(app_one_jobs.len(), 1);
    assert_eq!(app_one_jobs[0].job_id, job_id);

    let mut replacement = SchedulerJobDefinition::new(
        AutonomyScope::application(app_one),
        SchedulerScheduleSpec::Every {
            interval_ms: 120_000,
            anchor: None,
        },
        generic_target(),
    )
    .unwrap();
    replacement
        .metadata
        .insert("schedule.name".into(), "generic test schedule".into());
    let updated = provider
        .update_job(
            SchedulerUpdateJobCommand::new(
                trace.clone(),
                AutonomyScope::application(app_one),
                job_id.clone(),
                replacement,
                "integration_update",
            )
            .unwrap(),
        )
        .await
        .unwrap();
    assert!(updated.accepted);

    let paused = provider
        .pause_job(
            SchedulerLifecycleJobCommand::new(
                trace.clone(),
                AutonomyScope::application(app_one),
                job_id.clone(),
                SchedulerJobLifecycleCommand::Pause,
                "integration_pause",
            )
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(paused.lifecycle, Some(SchedulerJobLifecycleState::Paused));

    let resumed = provider
        .resume_job(
            SchedulerLifecycleJobCommand::new(
                trace.clone(),
                AutonomyScope::application(app_one),
                job_id.clone(),
                SchedulerJobLifecycleCommand::Resume,
                "integration_resume",
            )
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resumed.lifecycle, Some(SchedulerJobLifecycleState::Active));

    let summary = provider
        .get_job(
            SchedulerGetJobCommand::new(
                trace.clone(),
                AutonomyScope::application(app_one),
                job_id.clone(),
            )
            .unwrap(),
        )
        .await
        .unwrap()
        .expect("job summary after update");
    assert_eq!(
        summary.metadata.get("schedule.name").map(String::as_str),
        Some("generic test schedule")
    );

    let deleted = provider
        .delete_job(
            SchedulerDeleteJobCommand::new(
                trace.clone(),
                AutonomyScope::application(app_one),
                job_id.clone(),
                "integration_delete",
            )
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(deleted.lifecycle, Some(SchedulerJobLifecycleState::Deleted));

    let remaining = provider
        .list_jobs(
            SchedulerListJobsCommand::new(trace, AutonomyScope::application(app_one), Some(10))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(remaining.is_empty());
}
