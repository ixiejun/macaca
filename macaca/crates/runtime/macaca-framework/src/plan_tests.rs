mod tests {
    use super::super::*;

    // -----------------------------------------------------------------------
    // 1. Create plan + subtasks
    // -----------------------------------------------------------------------
    #[test]
    fn test_create_plan() {
        let mut nb = PlanNotebook::new();
        let plan = nb.create_plan("Fix bug", "Fix the login bug", "Login works");
        assert_eq!(plan.name, "Fix bug");
        assert_eq!(plan.state, PlanState::Todo);
        assert!(plan.subtasks.is_empty());

        let plan = nb.current_plan_mut().unwrap();
        plan.add_subtask("Reproduce", "Reproduce the bug", "Bug reproduced");
        plan.add_subtask("Fix", "Apply the fix", "Bug fixed");

        assert_eq!(nb.current_plan().unwrap().subtasks.len(), 2);
    }

    // -----------------------------------------------------------------------
    // 2. Sequential subtasks — only one InProgress at a time
    // -----------------------------------------------------------------------
    #[test]
    fn test_subtask_sequential() {
        let mut nb = PlanNotebook::new();
        nb.create_plan("Plan", "desc", "outcome");
        let plan = nb.current_plan_mut().unwrap();
        plan.add_subtask("Task A", "Do A", "A done");
        plan.add_subtask("Task B", "Do B", "B done");

        plan.start_subtask(0).unwrap();
        // Starting the second while first is in-progress must fail.
        let err = plan.start_subtask(1);
        assert!(matches!(err, Err(PlanError::AnotherInProgress)));
    }

    // -----------------------------------------------------------------------
    // 3. Finish subtask
    // -----------------------------------------------------------------------
    #[test]
    fn test_finish_subtask() {
        let mut nb = PlanNotebook::new();
        nb.create_plan("Plan", "desc", "outcome");
        let plan = nb.current_plan_mut().unwrap();
        plan.add_subtask("Task A", "Do A", "A done");
        plan.add_subtask("Task B", "Do B", "B done");

        plan.start_subtask(0).unwrap();
        plan.finish_subtask(0, "A completed").unwrap();

        assert_eq!(plan.subtasks[0].state, SubTaskState::Done);
        assert_eq!(plan.subtasks[0].outcome.as_deref(), Some("A completed"));
        // Auto-started next subtask.
        assert_eq!(plan.subtasks[1].state, SubTaskState::InProgress);
    }

    // -----------------------------------------------------------------------
    // 4. Finish entire plan — archived to historical
    // -----------------------------------------------------------------------
    #[test]
    fn test_finish_plan() {
        let mut nb = PlanNotebook::new();
        nb.create_plan("Plan", "desc", "outcome");
        nb.finish_plan("All done").unwrap();

        assert!(nb.current_plan().is_none());
        assert_eq!(nb.historical_plans().len(), 1);
        assert_eq!(nb.historical_plans()[0].state, PlanState::Done);
        assert_eq!(
            nb.historical_plans()[0].outcome.as_deref(),
            Some("All done")
        );
    }

    // -----------------------------------------------------------------------
    // 5. Abandon subtask
    // -----------------------------------------------------------------------
    #[test]
    fn test_abandon_subtask() {
        let mut nb = PlanNotebook::new();
        nb.create_plan("Plan", "desc", "outcome");
        let plan = nb.current_plan_mut().unwrap();
        plan.add_subtask("Task A", "Do A", "A done");
        plan.start_subtask(0).unwrap();
        plan.abandon_subtask(0).unwrap();

        assert_eq!(plan.subtasks[0].state, SubTaskState::Abandoned);
        assert!(plan.subtasks[0].finished_at.is_some());

        // Cannot abandon again (already terminal).
        let err = plan.abandon_subtask(0);
        assert!(matches!(err, Err(PlanError::InvalidTransition { .. })));
    }

    // -----------------------------------------------------------------------
    // 6. Recover historical plan
    // -----------------------------------------------------------------------
    #[test]
    fn test_recover_plan() {
        let mut nb = PlanNotebook::new();
        nb.create_plan("Old Plan", "old desc", "old outcome");
        nb.finish_plan("done").unwrap();

        nb.create_plan("New Plan", "new desc", "new outcome");

        // Recover the old plan (index 0 in historical).
        nb.recover_plan(0).unwrap();

        assert_eq!(nb.current_plan().unwrap().name, "Old Plan");
        // "New Plan" should now be in historical.
        assert_eq!(nb.historical_plans().len(), 1);
        assert_eq!(nb.historical_plans()[0].name, "New Plan");
    }

    // -----------------------------------------------------------------------
    // 7. Hint — no plan
    // -----------------------------------------------------------------------
    #[test]
    fn test_hint_no_plan() {
        let nb = PlanNotebook::new();
        let hint = nb.hint().unwrap();
        let text = hint.get_text();
        assert!(text.contains("no active plan"), "got: {text}");
    }

    // -----------------------------------------------------------------------
    // 8. Hint — subtask in progress
    // -----------------------------------------------------------------------
    #[test]
    fn test_hint_in_progress() {
        let mut nb = PlanNotebook::new();
        nb.create_plan("Plan", "desc", "outcome");
        let plan = nb.current_plan_mut().unwrap();
        plan.add_subtask("Task A", "Do A carefully", "A is done");
        plan.start_subtask(0).unwrap();

        let hint = nb.hint().unwrap();
        let text = hint.get_text();
        assert!(text.contains("Task A"), "got: {text}");
        assert!(text.contains("Do A carefully"), "got: {text}");
        assert!(text.contains("A is done"), "got: {text}");
    }

    // -----------------------------------------------------------------------
    // 9. Hint — all subtasks done
    // -----------------------------------------------------------------------
    #[test]
    fn test_hint_all_done() {
        let mut nb = PlanNotebook::new();
        nb.create_plan("Plan", "desc", "outcome");
        let plan = nb.current_plan_mut().unwrap();
        plan.add_subtask("Task A", "Do A", "A done");
        plan.start_subtask(0).unwrap();
        plan.finish_subtask(0, "completed").unwrap();

        let hint = nb.hint().unwrap();
        let text = hint.get_text();
        assert!(text.contains("All subtasks complete"), "got: {text}");
    }

    // -----------------------------------------------------------------------
    // 10. Serde memento round-trip
    // -----------------------------------------------------------------------
    #[test]
    fn test_state_roundtrip() {
        let mut nb = PlanNotebook::new();
        nb.create_plan("Plan A", "desc A", "outcome A");
        nb.finish_plan("A done").unwrap();
        nb.create_plan("Plan B", "desc B", "outcome B");
        {
            let plan = nb.current_plan_mut().unwrap();
            plan.add_subtask("Task 1", "Do 1", "1 done");
            plan.start_subtask(0).unwrap();
        }

        let state = serde_json::to_value(&nb).expect("serialize plan notebook");

        let restored: PlanNotebook =
            serde_json::from_value(state).expect("deserialize plan notebook");

        // Current plan preserved.
        let cur = restored.current_plan().unwrap();
        assert_eq!(cur.name, "Plan B");
        assert_eq!(cur.subtasks.len(), 1);
        assert_eq!(cur.subtasks[0].state, SubTaskState::InProgress);

        // Historical plan preserved.
        assert_eq!(restored.historical_plans().len(), 1);
        assert_eq!(restored.historical_plans()[0].name, "Plan A");
        assert_eq!(restored.historical_plans()[0].state, PlanState::Done);
    }

    // -----------------------------------------------------------------------
    // 11. Single active invariant — starting second while first runs fails
    // -----------------------------------------------------------------------
    #[test]
    fn test_single_active_invariant() {
        let mut nb = PlanNotebook::new();
        nb.create_plan("Plan", "desc", "outcome");
        let plan = nb.current_plan_mut().unwrap();
        plan.add_subtask("A", "do a", "a done");
        plan.add_subtask("B", "do b", "b done");
        plan.add_subtask("C", "do c", "c done");

        plan.start_subtask(0).unwrap();
        // Trying to start B while A is in progress.
        assert!(matches!(
            plan.start_subtask(1),
            Err(PlanError::AnotherInProgress)
        ));
        assert!(matches!(
            plan.start_subtask(2),
            Err(PlanError::AnotherInProgress)
        ));
        // Only A is InProgress.
        assert_eq!(
            plan.subtasks
                .iter()
                .filter(|s| s.state == SubTaskState::InProgress)
                .count(),
            1
        );
    }

    // -----------------------------------------------------------------------
    // 12. Auto-advance after finishing current subtask
    // -----------------------------------------------------------------------
    #[test]
    fn test_auto_advance_subtask() {
        let mut nb = PlanNotebook::new();
        nb.create_plan("Plan", "desc", "outcome");
        let plan = nb.current_plan_mut().unwrap();
        plan.add_subtask("A", "do a", "a done");
        plan.add_subtask("B", "do b", "b done");
        plan.add_subtask("C", "do c", "c done");

        plan.start_subtask(0).unwrap();
        plan.finish_subtask(0, "completed A").unwrap();

        // B should be auto-advanced to InProgress.
        assert_eq!(plan.subtasks[0].state, SubTaskState::Done);
        assert_eq!(plan.subtasks[1].state, SubTaskState::InProgress);
        assert_eq!(plan.subtasks[2].state, SubTaskState::Todo);

        plan.finish_subtask(1, "completed B").unwrap();
        // C auto-advanced.
        assert_eq!(plan.subtasks[2].state, SubTaskState::InProgress);
    }

    // -----------------------------------------------------------------------
    // 13. All subtasks done completes plan flow
    // -----------------------------------------------------------------------
    #[test]
    fn test_all_subtasks_done_completes_plan() {
        let mut nb = PlanNotebook::new();
        nb.create_plan("Plan", "desc", "outcome");
        let plan = nb.current_plan_mut().unwrap();
        plan.add_subtask("A", "do a", "a done");
        plan.add_subtask("B", "do b", "b done");

        plan.start_subtask(0).unwrap();
        plan.finish_subtask(0, "done A").unwrap();
        plan.finish_subtask(1, "done B").unwrap(); // B was auto-started

        assert!(plan.all_subtasks_finished());
        assert_eq!(plan.subtasks[0].state, SubTaskState::Done);
        assert_eq!(plan.subtasks[1].state, SubTaskState::Done);

        // Hint should say all done.
        let hint_text = nb.hint().unwrap().get_text();
        assert!(
            hint_text.contains("All subtasks complete"),
            "got: {hint_text}"
        );
    }

    // -----------------------------------------------------------------------
    // 14. Illegal state transition: Done → InProgress
    // -----------------------------------------------------------------------
    #[test]
    fn test_illegal_state_transition() {
        let mut nb = PlanNotebook::new();
        nb.create_plan("Plan", "desc", "outcome");
        let plan = nb.current_plan_mut().unwrap();
        plan.add_subtask("A", "do a", "a done");

        plan.start_subtask(0).unwrap();
        plan.finish_subtask(0, "done").unwrap();

        // A is now Done, no other subtask running. Trying to start it again should fail.
        let err = plan.start_subtask(0);
        assert!(matches!(err, Err(PlanError::InvalidTransition { .. })));
    }

    // -----------------------------------------------------------------------
    // 15. Empty plan behavior
    // -----------------------------------------------------------------------
    #[test]
    fn test_empty_plan_behavior() {
        let mut nb = PlanNotebook::new();
        nb.create_plan("Empty Plan", "no subtasks", "nothing");
        let plan = nb.current_plan().unwrap();

        // No subtasks → all_subtasks_finished is false (empty check).
        assert!(!plan.all_subtasks_finished());
        assert!(plan.current_subtask().is_none());
        assert!(plan.next_todo_subtask().is_none());

        // Hint should suggest adding subtasks.
        let hint_text = nb.hint().unwrap().get_text();
        assert!(hint_text.contains("Add subtasks"), "got: {hint_text}");
    }

    // -----------------------------------------------------------------------
    // 16. Abandon subtask, plan continues with others
    // -----------------------------------------------------------------------
    #[test]
    fn test_abandon_subtask_plan_continues() {
        let mut nb = PlanNotebook::new();
        nb.create_plan("Plan", "desc", "outcome");
        let plan = nb.current_plan_mut().unwrap();
        plan.add_subtask("A", "do a", "a done");
        plan.add_subtask("B", "do b", "b done");
        plan.add_subtask("C", "do c", "c done");

        plan.start_subtask(0).unwrap();
        plan.abandon_subtask(0).unwrap();

        assert_eq!(plan.subtasks[0].state, SubTaskState::Abandoned);
        // B and C are still Todo — can start B.
        plan.start_subtask(1).unwrap();
        assert_eq!(plan.subtasks[1].state, SubTaskState::InProgress);
    }

    // -----------------------------------------------------------------------
    // 17. Plan history archive — new plan archives old
    // -----------------------------------------------------------------------
    #[test]
    fn test_plan_history_archive() {
        let mut nb = PlanNotebook::new();
        nb.create_plan("Plan 1", "first", "first outcome");
        // Creating a second plan archives the first.
        nb.create_plan("Plan 2", "second", "second outcome");

        assert_eq!(nb.current_plan().unwrap().name, "Plan 2");
        assert_eq!(nb.historical_plans().len(), 1);
        assert_eq!(nb.historical_plans()[0].name, "Plan 1");

        // Creating a third archives the second.
        nb.create_plan("Plan 3", "third", "third outcome");
        assert_eq!(nb.current_plan().unwrap().name, "Plan 3");
        assert_eq!(nb.historical_plans().len(), 2);
        assert_eq!(nb.historical_plans()[0].name, "Plan 1");
        assert_eq!(nb.historical_plans()[1].name, "Plan 2");
    }

    // -----------------------------------------------------------------------
    // 18. Hint output varies by state
    // -----------------------------------------------------------------------
    #[test]
    fn test_hint_output_varies_by_state() {
        let mut nb = PlanNotebook::new();

        // No plan.
        let h1 = nb.hint().unwrap().get_text();
        assert!(h1.contains("no active plan"), "got: {h1}");

        // Plan with subtasks but none started (Todo state).
        nb.create_plan("Plan", "desc", "outcome");
        let plan = nb.current_plan_mut().unwrap();
        plan.add_subtask("Task X", "do X", "X done");
        let h2 = nb.hint().unwrap().get_text();
        assert!(h2.contains("Start the first one"), "got: {h2}");

        // InProgress.
        nb.current_plan_mut().unwrap().start_subtask(0).unwrap();
        let h3 = nb.hint().unwrap().get_text();
        assert!(h3.contains("Task X"), "got: {h3}");
        assert!(h3.contains("Working on subtask"), "got: {h3}");

        // Done.
        nb.current_plan_mut()
            .unwrap()
            .finish_subtask(0, "ok")
            .unwrap();
        let h4 = nb.hint().unwrap().get_text();
        assert!(h4.contains("All subtasks complete"), "got: {h4}");

        // All hints are different.
        assert_ne!(h1, h2);
        assert_ne!(h2, h3);
        assert_ne!(h3, h4);
    }

    // -----------------------------------------------------------------------
    // 19. PlanNotebook state_dict/load_state_dict with historical plans
    // -----------------------------------------------------------------------
    #[test]
    fn test_plan_serde_memento_roundtrip() {
        let mut nb = PlanNotebook::new();

        // Create & finish plan 1.
        nb.create_plan("Plan Alpha", "alpha desc", "alpha outcome");
        {
            let p = nb.current_plan_mut().unwrap();
            p.add_subtask("A1", "desc", "outcome");
            p.start_subtask(0).unwrap();
            p.finish_subtask(0, "done").unwrap();
        }
        nb.finish_plan("Alpha completed").unwrap();

        // Create plan 2 with an in-progress subtask.
        nb.create_plan("Plan Beta", "beta desc", "beta outcome");
        {
            let p = nb.current_plan_mut().unwrap();
            p.add_subtask("B1", "desc b1", "outcome b1");
            p.add_subtask("B2", "desc b2", "outcome b2");
            p.start_subtask(0).unwrap();
        }

        // Save and restore through the serde memento shape.
        let state = serde_json::to_value(&nb).expect("serialize plan notebook");
        let restored: PlanNotebook =
            serde_json::from_value(state).expect("deserialize plan notebook");

        // Current plan preserved.
        let cur = restored.current_plan().unwrap();
        assert_eq!(cur.name, "Plan Beta");
        assert_eq!(cur.subtasks.len(), 2);
        assert_eq!(cur.subtasks[0].state, SubTaskState::InProgress);
        assert_eq!(cur.subtasks[1].state, SubTaskState::Todo);

        // Historical plan preserved.
        assert_eq!(restored.historical_plans().len(), 1);
        assert_eq!(restored.historical_plans()[0].name, "Plan Alpha");
        assert_eq!(restored.historical_plans()[0].state, PlanState::Done);
        assert_eq!(
            restored.historical_plans()[0].outcome.as_deref(),
            Some("Alpha completed")
        );
    }
}
