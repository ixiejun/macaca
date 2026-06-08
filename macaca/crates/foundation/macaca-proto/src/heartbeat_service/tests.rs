//! Contract tests for Heartbeat service DTOs and command adapters.
//!
//! Extracted from the monolithic module so provider-neutral wake contracts stay under the OS
//! 500-line constitution while trace and terminal-state invariants remain guarded.

use super::*;

#[test]
fn heartbeat_wake_command_requires_trace_and_scope_key() {
    let command = HeartbeatWakeCommand::new(
        TraceContext::new("trace-heartbeat-wake"),
        AutonomyScope::global(),
        "scope.system.autonomy",
        HeartbeatWakeIntent::ScheduledTick,
    );
    assert!(command.is_ok());

    let mut trace = TraceContext::new("trace-heartbeat-wake");
    trace.trace_id.clear();
    let missing_trace = HeartbeatWakeCommand::new(
        trace,
        AutonomyScope::global(),
        "scope.system.autonomy",
        HeartbeatWakeIntent::ScheduledTick,
    );
    assert!(missing_trace.is_err());

    let missing_scope_key = HeartbeatWakeCommand::new(
        TraceContext::new("trace-heartbeat-wake"),
        AutonomyScope::global(),
        " ",
        HeartbeatWakeIntent::ScheduledTick,
    );
    assert!(missing_scope_key.is_err());
}

#[test]
fn heartbeat_scheduled_tick_helper_builds_provider_neutral_wake() {
    let command = HeartbeatWakeCommand::scheduled_tick(
        TraceContext::new("trace-scheduler-tick"),
        AutonomyScope::global(),
        "scope.scheduler.heartbeat",
    )
    .unwrap();

    assert_eq!(command.intent, HeartbeatWakeIntent::ScheduledTick);
    assert_eq!(command.wake_scope_key, "scope.scheduler.heartbeat");
}

#[test]
fn heartbeat_wake_command_round_trips_through_service_command() {
    let command = HeartbeatWakeCommand::new(
        TraceContext::new("trace-heartbeat-roundtrip"),
        AutonomyScope::global(),
        "scope.system.autonomy",
        HeartbeatWakeIntent::NativeCadence {
            profile_id: HeartbeatProfileId::new("profile.native.1").unwrap(),
        },
    )
    .unwrap();

    let service_command = command.clone().into_service_command().unwrap();
    assert_eq!(service_command.name.as_str(), HEARTBEAT_WAKE_COMMAND);
    assert!(service_command.trace.is_some());

    let decoded: HeartbeatWakeCommand = serde_json::from_value(service_command.payload).unwrap();
    assert_eq!(decoded.wake_scope_key, command.wake_scope_key);
    assert_eq!(decoded.intent, command.intent);
}

#[test]
fn heartbeat_complete_run_command_requires_terminal_state() {
    let run_id = HeartbeatRunId::new("run-terminal-1").unwrap();
    let succeeded = HeartbeatCompleteRunCommand::new(
        TraceContext::new("trace-complete"),
        run_id.clone(),
        HeartbeatRunState::Succeeded,
        "delegated_work_finished",
    );
    assert!(succeeded.is_ok());

    let non_terminal = HeartbeatCompleteRunCommand::new(
        TraceContext::new("trace-complete"),
        run_id,
        HeartbeatRunState::Running,
        "still_running",
    );
    assert!(non_terminal.is_err());
}

#[test]
fn heartbeat_complete_run_round_trips_through_service_command() {
    let command = HeartbeatCompleteRunCommand::new(
        TraceContext::new("trace-complete-roundtrip"),
        HeartbeatRunId::new("run-complete-1").unwrap(),
        HeartbeatRunState::Succeeded,
        "wake_dispatch_succeeded",
    )
    .unwrap();

    let service_command = command.clone().into_service_command().unwrap();
    assert_eq!(
        service_command.name.as_str(),
        HEARTBEAT_COMPLETE_RUN_COMMAND
    );

    let decoded: HeartbeatCompleteRunCommand =
        serde_json::from_value(service_command.payload).unwrap();
    assert_eq!(decoded.run_id, command.run_id);
    assert_eq!(decoded.state, HeartbeatRunState::Succeeded);
}
