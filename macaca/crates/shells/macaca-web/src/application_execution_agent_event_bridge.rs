//! Bridge Agent Execution progress into application-execution EventLog rows.
//!
//! The Web shell is the composition root where a WASM application delegation
//! can see both sides of the boundary: the generic `service.agent_execution`
//! progress stream and the generic `service.application_execution` gateway.
//! This module is therefore an Observer/Adapter, not an execution owner.  It
//! never interprets application prompts, workflow names, tool business payloads,
//! or provider-specific output.  It mirrors only bounded, sanitized lifecycle
//! facts so application-execution replay remains useful while the browser is
//! disconnected or refreshed.

use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use chrono::Utc;
use macaca_proto::{
    AgentExecutionCommand, AgentExecutionEvent, AgentExecutionIntent, AppendExecutionEventCommand,
    ApplicationExecutionEventEnvelope, ApplicationExecutionEventType, ApplicationExecutionPayload,
    ApplicationExecutionProviderKind, ApplicationExecutionScope,
};
use macaca_sdk::SystemApplicationExecutionClient;
use tracing::{info, warn};

const APPLICATION_EXECUTION_SCHEMA_VERSION: &str = "application-execution.v1";
const INTERNAL_CALLBACK_IDENTITY_REF: &str =
    "identity.internal.application_execution.agent_event_bridge";
const DEFAULT_HOSTED_PROVIDER_ID: &str = "provider.macaca_hosted";

/// Internal event mirror for one delegated agent execution.
///
/// The mirror is intentionally small and cloneable because framework hooks emit
/// progress through a channel.  Each observed event receives a monotonic local
/// ordinal that becomes part of the idempotency key, letting the gateway safely
/// deduplicate retries without relying on browser state or in-memory UI logs.
#[derive(Clone)]
pub(crate) struct ApplicationExecutionAgentEventMirror {
    client: Arc<dyn SystemApplicationExecutionClient>,
    scope: ApplicationExecutionScope,
    provider_id: String,
    provider_kind: ApplicationExecutionProviderKind,
    trace: macaca_proto::TraceContext,
    idempotency_prefix: String,
    next_ordinal: Arc<AtomicU64>,
}

impl ApplicationExecutionAgentEventMirror {
    /// Build a mirror only for agent executions that explicitly carry an
    /// application-execution run id.
    ///
    /// The run id is the protocol join key.  If it is absent, the agent run may
    /// be a normal chat, YAML workflow, task worker, or legacy executor flow; in
    /// those cases mirroring would invent state, so the method returns `None`.
    pub(crate) fn from_command(
        client: Arc<dyn SystemApplicationExecutionClient>,
        command: &AgentExecutionCommand,
    ) -> Option<Self> {
        if !matches!(command.execution_intent, AgentExecutionIntent::WasmDelegate) {
            return None;
        }
        let run_id = command
            .metadata
            .get("application_execution.run_id")
            .filter(|value| !value.trim().is_empty())?
            .clone();
        let provider_id = command
            .metadata
            .get("application_execution.provider_id")
            .filter(|value| !value.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| DEFAULT_HOSTED_PROVIDER_ID.into());
        let provider_kind = parse_provider_kind(
            command
                .metadata
                .get("application_execution.provider_kind")
                .map(String::as_str),
        );
        let scope = ApplicationExecutionScope::new(
            command.application_id,
            command.session_id.clone(),
            run_id.clone(),
            command
                .source_agent
                .clone()
                .unwrap_or_else(|| "service.agent_execution".into()),
        )
        .ok()?;
        info!(
            application_id = %scope.application_id,
            session_id = %scope.session_id,
            run_id = %scope.run_id,
            target_agent = %command.target_agent,
            trace_id = %command.trace.trace_id,
            "agent execution progress mirror attached to application execution run"
        );
        Some(Self {
            client,
            scope,
            provider_id,
            provider_kind,
            trace: command.trace.clone(),
            idempotency_prefix: format!(
                "{}:{}:{}",
                command.session_id,
                run_id,
                command
                    .task_id
                    .map(|task_id| task_id.0.to_string())
                    .unwrap_or_else(|| command.target_agent.clone())
            ),
            next_ordinal: Arc::new(AtomicU64::new(1)),
        })
    }

