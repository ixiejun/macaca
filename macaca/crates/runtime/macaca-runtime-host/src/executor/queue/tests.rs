//! Contract tests for `queue.rs` (extracted to satisfy OS file-size gate).


    use super::*;
    use chrono::Utc;
    use macaca_proto::ApplicationId;

    fn create_test_task(priority: u8) -> DelegatedTask {
        DelegatedTask {
            id: TaskId::new(),
            application_id: ApplicationId::new(),
            from_agent: "entry-agent".into(),
            to_agent: "task-agent".into(),
            prompt: "Test task".into(),
            priority,
            parallel: false,
            created_at: Utc::now(),
            deadline: None,
            parent_task: None,
            context: None,
        }
    }

    #[tokio::test]
    async fn test_enqueue_dequeue_priority() {
        let queue = ExecutionQueue::default();

        // Enqueue tasks with different priorities
        queue.enqueue(create_test_task(1)).await.unwrap();
        queue.enqueue(create_test_task(10)).await.unwrap();
        queue.enqueue(create_test_task(5)).await.unwrap();

        // First dequeue should get priority 10
        let task = queue.dequeue().await.unwrap();
        assert_eq!(task.priority, 10);

        // Second should get priority 5
        let task = queue.dequeue().await.unwrap();
        assert_eq!(task.priority, 5);

        // Third should get priority 1
        let task = queue.dequeue().await.unwrap();
        assert_eq!(task.priority, 1);
    }

    #[tokio::test]
    async fn test_concurrency_limit() {
        let queue = ExecutionQueue::new(2, 10);

        // Enqueue 3 tasks
        queue.enqueue(create_test_task(5)).await.unwrap();
        queue.enqueue(create_test_task(5)).await.unwrap();
        queue.enqueue(create_test_task(5)).await.unwrap();

        // First two should dequeue
        assert!(queue.dequeue().await.is_some());
        assert!(queue.dequeue().await.is_some());

        // Third should not dequeue due to concurrency limit
        assert!(queue.dequeue().await.is_none());
    }

    #[tokio::test]
    async fn test_admit_running_task_respects_concurrency_limit() {
        let queue = ExecutionQueue::new(1, 10);
        queue.admit_running_task(create_test_task(5)).await.unwrap();
        let second = queue.admit_running_task(create_test_task(3)).await;
        assert!(matches!(
            second,
            Err(QueueError::ConcurrencyLimitReached { .. })
        ));
    }

    #[tokio::test]
    async fn test_queue_full() {
        let queue = ExecutionQueue::new(10, 2);

        queue.enqueue(create_test_task(1)).await.unwrap();
        queue.enqueue(create_test_task(1)).await.unwrap();

        // Third should fail
        let result = queue.enqueue(create_test_task(1)).await;
        assert!(result.is_err());
    }
