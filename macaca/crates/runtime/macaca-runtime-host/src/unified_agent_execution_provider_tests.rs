//! Runtime-host integration tests for unified Agent Execution provider (task 2.2.5).
//!
//! These tests exercise `AgentExecutionSystemServiceProvider` directly with a
//! recording backend so YAML, WASM, and chat intents are proven to traverse the
//! same service boundary and produce comparable structured results for audit replay.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    AgentExecutionCommand, AgentExecutionIntent, AgentExecutionResult, ApplicationId,
    ServiceCommand, TraceContext, AGENT_EXECUTE_COMMAND,
};

use crate::agent_execution_service_provider::{
    agent_execution_service_descriptor, AgentExecutionBackend, AgentExecutionSystemServiceProvider,
};

/// Recording backend that captures execution intents for cross-path comparison.
///
/// The mutex is test-only; production backends remain lock-free behind `Arc`.
struct RecordingAgentExecutionBackend {
    recorded: Arc<Mutex<Vec<AgentExecutionIntent>>>,
}

impl RecordingAgentExecutionBackend {
    fn new(recorded: Arc<Mutex<Vec<AgentExecutionIntent>>>) -> Self {
        Self { recorded }
    }
}

#[async_trait]
impl AgentExecutionBackend for RecordingAgentExecutionBackend {
    async fn execute(
        &self,
        command: AgentExecutionCommand,
    ) -> macaca_proto::ServiceResult<AgentExecutionResult> {
        let intent = command.execution_intent.clone();
        self.recorded
            .lock()
            .expect("recording backend mutex poisoned")
            .push(intent);
        Ok(AgentExecutionResult::completed(
            &command,
            serde_json::json!({"output": "recorded"}),
        ))
    }
}

#[tokio::test]
async fn yaml_wasm_and_chat_intents_execute_through_same_service_provider() {
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let backend: Arc<dyn AgentExecutionBackend> = Arc::new(RecordingAgentExecutionBackend::new(
        Arc::clone(&recorded),
    ));
    let provider = AgentExecutionSystemServiceProvider::new(backend);

    let scenarios: [(AgentExecutionIntent, &str); 3] = [
        (AgentExecutionIntent::YamlWorkflowStep, "trace-yaml-workflow"),
        (AgentExecutionIntent::WasmDelegate, "trace-wasm-delegate"),
        (AgentExecutionIntent::ChatMainThread, "trace-chat-main-thread"),
    ];

    for (intent, trace_id) in scenarios {
        let command = AgentExecutionCommand::new(
            ApplicationId::from_name("fixture.application"),
            "session-unified-provider",
            "entry-agent",
            intent,
            "provider-neutral prompt",
            TraceContext::new(trace_id),
        )
        .expect("fixture command should validate");

        let reply = provider
            .call(command.into_service_command().expect("service command"))
            .await
            .expect("provider should accept unified intents");

        assert_eq!(reply.status, "completed");
        let result: AgentExecutionResult =
            serde_json::from_value(reply.output).expect("structured execution result");
        assert_eq!(result.output["output"], "recorded");
        assert_eq!(reply.trace.trace_id, trace_id);
    }

    let recorded = recorded
        .lock()
        .expect("recording backend mutex poisoned")
        .clone();
    assert_eq!(recorded.len(), 3);
    assert!(recorded.contains(&AgentExecutionIntent::YamlWorkflowStep));
    assert!(recorded.contains(&AgentExecutionIntent::WasmDelegate));
    assert!(recorded.contains(&AgentExecutionIntent::ChatMainThread));
}

#[tokio::test]
async fn agent_execution_service_descriptor_advertises_unified_trace_schema() {
    let descriptor = agent_execution_service_descriptor();

    assert_eq!(
        descriptor.trace_schema.as_str(),
        "trace.system_service.agent_execution.v1"
    );
    assert_eq!(
        descriptor.metadata.get("command.agent.execute").map(String::as_str),
        Some(AGENT_EXECUTE_COMMAND)
    );
}

#[tokio::test]
async fn unavailable_provider_rejects_all_session_intents_structurally() {
    let provider = AgentExecutionSystemServiceProvider::unavailable();

    for trace_id in ["trace-yaml-unavailable", "trace-wasm-unavailable"] {
        let err = provider
            .call(ServiceCommand::with_trace(
                macaca_proto::ServiceCommandName::new(AGENT_EXECUTE_COMMAND),
                serde_json::json!({}),
                TraceContext::new(trace_id),
            ))
            .await
            .expect_err("unavailable provider must fail closed");

        assert!(matches!(
            err,
            macaca_proto::ServiceError::ServiceUnavailable(_)
        ));
    }
}
