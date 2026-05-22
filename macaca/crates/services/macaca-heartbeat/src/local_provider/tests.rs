//! Local Heartbeat provider tests.
//!
//! These tests prove native Heartbeat cadence is owned by Heartbeat profiles
//! and does not depend on Scheduler jobs or due-run materialization.

use chrono::{Duration, Utc};
use macaca_proto::{
    AutonomyScope, HeartbeatCadencePolicy, HeartbeatCompleteRunCommand, HeartbeatGateKind,
    HeartbeatProfile, HeartbeatProfileId, HeartbeatRunState, HeartbeatScopeIdentity,
    HeartbeatUpdateProfileCommand, HeartbeatWakeIntent, TraceContext,
};

use crate::service_contract::HeartbeatService;

use super::LocalHeartbeatProvider;

fn trace() -> TraceContext {
    TraceContext::new("trace-native-heartbeat-test")
}

fn due_profile(profile_id: &str, scope_key: &str) -> HeartbeatProfile {
    HeartbeatProfile::new(
        HeartbeatProfileId::new(profile_id).unwrap(),
        HeartbeatScopeIdentity::new(AutonomyScope::global(), scope_key).unwrap(),
        HeartbeatCadencePolicy::FixedInterval {
            interval_ms: 1,
            anchor: Some(Utc::now() - Duration::milliseconds(5)),
        },
    )
    .unwrap()
}

#[tokio::test]
async fn native_heartbeat_profile_ticks_without_scheduler_job() {
    let provider = LocalHeartbeatProvider::new();
    provider
        .register_native_profile(due_profile("profile.native.test", "scope.native.test"))
        .unwrap();

    let results = provider.tick_native_profiles_once(trace()).await.unwrap();

    assert!(results.iter().any(|result| result.accepted));
    let snapshot = provider.snapshot_inner();
    assert!(snapshot.native_profiles_active >= 1);
    assert!(snapshot
        .recent_runs
        .iter()
        .any(|run| matches!(run.intent, HeartbeatWakeIntent::NativeCadence { .. })));
}

#[tokio::test]
async fn native_heartbeat_tick_records_trace_and_audit_memento() {
    let provider = LocalHeartbeatProvider::new();
    provider
        .register_native_profile(due_profile("profile.native.audit", "scope.native.audit"))
        .unwrap();

    provider.tick_native_profiles_once(trace()).await.unwrap();
    let snapshot = provider.snapshot_inner();

    let run = snapshot
        .recent_runs
        .iter()
        .find(|run| run.wake_scope_key == "scope.native.audit")
        .expect("native heartbeat run should be recorded");
    assert_eq!(run.trace_id, "trace-native-heartbeat-test");
    assert!(run
        .audit_id
        .as_deref()
        .unwrap_or_default()
        .starts_with("audit.heartbeat."));
    assert_eq!(run.state, HeartbeatRunState::Running);
    assert!(run.safe_status.contains("dispatch boundary"));
    assert!(snapshot
        .last_audit_ids
        .iter()
        .all(|audit_id| audit_id.starts_with("audit.heartbeat.")));
    assert!(snapshot
        .native_profiles
        .iter()
        .any(|profile| profile.last_run_id.is_some()));
}

#[tokio::test]
async fn heartbeat_completion_command_records_sanitized_dispatch_outcome() {
    let provider = LocalHeartbeatProvider::new();
    provider
        .register_native_profile(due_profile(
            "profile.native.complete",
            "scope.native.complete",
        ))
        .unwrap();

    let results = provider.tick_native_profiles_once(trace()).await.unwrap();
    let run_id = results
        .iter()
        .find(|result| result.accepted)
        .and_then(|result| result.run_id.clone())
        .expect("accepted heartbeat run should expose run id");
    let mut command = HeartbeatCompleteRunCommand::new(
        trace(),
        run_id,
        HeartbeatRunState::Failed,
        "agent_execution_missing_evidence",
    )
    .unwrap();
    command.metadata.insert(
        "agent_execution.completion_policy".into(),
        "require_artifact".into(),
    );
    command
        .metadata
        .insert("raw.prompt".into(), "must not be recorded".into());

    HeartbeatService::complete_run(&provider, command)
        .await
        .unwrap();
    let snapshot = provider.snapshot_inner();
    let run = snapshot
        .recent_runs
        .iter()
        .find(|run| run.wake_scope_key == "scope.native.complete")
        .expect("completed run should stay queryable");

    assert_eq!(run.state, HeartbeatRunState::Failed);
    assert_eq!(
        run.metadata.get("dispatch.reason_code").map(String::as_str),
        Some("agent_execution_missing_evidence")
    );
    assert_eq!(
        run.metadata
            .get("agent_execution.completion_policy")
            .map(String::as_str),
        Some("require_artifact")
    );
    assert!(!run.metadata.contains_key("raw.prompt"));
}

#[tokio::test]
async fn native_heartbeat_tick_respects_cooldown_gate() {
    let provider = LocalHeartbeatProvider::new();
    provider
        .register_native_profile(due_profile(
            "profile.native.cooldown",
            "scope.native.cooldown",
        ))
        .unwrap();

    let first = provider.tick_native_profiles_once(trace()).await.unwrap();
    assert!(first.iter().any(|result| result.accepted));
    let first_run_id = first
        .iter()
        .find(|result| result.accepted)
        .and_then(|result| result.run_id.clone())
        .expect("accepted heartbeat run should expose run id");
    // Native wakes remain pending while Runtime Host observes delegated work.
    // Complete the first run before forcing another due profile so this test
    // verifies the cooldown gate itself instead of duplicate-wake coalescing.
    HeartbeatService::complete_run(
        &provider,
        HeartbeatCompleteRunCommand::new(
            trace(),
            first_run_id,
            HeartbeatRunState::Skipped,
            "test_dispatch_completed",
        )
        .unwrap(),
    )
    .await
    .unwrap();
    provider
        .register_native_profile(due_profile(
            "profile.native.cooldown",
            "scope.native.cooldown",
        ))
        .unwrap();
    let second = provider.tick_native_profiles_once(trace()).await.unwrap();

    assert!(second.iter().any(|result| !result.accepted));
    assert!(second
        .iter()
        .flat_map(|result| result.gates.iter())
        .any(|gate| gate.gate == HeartbeatGateKind::Cooldown && !gate.allowed));
}

#[tokio::test]
async fn native_heartbeat_profile_update_changes_policy_and_records_audit() {
    let provider = LocalHeartbeatProvider::new();
    provider
        .register_native_profile(due_profile("profile.native.edit", "scope.native.edit"))
        .unwrap();

    let mut command = HeartbeatUpdateProfileCommand::new(
        trace(),
        AutonomyScope::global(),
        HeartbeatProfileId::new("profile.native.edit").unwrap(),
        "operator_edit",
    )
    .unwrap()
    .with_fixed_interval_ms(Some(12_000))
    .unwrap();
    command.enabled = Some(false);
    command
        .metadata
        .insert("operator.note".into(), "bounded".into());

    let result = provider.update_profile(command).await.unwrap();
    let profile = result.profile.expect("profile summary should be returned");

    assert!(result.accepted);
    assert!(!profile.enabled);
    assert_eq!(
        profile.metadata.get("operator.note").map(String::as_str),
        Some("bounded")
    );
    assert!(result
        .audit_id
        .as_deref()
        .unwrap_or_default()
        .starts_with("audit.heartbeat.profile.updated."));
}
