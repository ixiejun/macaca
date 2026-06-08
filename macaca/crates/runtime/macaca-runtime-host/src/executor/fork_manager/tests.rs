//! Unit tests for fork lifecycle state machine.
//!
//! Validates create/suspend/resume transitions without binding to any specific
//! application persona — agent names are arbitrary test fixtures only.

use macaca_proto::{AcceptanceCriteria, ApplicationId, ForkState, TaskId};

use super::{DelegateResult, ForkManager};

#[tokio::test]
async fn test_create_fork() {
    let manager = ForkManager::new();
    let app_id = ApplicationId::new();

    let fork_id = manager
        .create_fork(
            None,
            app_id,
            "worker-agent".to_string(),
            "Execute delegated task".to_string(),
            vec![],
            "Generic worker system prompt".to_string(),
            AcceptanceCriteria::default(),
        )
        .await
        .unwrap();

    let fork = manager.get_fork(fork_id).await;
    assert!(fork.is_some());
}

#[tokio::test]
async fn test_suspend_resume_fork() {
    let manager = ForkManager::new();
    let app_id = ApplicationId::new();

    let fork_id = manager
        .create_fork(
            None,
            app_id,
            "worker-agent".to_string(),
            "Execute delegated task".to_string(),
            vec![],
            "Generic worker system prompt".to_string(),
            AcceptanceCriteria::default(),
        )
        .await
        .unwrap();

    // Transition to Running so suspend_fork precondition is satisfied.
    {
        let mut forks = manager.forks.write().await;
        forks.get_mut(&fork_id).unwrap().state = ForkState::Running;
    }

    let task_id = TaskId::new();
    manager.suspend_fork(fork_id, task_id).await.unwrap();

    let fork = manager.get_fork(fork_id).await.unwrap();
    assert!(matches!(fork.state, ForkState::WaitingForHook));

    manager
        .resume_fork(
            fork_id,
            DelegateResult {
                task_id,
                success: true,
                output: "Task completed".to_string(),
                error: None,
                artifacts: vec!["artifact.txt".to_string()],
            },
        )
        .await
        .unwrap();

    let fork = manager.get_fork(fork_id).await.unwrap();
    assert!(matches!(fork.state, ForkState::Running));
}