    /// Persist one sanitized mirror event through the application-execution
    /// gateway.  Errors are logged and swallowed because telemetry mirroring
    /// must not fail the authoritative agent run.
    pub(crate) async fn observe(&self, event: &AgentExecutionEvent) {
        let ordinal = self.next_ordinal.fetch_add(1, Ordering::SeqCst);
        let (event_type, payload, suffix) = self.map_agent_event(event, ordinal);
        let envelope = ApplicationExecutionEventEnvelope {
            application_id: self.scope.application_id,
            session_id: self.scope.session_id.clone(),
            run_id: self.scope.run_id.clone(),
            seq: None,
            timestamp: Utc::now(),
            event_type,
            trace: self.trace.clone(),
            actor: self.scope.actor.clone(),
            provider_id: self.provider_id.clone(),
            provider_kind: self.provider_kind,
            visibility: "session".into(),
            causality: Vec::new(),
            sanitized_payload: payload,
            payload_ref: None,
            schema_version: APPLICATION_EXECUTION_SCHEMA_VERSION.into(),
            idempotency_key: format!("{}:{suffix}", self.idempotency_prefix),
        };
        let append = AppendExecutionEventCommand {
            lease_id: None,
            callback_identity_ref: INTERNAL_CALLBACK_IDENTITY_REF.into(),
            event: envelope,
        };
        match self.client.append_gateway_event(append).await {
            Ok(persisted) => {
                info!(
                    application_id = %persisted.application_id,
                    session_id = %persisted.session_id,
                    run_id = %persisted.run_id,
                    seq = ?persisted.seq,
                    event_type = ?persisted.event_type,
                    "mirrored agent execution progress into application execution EventLog"
                );
            }
            Err(error) => {
                warn!(
                    application_id = %self.scope.application_id,
                    session_id = %self.scope.session_id,
                    run_id = %self.scope.run_id,
                    event_type = ?event_type,
                    reason = %error,
                    "failed to mirror agent execution progress into application execution EventLog"
                );
            }
        }
    }

    fn map_agent_event(
        &self,
        event: &AgentExecutionEvent,
        ordinal: u64,
    ) -> (
        ApplicationExecutionEventType,
        ApplicationExecutionPayload,
        String,
    ) {
        match event {
            AgentExecutionEvent::Thinking { iteration, content } => (
                ApplicationExecutionEventType::ProviderHeartbeat,
                payload(
                    "agent execution progress observed",
                    serde_json::json!({
                        "stage": "thinking",
                        "iteration": iteration,
                        "content_hash": content.as_ref().map(|value| stable_hash(value)),
                        "content_len": content.as_ref().map(|value| value.len()),
                    }),
                ),
                format!("agent-thinking-{ordinal}"),
            ),
            AgentExecutionEvent::ToolCall {
                tool_name,
                tool_input,
                call_id,
            } => (
                ApplicationExecutionEventType::ToolCallRequested,
                payload(
                    "agent execution tool call requested",
                    serde_json::json!({
                        "tool_name": tool_name,
                        "call_id": call_id,
                        "input_hash": stable_json_hash(tool_input),
                    }),
                ),
                format!("agent-tool-call-{ordinal}"),
            ),
            AgentExecutionEvent::ToolResult {
                tool_name,
                output,
                is_error,
            } => (
                ApplicationExecutionEventType::ToolCallCompleted,
                payload(
                    "agent execution tool call completed",
                    serde_json::json!({
                        "tool_name": tool_name,
                        "is_error": is_error.unwrap_or(false),
                        "output_hash": stable_hash(output),
                        "output_len": output.len(),
                    }),
                ),
                format!("agent-tool-result-{ordinal}"),
            ),
            AgentExecutionEvent::Assistant { content } => (
                ApplicationExecutionEventType::LlmCompleted,
                payload(
                    "agent execution assistant message observed",
                    serde_json::json!({
                        "content_hash": stable_hash(content),
                        "content_len": content.len(),
                    }),
                ),
                format!("agent-assistant-{ordinal}"),
            ),
            AgentExecutionEvent::DriverTrace { driver_name, trace } => (
                ApplicationExecutionEventType::ToolCallDispatched,
                payload(
                    "agent execution driver trace observed",
                    serde_json::json!({
                        "driver_name": driver_name,
                        "trace_hash": stable_json_hash(trace),
                        "trace_event_type": trace.get("event_type").and_then(serde_json::Value::as_str),
                    }),
                ),
                format!("agent-driver-trace-{ordinal}"),
            ),
            AgentExecutionEvent::Completed { success, error } => {
                let event_type = if *success {
                    ApplicationExecutionEventType::ExecutionCompleted
                } else {
                    ApplicationExecutionEventType::ExecutionFailed
                };
                (
                    event_type,
                    payload(
                        if *success {
                            "agent execution completed"
                        } else {
                            "agent execution failed"
                        },
                        serde_json::json!({
                            "success": success,
                            "error_hash": error.as_ref().map(|value| stable_hash(value)),
                            "error_len": error.as_ref().map(|value| value.len()),
                        }),
                    ),
                    format!("agent-completed-{ordinal}"),
                )
            }
        }
    }
}

