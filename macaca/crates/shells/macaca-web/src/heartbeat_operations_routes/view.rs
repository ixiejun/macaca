//! JSON view helpers for Heartbeat Operations routes.
//!
//! The helpers centralize sanitization so route handlers stay small and the
//! Web shell keeps a single audit-friendly translation point from service DTOs
//! to browser responses.

use std::collections::BTreeSet;

use macaca_proto::{
    HeartbeatProfileMutationResult, HeartbeatProfileSummary, HeartbeatRunSummary,
    HeartbeatServiceSnapshot,
};

pub(super) fn snapshot(
    snapshot: HeartbeatServiceSnapshot,
    scope_keys: &BTreeSet<String>,
) -> serde_json::Value {
    serde_json::json!({
        "service_id": snapshot.service_id,
        "provider_id": snapshot.provider_id,
        "healthy": snapshot.healthy,
        "lifecycle_state": snapshot.lifecycle_state,
        "pending_wakes": snapshot.pending_wakes,
        "active_runs": snapshot.active_runs,
        "scheduler_ticks_active": snapshot.scheduler_ticks_active,
        "native_profiles_active": snapshot.native_profiles_active,
        "recent_runs": runs(snapshot.recent_runs),
        "last_gate_decisions": snapshot.last_gate_decisions,
        "last_audit_ids": snapshot.last_audit_ids,
        "captured_at": snapshot.captured_at,
        "scope_key": scope_keys.iter().next().cloned().unwrap_or_default(),
        "scope_keys": scope_keys,
    })
}

pub(super) fn profiles(
    profiles: Vec<HeartbeatProfileSummary>,
    scope_keys: &BTreeSet<String>,
) -> Vec<serde_json::Value> {
    profiles
        .into_iter()
        .filter(|profile| scope_keys.contains(&profile.scope_key))
        .map(profile)
        .collect()
}

pub(super) fn runs_for_scopes(
    runs: Vec<HeartbeatRunSummary>,
    scope_keys: &BTreeSet<String>,
    limit: usize,
) -> Vec<serde_json::Value> {
    runs.into_iter()
        .filter(|run| scope_keys.contains(&run.wake_scope_key))
        .take(limit)
        .map(run_view)
        .collect()
}

pub(super) fn runs(runs: Vec<HeartbeatRunSummary>) -> Vec<serde_json::Value> {
    runs.into_iter().map(run_view).collect()
}

fn run_view(run: HeartbeatRunSummary) -> serde_json::Value {
    serde_json::json!({
        "run_id": run.run_id.as_str(),
        "wake_scope_key": run.wake_scope_key,
        "intent": run.intent,
        "state": run.state,
        "disposition": run.disposition,
        "requested_at": run.requested_at,
        "started_at": run.started_at,
        "finished_at": run.finished_at,
        "gates": run.gates,
        "trace_id": run.trace_id,
        "audit_id": run.audit_id,
        "safe_status": run.safe_status,
        "metadata": run.metadata,
    })
}

pub(super) fn mutation(result: HeartbeatProfileMutationResult) -> serde_json::Value {
    serde_json::json!({
        "accepted": result.accepted,
        "profile": result.profile.map(profile),
        "trace_id": result.trace.trace_id,
        "audit_id": result.audit_id,
        "error": result.error,
        "metadata": result.metadata,
    })
}

fn profile(profile: HeartbeatProfileSummary) -> serde_json::Value {
    serde_json::json!({
        "profile_id": profile.profile_id.as_str(),
        "scope_key": profile.scope_key,
        "enabled": profile.enabled,
        "fixed_interval_ms": profile.fixed_interval_ms,
        "cooldown_ms": profile.cooldown_ms,
        "next_eligible_at": profile.next_eligible_at,
        "last_tick_at": profile.last_tick_at,
        "last_run_id": profile.last_run_id.as_ref().map(|id| id.as_str()),
        "safe_status": profile.safe_status,
        "metadata": profile.metadata,
    })
}

#[cfg(test)]
mod tests {
    use macaca_proto::{
        AutonomyScope, HeartbeatProfileId, HeartbeatProfileSummary, HeartbeatUpdateProfileCommand,
        TraceContext,
    };

    use super::*;

    #[test]
    fn profiles_filters_to_application_scope_key() {
        let wanted = summary("profile.application.visible", "application:app-1.heartbeat");
        let hidden = summary("profile.application.hidden", "application:app-2.heartbeat");

        let view = profiles(
            vec![wanted, hidden],
            &BTreeSet::from(["application:app-1.heartbeat".into()]),
        );

        assert_eq!(view.len(), 1);
        assert_eq!(view[0]["profile_id"], "profile.application.visible");
        assert_eq!(view[0]["scope_key"], "application:app-1.heartbeat");
    }

    #[test]
    fn mutation_returns_trace_audit_and_sanitized_profile() {
        let trace = TraceContext::new("test-heartbeat-profile-view");
        let result = HeartbeatProfileMutationResult {
            accepted: true,
            profile: Some(summary(
                "profile.application.visible",
                "application:app-1.heartbeat",
            )),
            trace: trace.clone(),
            audit_id: Some("audit.heartbeat.profile.updated.1".into()),
            error: None,
            metadata: Default::default(),
        };

        let view = mutation(result);

        assert_eq!(view["accepted"], true);
        assert_eq!(view["trace_id"], trace.trace_id);
        assert_eq!(view["audit_id"], "audit.heartbeat.profile.updated.1");
        assert_eq!(view["profile"]["profile_id"], "profile.application.visible");
    }

    fn summary(profile_id: &str, scope_key: &str) -> HeartbeatProfileSummary {
        HeartbeatProfileSummary {
            profile_id: HeartbeatProfileId::new(profile_id).unwrap(),
            scope_key: scope_key.into(),
            enabled: true,
            fixed_interval_ms: 60_000,
            cooldown_ms: Some(30_000),
            next_eligible_at: None,
            last_tick_at: None,
            last_run_id: None,
            safe_status: "native heartbeat profile policy updated".into(),
            metadata: Default::default(),
        }
    }

    #[test]
    fn profile_update_command_stays_heartbeat_scoped() {
        let command = HeartbeatUpdateProfileCommand::new(
            TraceContext::new("test-heartbeat-update-command"),
            AutonomyScope::global(),
            HeartbeatProfileId::new("profile.application.visible").unwrap(),
            "test_update",
        )
        .unwrap();

        assert_eq!(command.reason_code, "test_update");
    }
}
