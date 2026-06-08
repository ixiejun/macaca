//! Contract tests for `todo_store.rs` (extracted to satisfy OS file-size gate).


    use super::*;
    use macaca_persist::RedbStore;
    use tempfile::tempdir;

    async fn test_store() -> TodoStore {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.redb");
        // Keep dir alive by leaking (test only)
        let dir = Box::leak(Box::new(dir));
        let _ = dir;
        let store = RedbStore::open(db_path).unwrap();
        TodoStore::new(Arc::new(store))
    }

    #[tokio::test]
    async fn save_and_load_todo() {
        let store = test_store().await;
        let app_id = ApplicationId::new();
        let item = TodoItem::new(
            app_id.clone(),
            None,
            "backend",
            "coordinator",
            "Write API",
            "Create REST API",
            8,
        );
        let task_id = item.id;

        store.save_todo(&item).await;

        let loaded = store.get_todo(&app_id, &None, "backend", &task_id).await;
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.title, "Write API");
        assert_eq!(loaded.priority, 8);
    }

    #[tokio::test]
    async fn list_by_agent_and_status() {
        let store = test_store().await;
        let app_id = ApplicationId::new();

        let item1 = TodoItem::new(
            app_id.clone(),
            None,
            "backend",
            "coord",
            "Task 1",
            "Desc 1",
            5,
        );
        let mut item2 = TodoItem::new(
            app_id.clone(),
            None,
            "backend",
            "coord",
            "Task 2",
            "Desc 2",
            9,
        );
        item2.status = TodoStatus::InProgress;
        let item3 = TodoItem::new(
            app_id.clone(),
            None,
            "frontend",
            "coord",
            "Task 3",
            "Desc 3",
            7,
        );

        store.save_todo(&item1).await;
        store.save_todo(&item2).await;
        store.save_todo(&item3).await;

        let backend_todos = store.list_agent_todos(&app_id, &None, "backend").await;
        assert_eq!(backend_todos.len(), 2);

        let frontend_todos = store.list_agent_todos(&app_id, &None, "frontend").await;
        assert_eq!(frontend_todos.len(), 1);

        let pending = store
            .list_agent_todos_by_status(&app_id, &None, "backend", TodoStatus::Pending)
            .await;
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].title, "Task 1");
    }

    #[tokio::test]
    async fn rollback_in_progress() {
        let store = test_store().await;
        let app_id = ApplicationId::new();

        let mut item = TodoItem::new(
            app_id.clone(),
            None,
            "backend",
            "coord",
            "Running task",
            "Desc",
            5,
        );
        item.status = TodoStatus::InProgress;
        store.save_todo(&item).await;

        store.rollback_in_progress(&app_id).await;

        let loaded = store
            .get_todo(&app_id, &None, "backend", &item.id)
            .await
            .unwrap();
        assert_eq!(loaded.status, TodoStatus::Pending);
    }

    #[tokio::test]
    async fn goal_lifecycle() {
        let store = test_store().await;
        let app_id = ApplicationId::new();

        let goal = TodoGoal::new(app_id.clone(), None, "Build auth system");
        store.save_goal(&goal).await;

        let goals = store.list_goals(&app_id).await;
        assert_eq!(goals.len(), 1);

        let popped = store.pop_pending_goal(&app_id).await;
        assert!(popped.is_some());
        assert_eq!(popped.unwrap().status, TodoGoalStatus::Decomposing);

        // No more pending
        let popped2 = store.pop_pending_goal(&app_id).await;
        assert!(popped2.is_none());
    }

    #[tokio::test]
    async fn session_isolation() {
        let store = test_store().await;
        let app_id = ApplicationId::new();
        let sess_a = Some("session-a".to_string());
        let sess_b = Some("session-b".to_string());

        let item_a = TodoItem::new(
            app_id.clone(),
            sess_a.clone(),
            "backend",
            "coord",
            "Task A",
            "Desc",
            5,
        );
        let item_b = TodoItem::new(
            app_id.clone(),
            sess_b.clone(),
            "backend",
            "coord",
            "Task B",
            "Desc",
            5,
        );

        store.save_todo(&item_a).await;
        store.save_todo(&item_b).await;

        // Session-scoped queries are isolated
        let sess_a_items = store.list_all_todos_for_session(&app_id, "session-a").await;
        assert_eq!(sess_a_items.len(), 1);
        assert_eq!(sess_a_items[0].title, "Task A");

        let sess_b_items = store.list_all_todos_for_session(&app_id, "session-b").await;
        assert_eq!(sess_b_items.len(), 1);
        assert_eq!(sess_b_items[0].title, "Task B");

        // Cross-session query returns both
        let all = store.list_all_todos(&app_id).await;
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn goal_session_isolation() {
        let store = test_store().await;
        let app_id = ApplicationId::new();
        let sess = Some("sess-1".to_string());

        let goal = TodoGoal::new(app_id.clone(), sess.clone(), "Scoped goal");
        store.save_goal(&goal).await;

        let scoped = store.list_goals_for_session(&app_id, "sess-1").await;
        assert_eq!(scoped.len(), 1);

        let all = store.list_goals(&app_id).await;
        assert_eq!(all.len(), 1);
    }
