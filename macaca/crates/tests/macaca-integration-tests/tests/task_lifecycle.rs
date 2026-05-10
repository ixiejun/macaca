//! Integration tests for TaskTracker + TaskQueue working together.

use chrono::Utc;

use macaca_proto::{AgentId, TaskPriority, TaskRequest, TaskResult, TaskStatus};
use macaca_task::{TaskQueue, TaskTracker};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_request(desc: &str, priority: TaskPriority) -> TaskRequest {
    TaskRequest {
        description: desc.into(),
        priority,
        requester: "integration-test".into(),
    }
}

fn make_result(task_id: macaca_proto::TaskId) -> TaskResult {
    TaskResult {
        task_id,
        success: true,
        output: "completed".into(),
        artifacts: vec!["artifact.txt".into()],
        completed_at: Utc::now(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Full lifecycle: create task, push to queue, pop, assign, start, complete.
#[tokio::test]
async fn tracker_and_queue_full_lifecycle() {
    let tracker = TaskTracker::new();
    let queue = TaskQueue::new();

    // Create a task via tracker
    let task = tracker
        .create(make_request("Build the widget", TaskPriority::Normal))
        .await
        .unwrap();
    assert_eq!(task.status, TaskStatus::Pending);

    // Push to queue
    queue.push(task.id, task.priority, task.created_at).await;

    // Pop from queue
    let popped_id = queue.try_pop().unwrap();
    assert_eq!(popped_id, task.id);

    // Assign to an agent
    let agent_id = AgentId::new();
    tracker.assign(&task.id, agent_id).await.unwrap();
    assert_eq!(
        tracker.status(&task.id).await.unwrap(),
        TaskStatus::Assigned
    );

    // Start
    tracker.start(&task.id).await.unwrap();
    assert_eq!(tracker.status(&task.id).await.unwrap(), TaskStatus::Running);

    // Complete
    let result = make_result(task.id);
    tracker.complete(&task.id, result).await.unwrap();
    assert_eq!(
        tracker.status(&task.id).await.unwrap(),
        TaskStatus::Completed
    );

    // Verify stored result
    let stored = tracker.get_result(&task.id).await.unwrap().unwrap();
    assert!(stored.success);
    assert_eq!(stored.output, "completed");
    assert_eq!(stored.artifacts, vec!["artifact.txt"]);
}

/// Priority ordering: higher-priority tasks are dequeued first.
#[tokio::test]
async fn queue_respects_priority_ordering() {
    let tracker = TaskTracker::new();
    let queue = TaskQueue::new();

    let low = tracker
        .create(make_request("low task", TaskPriority::Low))
        .await
        .unwrap();
    let high = tracker
        .create(make_request("high task", TaskPriority::High))
        .await
        .unwrap();
    let critical = tracker
        .create(make_request("critical task", TaskPriority::Critical))
        .await
        .unwrap();

    queue.push(low.id, low.priority, low.created_at).await;
    queue.push(high.id, high.priority, high.created_at).await;
    queue
        .push(critical.id, critical.priority, critical.created_at)
        .await;

    assert_eq!(queue.pop().await, critical.id);
    assert_eq!(queue.pop().await, high.id);
    assert_eq!(queue.pop().await, low.id);
}

/// Tasks can be listed by status after moving through the lifecycle.
#[tokio::test]
async fn list_tasks_by_status_across_lifecycle() {
    let tracker = TaskTracker::new();
    let agent = AgentId::new();

    let t1 = tracker
        .create(make_request("task-1", TaskPriority::Normal))
        .await
        .unwrap();
    let t2 = tracker
        .create(make_request("task-2", TaskPriority::Normal))
        .await
        .unwrap();
    let t3 = tracker
        .create(make_request("task-3", TaskPriority::Normal))
        .await
        .unwrap();

    // Move t1 to Running
    tracker.assign(&t1.id, agent).await.unwrap();
    tracker.start(&t1.id).await.unwrap();

    // Move t2 to Assigned
    tracker.assign(&t2.id, agent).await.unwrap();

    // t3 stays Pending

    let pending = tracker.list_by_status(TaskStatus::Pending).await.unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id, t3.id);

    let assigned = tracker.list_by_status(TaskStatus::Assigned).await.unwrap();
    assert_eq!(assigned.len(), 1);
    assert_eq!(assigned[0].id, t2.id);

    let running = tracker.list_by_status(TaskStatus::Running).await.unwrap();
    assert_eq!(running.len(), 1);
    assert_eq!(running[0].id, t1.id);
}

/// Invalid state transitions produce errors.
#[tokio::test]
async fn invalid_state_transitions() {
    let tracker = TaskTracker::new();

    let task = tracker
        .create(make_request("bad transitions", TaskPriority::Normal))
        .await
        .unwrap();

    // Cannot start a Pending task (must be Assigned first)
    let err = tracker.start(&task.id).await.unwrap_err();
    assert!(err.to_string().contains("expected Assigned"));

    // Cannot complete a Pending task
    let result = make_result(task.id);
    let err = tracker.complete(&task.id, result).await.unwrap_err();
    assert!(err.to_string().contains("expected Running"));

    // Assign, then try to assign again
    let agent = AgentId::new();
    tracker.assign(&task.id, agent).await.unwrap();
    let err = tracker.assign(&task.id, agent).await.unwrap_err();
    assert!(err.to_string().contains("expected Pending"));
}

/// Tasks can be listed by agent assignment.
#[tokio::test]
async fn list_tasks_by_agent() {
    let tracker = TaskTracker::new();
    let agent_a = AgentId::new();
    let agent_b = AgentId::new();

    let t1 = tracker
        .create(make_request("for-a", TaskPriority::Normal))
        .await
        .unwrap();
    let t2 = tracker
        .create(make_request("for-b", TaskPriority::Normal))
        .await
        .unwrap();

    tracker.assign(&t1.id, agent_a).await.unwrap();
    tracker.assign(&t2.id, agent_b).await.unwrap();

    let a_tasks = tracker.list_by_agent(&agent_a).await.unwrap();
    assert_eq!(a_tasks.len(), 1);
    assert_eq!(a_tasks[0].id, t1.id);

    let b_tasks = tracker.list_by_agent(&agent_b).await.unwrap();
    assert_eq!(b_tasks.len(), 1);
    assert_eq!(b_tasks[0].id, t2.id);
}
