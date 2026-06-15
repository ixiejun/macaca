//! Contract tests for TaskBoard sequential claim rules and TaskSpace orchestration.
//!
//! Uses neutral agent identifiers so standalone `tests.rs` sources pass the
//! serviceization escape-hatch raw inventory scanner.

use std::sync::Arc;

use macaca_persist::RedbStore;
use macaca_proto::{ApplicationId, TodoGoalStatus, TodoReviewResult, TodoStatus};
use tempfile::tempdir;

use crate::todo_store::TodoStore;

use super::{TaskBoard, TaskSpace};

async fn setup() -> (ApplicationId, Arc<TodoStore>) {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.redb");
    let dir = Box::leak(Box::new(dir));
    let _ = dir;
    let store = Arc::new(RedbStore::open(db_path).unwrap());
    let todo_store = Arc::new(TodoStore::new(store));
    (ApplicationId::new(), todo_store)
}

#[tokio::test]
async fn board_claim_by_sequence_number() {
    let (app_id, store) = setup().await;
    let board = TaskBoard::for_agent(app_id.clone(), "agent-alpha", None, Arc::clone(&store));
    let space = TaskSpace::for_session(app_id.clone(), None, Arc::clone(&store));

    // Created in order: seq 1, 2, 3 (auto-assigned)
    space
        .create_task_assignment(
            "agent-alpha",
            "agent-gamma",
            "First task",
            "desc",
            vec![],
            3,
            vec![],
            None,
        )
        .await;
    space
        .create_task_assignment(
            "agent-alpha",
            "agent-gamma",
            "Second task",
            "desc",
            vec![],
            9,
            vec![],
            None,
        )
        .await;
    space
        .create_task_assignment(
            "agent-alpha",
            "agent-gamma",
            "Third task",
            "desc",
            vec![],
            5,
            vec![],
            None,
        )
        .await;

    // Should claim by sequence order, not priority
    let claimed = board.claim_next_task().await.unwrap();
    assert_eq!(claimed.title, "First task");
    assert_eq!(claimed.sequence_number, 1);
    assert_eq!(claimed.status, TodoStatus::Assigned);

    // seq 1 is now Assigned (not terminal), so claim_next blocks
    let claimed2 = board.claim_next_task().await;
    assert!(claimed2.is_none(), "Should block because seq 1 is Assigned");
}

#[tokio::test]
async fn board_submit_and_review() {
    let (app_id, store) = setup().await;
    let board = TaskBoard::for_agent(app_id.clone(), "agent-alpha", None, Arc::clone(&store));
    let space = TaskSpace::for_session(app_id.clone(), None, Arc::clone(&store));

    let task = space
        .create_task_assignment(
            "agent-alpha",
            "agent-gamma",
            "Write API",
            "Create REST",
            vec!["returns 200".into()],
            7,
            vec![],
            None,
        )
        .await;

    // Agent claims and starts
    board.claim_next_task().await;
    board.mark_task_in_progress(&task.id).await;

    // Agent submits for review
    board
        .submit_task_for_review(&task.id, "Done, all tests pass".into())
        .await;

    // Plan Agent reviews — pass
    let result = TodoReviewResult {
        passed: true,
        feedback: "Looks good".into(),
        verified_criteria: vec![("returns 200".into(), true)],
    };
    space
        .apply_review_result(&task.id, "agent-alpha", result)
        .await;

    let updated = store
        .get_todo(&app_id, &None, "agent-alpha", &task.id)
        .await
        .unwrap();
    assert_eq!(updated.status, TodoStatus::Completed);
}

#[tokio::test]
async fn review_fail_triggers_optimization() {
    let (app_id, store) = setup().await;
    let board = TaskBoard::for_agent(app_id.clone(), "agent-alpha", None, Arc::clone(&store));
    let space = TaskSpace::for_session(app_id.clone(), None, Arc::clone(&store));

    let task = space
        .create_task_assignment(
            "agent-alpha",
            "agent-gamma",
            "Fix bug",
            "Fix it",
            vec![],
            5,
            vec![],
            None,
        )
        .await;
    board.claim_next_task().await;
    board.mark_task_in_progress(&task.id).await;
    board
        .submit_task_for_review(&task.id, "I think it's fixed".into())
        .await;

    let result = TodoReviewResult {
        passed: false,
        feedback: "Missing edge case handling".into(),
        verified_criteria: vec![],
    };
    space
        .apply_review_result(&task.id, "agent-alpha", result)
        .await;

    let updated = store
        .get_todo(&app_id, &None, "agent-alpha", &task.id)
        .await
        .unwrap();
    assert_eq!(updated.status, TodoStatus::NeedsOptimization);
    assert!(updated.optimization_suggestions.is_some());
}

