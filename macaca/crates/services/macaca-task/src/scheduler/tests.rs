//! Contract tests for `scheduler.rs` (extracted to satisfy OS file-size gate).


    use super::*;
    use macaca_persist::RedbStore;
    use tempfile::tempdir;

    fn make_store() -> (Arc<RedbStore>, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("sched.redb");
        let store = Arc::new(RedbStore::open(db_path).unwrap());
        (store, dir)
    }

    fn make_scheduler(store: Arc<RedbStore>) -> TaskScheduler {
        TaskScheduler::new(
            store,
            SchedulerConfig {
                check_interval_secs: 60,
            },
        )
    }

    // ── is_due for interval schedules ────────────────────────────────────────

    #[test]
    fn test_interval_schedule_is_due() {
        let app_id = ApplicationId::new();
        let action = ScheduleAction::CreateGoal {
            description: "daily sync".into(),
        };
        let mut entry = ScheduleEntry::new_interval(app_id, "test", 3600, action);

        // Freshly created: next_run_at is now + 3600 s → not due yet.
        assert!(
            !entry.is_due(),
            "should not be due immediately after creation"
        );

        // Simulate that the last run happened more than 3600 s ago.
        entry.last_run_at = Some(Utc::now() - chrono::Duration::seconds(7200));
        entry.compute_next_run();
        // next_run_at = last_run_at + 3600 s = now - 3600 s → past → due.
        assert!(entry.is_due(), "should be due after interval elapsed");
    }

    // ── disabled schedule is never due ───────────────────────────────────────

    #[test]
    fn test_disabled_schedule_not_due() {
        let app_id = ApplicationId::new();
        let action = ScheduleAction::CreateGoal {
            description: "disabled".into(),
        };
        let mut entry = ScheduleEntry::new_interval(app_id, "test", 1, action);

        // Force next_run_at into the past.
        entry.last_run_at = Some(Utc::now() - chrono::Duration::seconds(100));
        entry.compute_next_run();

        entry.enabled = false;
        assert!(!entry.is_due(), "disabled schedule must never be due");
    }

    // ── cron: compute_next_run sets a future next_run_at ────────────────────

    #[test]
    fn test_cron_compute_next_run() {
        let app_id = ApplicationId::new();
        let action = ScheduleAction::CreateTask {
            agent: "agent-alpha".into(),
            title: "Daily report".into(),
            description: "Generate daily report".into(),
            priority: 5,
        };
        // "0 9 * * *" (5-field) → should parse after normalisation.
        let entry = ScheduleEntry::new_cron(app_id, "daily", "0 9 * * *", action);

        assert!(
            entry.next_run_at.is_some(),
            "next_run_at must be computed for a valid cron expression"
        );
        assert!(
            entry.next_run_at.unwrap() > Utc::now(),
            "next_run_at must be in the future"
        );
    }

    // ── CRUD: create → list → get → delete ──────────────────────────────────

    #[tokio::test]
    async fn test_schedule_crud() {
        let (store, _dir) = make_store();
        let scheduler = make_scheduler(Arc::clone(&store));
        let app_id = ApplicationId::new();

        let action = ScheduleAction::CreateGoal {
            description: "cleanup".into(),
        };
        let entry = ScheduleEntry::new_interval(app_id.clone(), "cleanup", 300, action);
        let id = entry.id;

        // Create
        let created = scheduler.create(entry).await;
        assert_eq!(created.id, id);

        // List
        let list = scheduler.list(&app_id).await;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "cleanup");

        // Get
        let fetched = scheduler.get(&app_id, &id).await;
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().interval_secs, Some(300));

        // Delete
        scheduler.delete(&app_id, &id).await;
        let after_delete = scheduler.list(&app_id).await;
        assert!(after_delete.is_empty());
    }

    // ── set_enabled toggles and recomputes ───────────────────────────────────

    #[tokio::test]
    async fn test_set_enabled_toggle() {
        let (store, _dir) = make_store();
        let scheduler = make_scheduler(Arc::clone(&store));
        let app_id = ApplicationId::new();

        let action = ScheduleAction::CreateGoal {
            description: "ping".into(),
        };
        let entry = ScheduleEntry::new_interval(app_id.clone(), "ping", 60, action);
        let id = entry.id;
        scheduler.create(entry).await;

        // Disable
        let ok = scheduler.set_enabled(&app_id, &id, false).await;
        assert!(ok);
        let e = scheduler.get(&app_id, &id).await.unwrap();
        assert!(!e.enabled);

        // Re-enable — next_run_at must be recomputed (not None).
        let ok = scheduler.set_enabled(&app_id, &id, true).await;
        assert!(ok);
        let e = scheduler.get(&app_id, &id).await.unwrap();
        assert!(e.enabled);
        assert!(e.next_run_at.is_some());
    }

    // ── set_enabled returns false for unknown id ─────────────────────────────

    #[tokio::test]
    async fn test_set_enabled_unknown() {
        let (store, _dir) = make_store();
        let scheduler = make_scheduler(store);
        let app_id = ApplicationId::new();
        let unknown_id = TaskId::new();

        let ok = scheduler.set_enabled(&app_id, &unknown_id, false).await;
        assert!(!ok);
    }

    // ── normalise_cron: 5-field → 7-field, 7-field unchanged ────────────────

    #[test]
    fn test_normalise_cron() {
        let five = ScheduleEntry::normalise_cron("0 9 * * *");
        assert_eq!(five.split_whitespace().count(), 7);

        let seven = ScheduleEntry::normalise_cron("0 0 9 * * * *");
        assert_eq!(seven.split_whitespace().count(), 7);
        assert_eq!(seven, "0 0 9 * * * *");
    }
