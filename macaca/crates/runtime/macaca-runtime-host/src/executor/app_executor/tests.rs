//! Unit tests for application executor worker health and shutdown semantics.
//!
//! Agent names in fixtures are arbitrary test data — they do not encode any
//! application-specific routing or persona behavior.

use std::sync::Arc;
use std::time::Duration;

use crate::executor::{AgentInfo, AgentRunner, ApplicationId, TaskContext, TaskId, TaskResult};

use super::executor::ApplicationExecutor;
use super::types::{ApplicationExecutorConfig, WorkerHealth, WorkerState};

/// Mock AgentRunner for testing supervisor/worker lifecycle without LLM I/O.
struct MockRunner;

#[async_trait::async_trait]
impl AgentRunner for MockRunner {
    async fn execute_agent(
        &self,
        _application_id: &ApplicationId,
        agent_name: &str,
        _prompt: &str,
        _context: Option<TaskContext>,
    ) -> Result<TaskResult, String> {
        tokio::time::sleep(Duration::from_millis(10)).await;
        Ok(TaskResult {
            task_id: TaskId::new(),
            success: true,
            output: format!("{} executed", agent_name),
            error: None,
            artifacts: vec![],
            completed_at: chrono::Utc::now(),
            tokens_used: None,
        })
    }

    async fn list_agents(&self) -> Vec<AgentInfo> {
        vec![AgentInfo {
            id: "test-worker".to_string(),
            name: "worker-agent".to_string(),
            capabilities: vec!["generic".to_string()],
            current_load: 0,
            max_load: 10,
            available: true,
        }]
    }

    async fn agent_exists(&self, agent_name: &str) -> bool {
        agent_name == "worker-agent"
    }
}

fn create_test_executor() -> ApplicationExecutor {
    ApplicationExecutor::new(
        ApplicationId::new(),
        "test-app".to_string(),
        vec![AgentInfo {
            id: "test-worker".to_string(),
            name: "worker-agent".to_string(),
            capabilities: vec!["generic".to_string()],
            current_load: 0,
            max_load: 10,
            available: true,
        }],
        Arc::new(MockRunner),
        ApplicationExecutorConfig::default(),
    )
}

#[test]
fn test_application_executor_config_defaults() {
    let config = ApplicationExecutorConfig::default();
    assert_eq!(config.max_parallel, 4);
    assert_eq!(config.max_queue_size, 100);
    assert!(config.enable_events);
}

#[tokio::test]
async fn test_worker_health_check_healthy() {
    let executor = create_test_executor();

    let health = executor.check_worker_health().await;
    match health {
        WorkerHealth::Healthy { last_heartbeat } => {
            assert!(last_heartbeat < Duration::from_secs(5));
        }
        _ => panic!("Expected worker to be healthy"),
    }
}

#[tokio::test]
async fn test_worker_is_healthy() {
    let executor = create_test_executor();
    assert!(executor.is_worker_healthy().await);
}

#[tokio::test]
async fn test_worker_state_running_after_creation() {
    let executor = create_test_executor();

    let state = executor.worker_state.read().await;
    assert_eq!(*state, WorkerState::Running);
}

#[tokio::test]
async fn test_worker_heartbeat_updates() {
    let executor = create_test_executor();

    let _before = *executor.worker_heartbeat.read().await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    let after = *executor.worker_heartbeat.read().await;

    assert!(after.elapsed() < Duration::from_secs(1));
}

#[tokio::test]
async fn test_worker_graceful_shutdown() {
    let executor = create_test_executor();

    assert!(executor.is_worker_healthy().await);
    executor.shutdown().await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    let state = executor.worker_state.read().await;
    assert_eq!(*state, WorkerState::Shutdown);
}

#[test]
fn test_worker_state_enum_values() {
    assert_eq!(WorkerState::Running, WorkerState::Running);
    assert_eq!(WorkerState::Disconnected, WorkerState::Disconnected);
    assert_eq!(WorkerState::Shutdown, WorkerState::Shutdown);

    assert_ne!(WorkerState::Running, WorkerState::Disconnected);
    assert_ne!(WorkerState::Running, WorkerState::Shutdown);
    assert_ne!(WorkerState::Disconnected, WorkerState::Shutdown);
}
