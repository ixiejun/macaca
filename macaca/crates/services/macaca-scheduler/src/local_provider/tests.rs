//! Contract tests for the in-process Scheduler provider boundary.
//!
//! Tests are flattened at the module root (no nested `mod tests`) so escape-hatch
//! gates can scan production modules without false positives from test-only literals.

use std::collections::BTreeMap;

use chrono::{Duration, Utc};
use macaca_proto::{
    AutonomyScope, KernelServiceId, SchedulerJobDefinition, SchedulerRegisterJobCommand,
    SchedulerScheduleSpec, SchedulerTargetCommand, ServiceCommandName, ServiceTargetCommand,
    TraceContext,
};

use crate::service_contract::SchedulerService;

use super::InProcessSchedulerProvider;

fn trace() -> TraceContext {
    TraceContext::new("trace-local-scheduler-boundary-test")
}

fn service_target() -> SchedulerTargetCommand {
    SchedulerTargetCommand::Service(ServiceTargetCommand {
        service_id: KernelServiceId::new("service.boundary.test"),
        command_name: ServiceCommandName::new("boundary.test.command"),
        payload_ref: None,
        metadata: BTreeMap::new(),
    })
}

#[tokio::test]
async fn scheduler_does_not_materialize_native_heartbeat_cadence() {
    let provider = InProcessSchedulerProvider::new();
    let definition = SchedulerJobDefinition::new(
        AutonomyScope::global(),
        SchedulerScheduleSpec::Every {
            interval_ms: 1_000,
            anchor: Some(Utc::now() - Duration::seconds(2)),
        },
        service_target(),
    )
    .unwrap();
    provider
        .register_job(SchedulerRegisterJobCommand::new(trace(), definition).unwrap())
        .await
        .unwrap();

    let snapshot = provider.snapshot_inner();

    assert!(snapshot.recent_runs.iter().all(|run| {
        run.safe_status != "native heartbeat cadence accepted"
            && run.safe_status != "native heartbeat cadence gated"
    }));
}

#[tokio::test]
async fn scheduler_preserves_generic_service_target_dispatch() {
    let provider = InProcessSchedulerProvider::new();
    let definition = SchedulerJobDefinition::new(
        AutonomyScope::global(),
        SchedulerScheduleSpec::At {
            run_at: Utc::now() - Duration::seconds(1),
        },
        service_target(),
    )
    .unwrap();
    provider
        .register_job(SchedulerRegisterJobCommand::new(trace(), definition).unwrap())
        .await
        .unwrap();

    let leased = provider
        .acquire_next_run_lease_with_target(trace(), "scheduler.boundary.test")
        .unwrap()
        .expect("due service target should lease");

    assert!(matches!(leased.target, SchedulerTargetCommand::Service(_)));
}