fn payload(summary: &str, data: serde_json::Value) -> ApplicationExecutionPayload {
    let mut payload = ApplicationExecutionPayload::summary(summary);
    payload.data = Some(data);
    payload
}

fn parse_provider_kind(value: Option<&str>) -> ApplicationExecutionProviderKind {
    match value {
        Some("ExternalAppBackend") => ApplicationExecutionProviderKind::ExternalAppBackend,
        Some("RemoteAgent") => ApplicationExecutionProviderKind::RemoteAgent,
        Some("Unavailable") => ApplicationExecutionProviderKind::Unavailable,
        _ => ApplicationExecutionProviderKind::MacacaHosted,
    }
}

fn stable_json_hash(value: &serde_json::Value) -> String {
    serde_json::to_string(value)
        .map(|encoded| stable_hash(&encoded))
        .unwrap_or_else(|_| stable_hash("{}"))
}

fn stable_hash(value: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use macaca_proto::{AgentExecutionCommand, AgentExecutionIntent};
    use macaca_sdk::UnavailableSystemApplicationExecutionClient;

    use super::*;

    #[test]
    fn mirror_attaches_only_to_wasm_delegate_with_application_execution_run_id() {
        let client = Arc::new(UnavailableSystemApplicationExecutionClient::new());
        let mut command = AgentExecutionCommand::new(
            macaca_proto::ApplicationId::from_name("demo"),
            "session-a",
            "coordinator",
            AgentExecutionIntent::WasmDelegate,
            "run app-owned task",
            macaca_proto::TraceContext::new("trace-wasm-delegate"),
        )
        .unwrap();

        assert!(
            ApplicationExecutionAgentEventMirror::from_command(client.clone(), &command).is_none()
        );

        command
            .metadata
            .insert("application_execution.run_id".into(), "run-a".into());
        let mirror = ApplicationExecutionAgentEventMirror::from_command(client, &command)
            .expect("wasm delegate with run id should mirror application execution events");

        assert_eq!(mirror.scope.session_id, "session-a");
        assert_eq!(mirror.scope.run_id, "run-a");
        assert_eq!(mirror.provider_id, DEFAULT_HOSTED_PROVIDER_ID);
    }

    #[test]
    fn mirror_redacts_agent_event_payloads_to_hashes_and_lengths() {
        let client = Arc::new(UnavailableSystemApplicationExecutionClient::new());
        let mut command = AgentExecutionCommand::new(
            macaca_proto::ApplicationId::from_name("demo"),
            "session-a",
            "coordinator",
            AgentExecutionIntent::WasmDelegate,
            "run app-owned task",
            macaca_proto::TraceContext::new("trace-wasm-delegate-redaction"),
        )
        .unwrap();
        command
            .metadata
            .insert("application_execution.run_id".into(), "run-a".into());
        let mirror = ApplicationExecutionAgentEventMirror::from_command(client, &command).unwrap();

        let (event_type, payload, _) = mirror.map_agent_event(
            &AgentExecutionEvent::ToolResult {
                tool_name: "file_write".into(),
                output: "{\"path\":\"shared/main.rs\",\"content\":\"secret\"}".into(),
                is_error: Some(false),
            },
            1,
        );

        assert_eq!(event_type, ApplicationExecutionEventType::ToolCallCompleted);
        let data = payload.data.expect("sanitized payload data");
        assert_eq!(data["tool_name"], "file_write");
        assert!(data.get("output_hash").is_some());
        assert!(data.get("output_len").is_some());
        assert!(
            !data.to_string().contains("secret"),
            "application execution events must not persist raw tool output"
        );
    }
}