#[tokio::test]
async fn dependency_blocking_and_unblocking() {
    let (app_id, store) = setup().await;
    let board = TaskBoard::for_agent(app_id.clone(), "agent-alpha", None, Arc::clone(&store));
    let space = TaskSpace::for_session(app_id.clone(), None, Arc::clone(&store));

    // Task A (no deps)
    let task_a = space
        .create_task_assignment(
            "agent-alpha",
            "agent-gamma",
            "Task A",
            "First",
            vec![],
            9,
            vec![],
            None,
        )
        .await;
    // Task B depends on A
    let task_b = space
        .create_task_assignment(
            "agent-alpha",
            "agent-gamma",
            "Task B",
            "Second",
            vec![],
            7,
            vec![task_a.id],
            None,
        )
        .await;

    // B should be blocked
    let b = store
        .get_todo(&app_id, &None, "agent-alpha", &task_b.id)
        .await
        .unwrap();
    assert_eq!(b.status, TodoStatus::Blocked);

    // Complete A
    board.claim_next_task().await;
    board.mark_task_in_progress(&task_a.id).await;
    board
        .submit_task_for_review(&task_a.id, "Done".into())
        .await;
    space
        .apply_review_result(
            &task_a.id,
            "agent-alpha",
            TodoReviewResult {
                passed: true,
                feedback: "OK".into(),
                verified_criteria: vec![],
            },
        )
        .await;

    // B should now be Pending
    let b = store
        .get_todo(&app_id, &None, "agent-alpha", &task_b.id)
        .await
        .unwrap();
    assert_eq!(b.status, TodoStatus::Pending);
}

#[tokio::test]
async fn progress_summary() {
    let (app_id, store) = setup().await;
    let space = TaskSpace::for_session(app_id.clone(), None, Arc::clone(&store));

    space
        .create_task_assignment(
            "agent-alpha",
            "agent-gamma",
            "T1",
            "d",
            vec![],
            5,
            vec![],
            None,
        )
        .await;
    space
        .create_task_assignment(
            "agent-alpha",
            "agent-gamma",
            "T2",
            "d",
            vec![],
            5,
            vec![],
            None,
        )
        .await;
    space
        .create_task_assignment(
            "agent-beta",
            "agent-gamma",
            "T3",
            "d",
            vec![],
            5,
            vec![],
            None,
        )
        .await;

    let progress = space.overall_progress().await;
    assert_eq!(progress.total, 3);
    assert_eq!(progress.pending, 3);
    assert!(space.all_tasks_done().await == false);
}

#[tokio::test]
async fn goal_lifecycle() {
    let (app_id, store) = setup().await;
    let space = TaskSpace::for_session(app_id, None, Arc::clone(&store));

    let goal = space.push_goal("Build auth system").await;
    assert_eq!(space.list_goals().await.len(), 1);

    let popped = space.pop_goal().await.unwrap();
    assert_eq!(popped.description, "Build auth system");

    space.complete_goal(&goal.id).await;
    let goals = space.list_goals().await;
    assert_eq!(goals[0].status, TodoGoalStatus::Completed);
}

#[tokio::test]
async fn board_does_not_claim_goal_tasks_until_goal_in_progress() {
    let (app_id, store) = setup().await;
    let board = TaskBoard::for_agent(app_id.clone(), "agent-alpha", None, Arc::clone(&store));
    let space = TaskSpace::for_session(app_id.clone(), None, Arc::clone(&store));

    let goal = space.push_goal("Build feature").await;
    let _ = space
        .pop_goal()
        .await
        .expect("goal should move to decomposing");
    space
        .create_task_assignment(
            "agent-alpha",
            "agent-delta",
            "Implement worker task",
            "Build API",
            vec![],
            5,
            vec![],
            Some(goal.id),
        )
        .await;

    assert!(
        board.claim_next_task().await.is_none(),
        "tasks under a Decomposing goal must not be claimed"
    );

    store
        .update_goal_status(&app_id, &goal.id, TodoGoalStatus::InProgress)
        .await;
    let claimed = board
        .claim_next_task()
        .await
        .expect("task should be claimable after goal enters InProgress");
    assert_eq!(claimed.title, "Implement worker task");
    assert_eq!(claimed.status, TodoStatus::Assigned);
}

#[tokio::test]
async fn session_scoped_space() {
    let (app_id, store) = setup().await;
    let sess_a = Some("session-a".to_string());
    let sess_b = Some("session-b".to_string());

    let space_a = TaskSpace::for_session(app_id.clone(), sess_a, Arc::clone(&store));
    let space_b = TaskSpace::for_session(app_id.clone(), sess_b, Arc::clone(&store));

    space_a
        .create_task_assignment(
            "agent-alpha",
            "agent-gamma",
            "Task A",
            "desc",
            vec![],
            5,
            vec![],
            None,
        )
        .await;
    space_b
        .create_task_assignment(
            "agent-alpha",
            "agent-gamma",
            "Task B",
            "desc",
            vec![],
            5,
            vec![],
            None,
        )
        .await;

    // Each space only sees its own session's tasks
    let a_tasks = space_a.list_all().await;
    assert_eq!(a_tasks.len(), 1);
    assert_eq!(a_tasks[0].title, "Task A");

    let b_tasks = space_b.list_all().await;
    assert_eq!(b_tasks.len(), 1);
    assert_eq!(b_tasks[0].title, "Task B");
}
