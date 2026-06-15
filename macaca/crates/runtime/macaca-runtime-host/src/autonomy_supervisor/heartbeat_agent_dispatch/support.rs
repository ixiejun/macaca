//! Heartbeat dispatch support — wake metadata extraction and declaration matching.
//!
//! Pure helper functions with no I/O. Keeps [`super::strategy`] focused on
//! ServiceRuntime orchestration while these utilities implement the
//! **Specification** pattern for wake-to-declaration correlation.

use std::collections::BTreeMap;

use macaca_proto::{
    ApplicationHeartbeatAgentView, ApplicationId, HeartbeatCommandResult, MacacaError,
    HEARTBEAT_SERVICE_ID,
};
use uuid::Uuid;

/// Extract application scope from wake metadata when a valid UUID is present.
pub(crate) fn application_id_from_wake(wake: &HeartbeatCommandResult) -> Option<ApplicationId> {
    wake.metadata
        .get("application_id")
        .and_then(|value| Uuid::parse_str(value).ok())
        .map(ApplicationId)
}

/// Resolve session id from wake metadata, falling back to a heartbeat-scoped default.
///
/// The fallback keeps agent execution traceable even when the wake omits an
/// explicit session id — it is derived from the declaration's application id,
/// not from any application-specific naming convention.
pub(crate) fn session_id_from_wake(
    wake: &HeartbeatCommandResult,
    declaration: &ApplicationHeartbeatAgentView,
) -> String {
    wake.metadata
        .get("session_id")
        .cloned()
        .unwrap_or_else(|| format!("heartbeat:{}", declaration.application_id))
}

/// Build dispatch metadata forwarded to Agent Execution with provenance stamps.
///
/// Copies evidence and skill-alias keys from the declaration when present,
/// and stamps heartbeat run/audit identifiers from the accepted wake.
pub(crate) fn dispatch_metadata(
    wake: &HeartbeatCommandResult,
    declaration: &ApplicationHeartbeatAgentView,
) -> BTreeMap<String, String> {
    let mut metadata = BTreeMap::new();
    // Stamp dispatch metadata with the canonical Heartbeat service id from proto DTOs.
    metadata.insert("source".into(), HEARTBEAT_SERVICE_ID.into());
    metadata.insert("execution_intent".into(), "heartbeat".into());
    metadata.insert("profile_id".into(), declaration.profile_id.clone());
    metadata.insert(
        "native_profile_id".into(),
        declaration.native_profile_id.clone(),
    );
    metadata.insert("wake_scope_key".into(), declaration.wake_scope_key.clone());
    if let Some(run_id) = wake.run_id.as_ref() {
        metadata.insert("heartbeat_run_id".into(), run_id.as_str().to_string());
    }
    if let Some(audit_id) = wake.audit_id.as_ref() {
        metadata.insert("heartbeat_audit_id".into(), audit_id.clone());
    }
    for (key, value) in &declaration.metadata {
        if (key.starts_with("evidence.") || key.starts_with("skill.alias."))
            && !value.trim().is_empty()
        {
            metadata.insert(key.clone(), value.clone());
        }
    }
    metadata
}

/// Whether a manifest declaration applies to the given accepted wake.
///
/// Matching prefers explicit profile or scope keys on the wake when present;
/// otherwise all declarations for the application are eligible (caller filters
/// by application id during query).
pub(crate) fn declaration_matches_wake(
    wake: &HeartbeatCommandResult,
    declaration: &ApplicationHeartbeatAgentView,
) -> bool {
    let wake_profile = wake
        .metadata
        .get("native_profile_id")
        .or_else(|| wake.metadata.get("heartbeat.profile_id"));
    let wake_scope = wake.metadata.get("scope_key");
    if let Some(profile_id) = wake_profile {
        return profile_id == &declaration.native_profile_id
            || profile_id == &declaration.profile_id;
    }
    if let Some(scope_key) = wake_scope {
        if scope_key.contains(".agent:") {
            return scope_key == &declaration.wake_scope_key;
        }
    }
    true
}

/// Map execution errors to stable reason codes for summary and audit logs.
pub(crate) fn dispatch_error_reason(error: &MacacaError) -> &'static str {
    let safe = error.to_string();
    if safe.contains("timed out") {
        "agent_execution_timed_out"
    } else if safe.contains("without result evidence") {
        "agent_execution_missing_evidence"
    } else if safe.contains("did not complete") {
        "agent_execution_not_completed"
    } else if safe.contains("returned no result output") {
        "agent_execution_missing_result"
    } else {
        "agent_execution_failed"
    }
}
