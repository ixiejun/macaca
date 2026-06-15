//! Runtime event bridge for session-visible observability.
//!
//! The bridge is deliberately a Web-shell adapter: it does not decide Skill,
//! MCP, WASM, or service-call semantics.  Callers pass already-sanitized
//! lifecycle facts, and the bridge guarantees the Macaca invariant that
//! EventLog persistence happens before live SSE delivery.

use std::convert::Infallible;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use axum::response::sse::Event;
use macaca_host_composition::runtime_host::{
    FilteredSkill, SkillSnapshot, SkillSnapshotEntry, SkillSourceScope,
};
use macaca_proto::AppendEventCommand;
use macaca_proto::ExecutionControlEvent;
use serde_json::json;
use tokio::sync::{mpsc, RwLock};

use crate::state::AppState;

type SseSender = Arc<RwLock<mpsc::Sender<Result<Event, Infallible>>>>;

/// Emit one session-visible runtime event.
///
/// The payload must already be sanitized by the caller.  This helper only
/// handles transport ordering and indexing metadata: append first, then notify
/// the currently active SSE sender when one exists.
pub(crate) async fn emit_runtime_event(
    state: &Arc<AppState>,
    session_id: &str,
    event_type: &str,
    source: &str,
    agent_name: Option<&str>,
    payload: serde_json::Value,
) {
    let mut command = AppendEventCommand::new(session_id, event_type, source, payload.clone());
    if let Some(agent_name) = agent_name.filter(|name| !name.trim().is_empty()) {
        command = command.with_agent_name(agent_name);
    }
    state.persist.event_log.append_command(command).await;

    if let Some(sse_tx) = active_sse_sender(state, session_id).await {
        let sender = sse_tx.read().await;
        let _ = sender
            .send(Ok(Event::default()
                .event(event_type)
                .data(payload.to_string())))
            .await;
    }
}

async fn active_sse_sender(state: &Arc<AppState>, session_id: &str) -> Option<SseSender> {
    let active_sessions = state.sessions.active_sessions.read().await;
    active_sessions
        .get(session_id)
        .map(|session| Arc::clone(&session.sse_tx))
}

/// Build a bounded payload for one `host_command_results` entry.
///
/// Host command results may contain raw provider output.  The UI only needs to
/// know that a data/service operation progressed, so this Memento view keeps
/// status, trace, metadata identifiers, and a deterministic local hash.
pub(crate) fn build_host_command_result_event_payload(
    result: &serde_json::Value,
) -> serde_json::Value {
    let metadata = result.get("metadata").unwrap_or(&serde_json::Value::Null);
    let trace = result.get("trace").unwrap_or(&serde_json::Value::Null);
    let output_hash = result.get("output").map(stable_json_hash);
    json!({
        "stage": "service_call_result",
        "index": result.get("index").cloned().unwrap_or(serde_json::Value::Null),
        "status": result.get("status").cloned().unwrap_or(serde_json::Value::Null),
        "trace_id": trace.get("trace_id").cloned().unwrap_or(serde_json::Value::Null),
        "service_id": metadata
            .get("service_id")
            .or_else(|| metadata.get("service.id"))
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "operation": metadata
            .get("operation")
            .or_else(|| metadata.get("service.operation"))
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "provider_id": metadata
            .get("provider_id")
            .or_else(|| metadata.get("provider.id"))
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "output_hash": output_hash,
    })
}

/// Emit bounded data-retrieval visibility events from WASM host-command output.
pub(crate) async fn emit_host_command_result_events(
    state: &Arc<AppState>,
    session_id: &str,
    source: &str,
    output: &serde_json::Value,
) {
    let Some(results) = output
        .get("host_command_results")
        .and_then(|value| value.as_array())
    else {
        return;
    };

    for result in results {
        emit_runtime_event(
            state,
            session_id,
            "service_call_audit",
            source,
            None,
            build_host_command_result_event_payload(result),
        )
        .await;
    }
}

/// Emit replayable execution-control events for a session.
///
/// Execution-control state changes are produced by the runtime-host service,
/// not by the Web shell.  This adapter only maps the already-sanitized service
/// event into the session EventLog/SSE channel.  It deliberately reuses
/// `emit_runtime_event` so the durable EventLog append happens before any live
/// UI delivery, preserving the audit-before-diagnostics invariant.
pub(crate) async fn emit_execution_control_events(
    state: &Arc<AppState>,
    session_id: &str,
    source: &str,
    events: &[ExecutionControlEvent],
) {
    for event in events {
        emit_runtime_event(
            state,
            session_id,
            "execution_control",
            source,
            None,
            build_execution_control_event_payload(event),
        )
        .await;
    }
}

/// Build the bounded session-visible payload for one execution-control event.
///
/// The runtime-host service already sanitizes event metadata.  The Web bridge
/// keeps only identifiers, state, reason, trace id, and sanitized metadata so
/// replay has enough evidence without persisting raw prompts, manifests,
/// package bytes, provider payloads, or application-specific business fields.
pub(crate) fn build_execution_control_event_payload(
    event: &ExecutionControlEvent,
) -> serde_json::Value {
    json!({
        "execution_id": event.execution_id,
        "state": format!("{:?}", event.state),
        "reason_code": event.reason_code,
        "trace_id": event.trace.trace_id,
        "metadata": event.metadata,
    })
}

