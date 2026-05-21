//! Local Heartbeat provider tests.
//!
//! These tests prove native Heartbeat cadence is owned by Heartbeat profiles
//! and does not depend on Scheduler jobs or due-run materialization.

use chrono::{Duration, Utc};
use macaca_proto::{
    AutonomyScope, HeartbeatCadencePolicy, HeartbeatGateKind, HeartbeatProfile, HeartbeatProfileId,
    HeartbeatScopeIdentity, HeartbeatWakeIntent, TraceContext,
};

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
    assert!(run.safe_status.contains("completed"));
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
