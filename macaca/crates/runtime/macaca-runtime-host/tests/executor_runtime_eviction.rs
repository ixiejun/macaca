//! Integration tests proving executor orchestration lives in `macaca-runtime-host`
//! after P2.4 microkernel eviction (task 3.4).
//!
//! These tests exercise the **public** runtime-host executor surface exactly as
//! `macaca-web` composition root and `execution_control` coordinators consume it.
//! They intentionally avoid application-specific agent names — fixtures use
//! provider-neutral identifiers only.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use macaca_kernel::UnavailableKernelPersistencePort;
use macaca_proto::{ApplicationId, TaskContext, TaskId};
use macaca_runtime_host::{
    AgentInfo, AgentRunner, ApplicationExecutorRegistry, ExecutorEvent, ExecutorEventFactory,
    ForkManager, TaskResult, TaskStatus, TokenUsage,
};

/// Minimal in-process agent runner used to validate registry wiring without LLM calls.
struct RecordingAgentRunner;

#[async_trait]
impl AgentRunner for RecordingAgentRunner {
    async fn execute_agent(
        &self,
        _application_id: &ApplicationId,
        agent_name: &str,
        prompt: &str,
        _context: Option<TaskContext>,
    ) -> Result<TaskResult, String> {
        Ok(TaskResult {
            task_id: TaskId::new(),
            success: true,
            output: format!("{agent_name}:{prompt}"),
            error: None,
            artifacts: vec![],
            completed_at: Utc::now(),
            tokens_used: Some(TokenUsage {
                prompt_tokens: 1,
                completion_tokens: 1,
                total_tokens: 2,
            }),
        })
    }

    async fn list_agents(&self) -> Vec<AgentInfo> {
        vec![AgentInfo {
            id: "fixture-agent".into(),
            name: "fixture-agent".into(),
            capabilities: vec!["generic".into()],
            current_load: 0,
            max_load: 4,
            available: true,
        }]
    }

    async fn agent_exists(&self, agent_name: &str) -> bool {
        agent_name == "fixture-agent"
    }
}

#[tokio::test]
async fn executor_registry_registers_application_scope() {
    let registry = Arc::new(ApplicationExecutorRegistry::new(Arc::new(
        RecordingAgentRunner,
    )));
    let app_id = ApplicationId::new();

    let executor = registry
        .register_application(
            app_id.clone(),
            "fixture-app".into(),
            vec![AgentInfo {
                id: "fixture-agent".into(),
                name: "fixture-agent".into(),
                capabilities: vec!["generic".into()],
                current_load: 0,
                max_load: 4,
                available: true,
            }],
        )
        .await;

    let resolved = registry.get(&app_id).await.expect("executor registered");
    assert!(Arc::ptr_eq(&executor, &resolved));
}

#[tokio::test]
async fn executor_event_factory_emits_task_lifecycle_variants() {
    let task_id = TaskId::new();
    let factory = ExecutorEventFactory::new(task_id.clone(), "fixture-worker");

    let started = factory.started();
    match started {
        ExecutorEvent::TaskStarted {
            task_id: emitted_id,
            agent,
        } => {
            assert_eq!(emitted_id, task_id);
            assert_eq!(agent, "fixture-worker");
        }
        other => panic!("expected TaskStarted, got {other:?}"),
    }

    let completed = factory.completed("ok");
    match completed {
        ExecutorEvent::TaskCompleted { result, .. } => assert!(result.success),
        other => panic!("expected TaskCompleted, got {other:?}"),
    }
}

#[tokio::test]
async fn fork_manager_accepts_kernel_persistence_port() {
    // ForkManager depends only on kernel persistence/logging ports — not on kernel executor.
    let app_id = ApplicationId::new();
    let persistence = Arc::new(UnavailableKernelPersistencePort);
    let fork_manager =
        ForkManager::new_with_store(Some(persistence), app_id.clone());

    let forks = fork_manager.list_forks(&app_id).await;
    assert!(forks.is_empty());
}

#[test]
fn kernel_crate_has_no_executor_module_export() {
    // Static contract: microkernel must not re-export executor symbols after eviction.
    let kernel_source = include_str!("../../../kernel/macaca-kernel/src/lib.rs");
    assert!(
        !kernel_source.contains("pub mod executor"),
        "kernel lib.rs must not declare pub mod executor"
    );
    assert!(
        !kernel_source.contains("pub use executor"),
        "kernel lib.rs must not re-export executor types"
    );
}

#[test]
fn task_status_enum_remains_available_from_runtime_host() {
    // Shell adapters map executor task status to SSE — verify enum surface is stable.
    assert!(matches!(TaskStatus::Queued, TaskStatus::Queued));
    assert!(matches!(TaskStatus::Running, TaskStatus::Running));
    assert!(matches!(TaskStatus::Completed, TaskStatus::Completed));
    assert!(matches!(TaskStatus::Failed, TaskStatus::Failed));
    assert!(matches!(TaskStatus::Cancelled, TaskStatus::Cancelled));
}