/// Emit a compact skill snapshot lifecycle event for a session.
///
/// Skill snapshots can include prompt text and full instruction bodies.  This
/// adapter applies the Bridge pattern at the Web boundary: it turns that rich
/// service result into a small, replayable event record without teaching Web
/// how to interpret individual skills.
pub(crate) async fn emit_skill_snapshot_event(
    state: &Arc<AppState>,
    session_id: &str,
    agent_name: &str,
    event_type: &str,
    snapshot: &SkillSnapshot,
    trace_id: &str,
) {
    emit_runtime_event(
        state,
        session_id,
        event_type,
        agent_name,
        Some(agent_name),
        build_skill_snapshot_event_payload(snapshot, trace_id),
    )
    .await;
}

/// Build the bounded session-visible payload for a skill snapshot.
///
/// The Memento stored in EventLog intentionally keeps only lifecycle metadata:
/// counts, sources, trace id, and truncation flags.  It must not persist full
/// prompts, skill instruction bodies, package paths, or other raw inputs.
pub(crate) fn build_skill_snapshot_event_payload(
    snapshot: &SkillSnapshot,
    trace_id: &str,
) -> serde_json::Value {
    let mut sources: Vec<_> = snapshot
        .skills
        .iter()
        .map(|skill| skill.source.clone())
        .collect();
    sources.sort();
    sources.dedup();
    json!({
        "agent": snapshot.agent,
        "trace_id": trace_id,
        "skill_count": snapshot.skills.len(),
        "filtered_count": snapshot.filtered.len(),
        "truncated": snapshot.truncated,
        "compact": snapshot.compact,
        "sources": sources,
    })
}

fn stable_json_hash(value: &serde_json::Value) -> String {
    let encoded = serde_json::to_string(value).unwrap_or_default();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    encoded.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_call_audit_event_payload_is_bounded_and_sanitized() {
        let payload = build_host_command_result_event_payload(&json!({
            "index": 0,
            "status": "ok",
            "output": {
                "raw_provider_payload": "do-not-store",
                "items": [1, 2, 3]
            },
            "metadata": {
                "service_id": "service.example",
                "operation": "fetch",
                "provider_id": "provider.example"
            },
            "trace": {
                "trace_id": "trace-data-visible"
            }
        }));

        assert_eq!(payload["stage"], "service_call_result");
        assert_eq!(payload["service_id"], "service.example");
        assert_eq!(payload["operation"], "fetch");
        assert_eq!(payload["provider_id"], "provider.example");
        assert_eq!(payload["trace_id"], "trace-data-visible");
        assert!(payload.get("output").is_none());
        assert!(payload["output_hash"].as_str().is_some());
    }

    #[test]
    fn skill_snapshot_event_payload_is_bounded_and_sanitized() {
        let mut snapshot = SkillSnapshot {
            agent: "agent".into(),
            prompt: "full SKILL.md instructions must never be stored in runtime events".into(),
            skills: vec![SkillSnapshotEntry {
                name: "playwright-mcp".into(),
                description: "Browser".into(),
                source_location: std::path::PathBuf::from("/tmp/SKILL.md"),
                source_base_dir: std::path::PathBuf::from("/tmp"),
                location: std::path::PathBuf::from("/tmp/SKILL.md"),
                base_dir: std::path::PathBuf::from("/tmp"),
                source: "test".into(),
                source_scope: SkillSourceScope::MacacaCentral,
                primary_env: None,
                required_env: Vec::new(),
                install: Vec::new(),
                mcp_servers: Vec::new(),
            }],
            filtered: vec![FilteredSkill {
                name: "hidden".into(),
                reason: "policy".into(),
                source: "test".into(),
            }],
            truncated: true,
            compact: true,
            version: 1,
        };
        snapshot.skills.push(SkillSnapshotEntry {
            name: "duplicate-source".into(),
            description: "Same source".into(),
            source_location: std::path::PathBuf::from("/tmp/OTHER.md"),
            source_base_dir: std::path::PathBuf::from("/tmp"),
            location: std::path::PathBuf::from("/tmp/OTHER.md"),
            base_dir: std::path::PathBuf::from("/tmp"),
            source: "test".into(),
            source_scope: SkillSourceScope::MacacaCentral,
            primary_env: None,
            required_env: Vec::new(),
            install: Vec::new(),
            mcp_servers: Vec::new(),
        });

        let payload = build_skill_snapshot_event_payload(&snapshot, "trace-skill-visible");

        assert_eq!(payload["agent"], "agent");
        assert_eq!(payload["skill_count"], 2);
        assert_eq!(payload["filtered_count"], 1);
        assert_eq!(payload["truncated"], true);
        assert_eq!(payload["compact"], true);
        assert_eq!(payload["trace_id"], "trace-skill-visible");
        assert_eq!(payload["sources"].as_array().unwrap().len(), 1);
        assert!(payload.get("prompt").is_none());
        assert!(payload.get("skills").is_none());
    }

    #[test]
    fn execution_control_event_payload_is_bounded_and_sanitized() {
        let event = ExecutionControlEvent {
            execution_id: "execution-a".into(),
            state: macaca_proto::ExecutionControlState::Paused,
            reason_code: "execution_control.pause_entered".into(),
            trace: macaca_proto::TraceContext::new("trace-execution-control"),
            metadata: std::collections::BTreeMap::from([(
                "safe".to_string(),
                "visible".to_string(),
            )]),
        };

        let payload = build_execution_control_event_payload(&event);

        assert_eq!(payload["execution_id"], "execution-a");
        assert_eq!(payload["state"], "Paused");
        assert_eq!(payload["reason_code"], "execution_control.pause_entered");
        assert_eq!(payload["trace_id"], "trace-execution-control");
        assert_eq!(payload["metadata"]["safe"], "visible");
        assert!(payload.get("prompt").is_none());
        assert!(payload.get("manifest").is_none());
    }
}
