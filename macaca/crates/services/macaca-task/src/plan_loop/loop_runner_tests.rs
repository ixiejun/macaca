//! Review dispatch retry tests for the PlanLoop scheduler.
//!
//! These tests exercise the generic Task Service contract: `PendingReview`
//! dispatches are deduplicated during a backoff window, retried when a persisted
//! review decision is missing, and forgotten once Task Board state changes.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use macaca_persist::RedbStore;
use macaca_proto::{ApplicationId, PlanEvent, TodoReviewResult, TodoStatus};
use tempfile::tempdir;

use crate::todo_board::{TaskBoard, TaskSpace};
use crate::todo_store::TodoStore;

use super::loop_runner::{PlanLoop, ReviewDispatchState};
use super::PlanLoopConfig;

async fn setup_pending_review() -> (
    ApplicationId,
    Arc<TodoStore>,
    Arc<TaskSpace>,
    macaca_proto::TodoItem,
) {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("plan-loop-review-retry.redb");
    let _leaked_dir = Box::leak(Box::new(dir));
    let store = Arc::new(TodoStore::new(Arc::new(RedbStore::open(db_path).unwrap())));
    let app_id = ApplicationId::new();
    let space = Arc::new(TaskSpace::for_session(
        app_id.clone(),
        Some("session-alpha".into()),
        Arc::clone(&store),
    ));
    let board = TaskBoard::for_agent(
        app_id.clone(),
        "agent-alpha",
        Some("session-alpha".into()),
        Arc::clone(&store),
    );
    let task = space
        .create_task_assignment(
            "agent-alpha",
            "agent-planner",
            "Reviewable task",
            "Generic task under review",
            vec!["criterion".into()],
            5,
            vec![],
            None,
        )
        .await;

    board.claim_next_task().await;
    board.mark_task_in_progress(&task.id).await;
    board
        .submit_task_for_review(&task.id, "Ready for generic review".into())
        .await;

    (app_id, store, space, task)
}

fn retry_test_config() -> PlanLoopConfig {
    PlanLoopConfig {
        check_interval: Duration::from_secs(60),
        max_reviews_per_cycle: 10,
        review_retry_backoff: Duration::from_millis(20),
        max_review_dispatch_attempts: 2,
    }
}

#[tokio::test]
async fn pending_review_dispatch_emits_once_initially() {
    let (_app_id, _store, space, _task) = setup_pending_review().await;
    let loop_runner = PlanLoop::with_components(space, retry_test_config());
    let (tx, mut rx) = tokio::sync::mpsc::channel(8);
    let mut dispatches: HashMap<macaca_proto::TaskId, ReviewDispatchState> = HashMap::new();

    loop_runner.emit_pending_reviews(&tx, &mut dispatches).await;
    let event = rx.recv().await.expect("initial review event should emit");

    match event {
        PlanEvent::ReviewNeeded { title, agent, .. } => {
            assert_eq!(title, "Reviewable task");
            assert_eq!(agent, "agent-alpha");
        }
        other => panic!("expected review event, got {other:?}"),
    }
    assert_eq!(dispatches.len(), 1);
}

#[tokio::test]
async fn pending_review_dispatch_suppresses_until_backoff_then_retries() {
    let (_app_id, _store, space, _task) = setup_pending_review().await;
    let loop_runner = PlanLoop::with_components(space, retry_test_config());
    let (tx, mut rx) = tokio::sync::mpsc::channel(8);
    let mut dispatches = HashMap::new();

    loop_runner.emit_pending_reviews(&tx, &mut dispatches).await;
    assert!(matches!(
        rx.recv().await,
        Some(PlanEvent::ReviewNeeded { .. })
    ));

    loop_runner.emit_pending_reviews(&tx, &mut dispatches).await;
    assert!(
        tokio::time::timeout(Duration::from_millis(5), rx.recv())
            .await
            .is_err(),
        "backoff window should suppress immediate duplicate review dispatch"
    );

    tokio::time::sleep(Duration::from_millis(25)).await;
    loop_runner.emit_pending_reviews(&tx, &mut dispatches).await;
    assert!(matches!(
        rx.recv().await,
        Some(PlanEvent::ReviewNeeded { .. })
    ));

    tokio::time::sleep(Duration::from_millis(25)).await;
    loop_runner.emit_pending_reviews(&tx, &mut dispatches).await;
    assert!(
        tokio::time::timeout(Duration::from_millis(5), rx.recv())
            .await
            .is_err(),
        "retry limit should prevent unbounded review storms"
    );
}

#[tokio::test]
async fn pending_review_dispatch_state_clears_after_review_status_changes() {
    let (app_id, store, space, task) = setup_pending_review().await;
    let loop_runner = PlanLoop::with_components(Arc::clone(&space), PlanLoopConfig::default());
    let (tx, mut rx) = tokio::sync::mpsc::channel(8);
    let mut dispatches = HashMap::new();

    loop_runner.emit_pending_reviews(&tx, &mut dispatches).await;
    assert!(matches!(
        rx.recv().await,
        Some(PlanEvent::ReviewNeeded { .. })
    ));
    assert_eq!(dispatches.len(), 1);

    space
        .apply_review_result(
            &task.id,
            "agent-alpha",
            TodoReviewResult {
                passed: true,
                feedback: "Accepted".into(),
                verified_criteria: vec![],
            },
        )
        .await;

    loop_runner.emit_pending_reviews(&tx, &mut dispatches).await;
    assert_eq!(dispatches.len(), 0);

    let completed = store
        .get_todo(
            &app_id,
            &Some("session-alpha".into()),
            "agent-alpha",
            &task.id,
        )
        .await
        .expect("task should remain stored");
    assert_eq!(completed.status, TodoStatus::Completed);
}
