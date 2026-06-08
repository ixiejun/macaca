//! Test doubles for HeartbeatLane contract tests.
//!
//! Each double implements a generic OS service boundary (Application Service
//! query, Agent Execution backend) without embedding workflow names, prompts, or
//! application-specific routing.  They exist only to observe sanitized command
//! envelopes crossing the service-runtime boundary under test.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;
use macaca_app::application_service_descriptor;
use macaca_kernel::SystemService;
use macaca_proto::{
    AgentExecutionCommand, AgentExecutionResult, ApplicationHeartbeatAgentsResult,
    CleanupPolicy, ServiceCallResult, ServiceCommand, ServiceDescriptor, ServiceError,
    ServiceHealth, ServiceResult, APPLICATION_HEARTBEAT_AGENTS_QUERY_COMMAND,
};

use crate::AgentExecutionBackend;

/// Minimal Application Service test double for heartbeat declaration lookup.
///
/// The fake intentionally speaks only the generic `SystemService` contract
/// so the test exercises the same service-runtime boundary that production
/// Runtime Host uses. It does not embed application names, workflow behavior,
/// or filesystem proof logic.
pub(super) struct FakeApplicationHeartbeatService {
    descriptor: ServiceDescriptor,
    declarations: ApplicationHeartbeatAgentsResult,
}

impl FakeApplicationHeartbeatService {
    /// Bind the double to one sanitized heartbeat declaration list.
    pub(super) fn new(declarations: ApplicationHeartbeatAgentsResult) -> Self {
        Self {
            descriptor: application_service_descriptor(),
            declarations,
        }
    }
}

#[async_trait]
impl SystemService for FakeApplicationHeartbeatService {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)?;
        if command.name.as_str() != APPLICATION_HEARTBEAT_AGENTS_QUERY_COMMAND {
            return Err(ServiceError::UnsupportedCommand(command.name.to_string()));
        }
        Ok(ServiceCallResult {
            status: "ok".into(),
            output: serde_json::to_value(&self.declarations)
                .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?,
            trace,
            metadata: BTreeMap::new(),
            cleanup_hint: Some(CleanupPolicy::None),
        })
    }

    async fn stop(&self) -> ServiceResult<()> {
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(ServiceHealth::Healthy)
    }
}

/// Slow Agent Execution backend used to prove HeartbeatLane does not await
/// long model/tool work inside the supervisor tick path.
#[derive(Default)]
pub(super) struct SlowExecutionBackend {
    pub(super) started: AtomicUsize,
}

#[async_trait]
impl AgentExecutionBackend for SlowExecutionBackend {
    async fn execute(&self, command: AgentExecutionCommand) -> ServiceResult<AgentExecutionResult> {
        self.started.fetch_add(1, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_secs(5)).await;
        let mut result =
            AgentExecutionResult::completed(&command, serde_json::json!({"accepted": true}));
        result.metadata.insert(
            "result_evidence_ref".into(),
            "event/heartbeat-result/1".into(),
        );
        Ok(result)
    }
}

/// Agent Execution backend that captures commands crossing the service boundary.
///
/// The lane under test still calls `service.agent_execution`; this recorder
/// only stores sanitized command envelopes so assertions can prove which
/// manifest agent was dispatched after each native cadence tick.
#[derive(Default)]
pub(super) struct RecordingExecutionBackend {
    pub(super) commands: Mutex<Vec<AgentExecutionCommand>>,
}

#[async_trait]
impl AgentExecutionBackend for RecordingExecutionBackend {
    async fn execute(&self, command: AgentExecutionCommand) -> ServiceResult<AgentExecutionResult> {
        self.commands.lock().unwrap().push(command.clone());
        let mut result =
            AgentExecutionResult::completed(&command, serde_json::json!({"accepted": true}));
        result.metadata.insert(
            "result_evidence_ref".into(),
            "event/heartbeat-result/1".into(),
        );
        Ok(result)
    }
}

/// Agent Execution backend that completes without durable evidence.
///
/// The runtime-host evidence gate should reject this result and the Heartbeat
/// run memento should become failed through the generic completion command.
#[derive(Default)]
pub(super) struct NoEvidenceExecutionBackend;

#[async_trait]
impl AgentExecutionBackend for NoEvidenceExecutionBackend {
    async fn execute(&self, command: AgentExecutionCommand) -> ServiceResult<AgentExecutionResult> {
        Ok(AgentExecutionResult::completed(
            &command,
            serde_json::json!({"accepted": true}),
        ))
    }
}
