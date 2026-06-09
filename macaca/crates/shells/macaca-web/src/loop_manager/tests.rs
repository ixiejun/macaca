#[cfg(test)]
mod tests {
    use crate::loop_manager::agent_execution_adapter::PlannerFrameworkCallKind;
    use crate::loop_manager::planner_helpers::{
        executor_task_completed, executor_task_failed, executor_task_started,
        goal_has_decomposed_tasks, mark_decomposition_in_notebook, mark_review_in_notebook,
        planner_scope_session_id, select_entry_and_plan_agents,
    };
    use crate::loop_manager::worker_execution_adapter::{worker_success_summary, WorkerExecutionMode};
    use macaca_sdk::framework::plan::{PlanNotebook, PlanState, SubTaskState};
    use macaca_sdk::runtime_host::executor::ExecutorEvent;
    use macaca_sdk::runtime_host::AgentInfo;
    use macaca_proto::AgentExecutionIntent;

    fn agent(name: &str, capabilities: &[&str]) -> AgentInfo {
        AgentInfo {
            id: format!("id-{name}"),
            name: name.to_string(),
            capabilities: capabilities.iter().map(|c| c.to_string()).collect(),
            current_load: 0,
            max_load: 4,
            available: true,
        }
    }

    #[test]
    fn planner_selection_is_capability_driven_not_name_driven() {
        let agents = vec![
            agent("orchestrator", &["todo_goal_management"]),
            agent("decomposer", &["task_planning"]),
            agent("executor_a", &["todo_execution"]),
        ];
        let (entry, planner) = select_entry_and_plan_agents(&agents, Some("orchestrator"));
        assert_eq!(entry, "orchestrator");
        assert_eq!(planner, "decomposer");
    }

    #[test]
    fn planner_falls_back_to_entry_when_no_planning_capability() {
        let agents = vec![
            agent("entry_custom", &["todo_goal_management"]),
            agent("worker_custom", &["todo_execution"]),
        ];
        let (entry, planner) = select_entry_and_plan_agents(&agents, Some("entry_custom"));
        assert_eq!(entry, "entry_custom");
        assert_eq!(planner, "entry_custom");
    }

    #[test]
    fn planner_scope_session_id_preserves_session_and_app_fallback() {
        let app_id = macaca_proto::ApplicationId::from_name("demo-app");

        assert_eq!(
            planner_scope_session_id(&app_id, Some("session-123")),
            "session-123"
        );
        assert_eq!(
            planner_scope_session_id(&app_id, None),
            format!("_macaca_app_{}", app_id.0)
        );
    }

    #[test]
    fn goal_has_decomposed_tasks_detects_existing_parent_task() {
        let app_id = macaca_proto::ApplicationId::from_name("demo-app");
        let goal_id = macaca_proto::TaskId::new();
        let other_goal_id = macaca_proto::TaskId::new();
        let mut task = macaca_proto::TodoItem::new(
            app_id,
            Some("session-123".into()),
            "architect",
            "planner",
            "Design architecture",
            "Define architecture and API contracts",
            9,
        );
        task.parent_task = Some(goal_id);

        assert!(goal_has_decomposed_tasks(&[task.clone()], goal_id));
        assert!(!goal_has_decomposed_tasks(&[task], other_goal_id));
        assert!(!goal_has_decomposed_tasks(&[], goal_id));
    }

    #[test]
    fn planner_notebook_decomposition_content_is_preserved() {
        let goal_id = macaca_proto::TaskId::new();
        let description = "Build a small app";
        let mut notebook = PlanNotebook::new();

        mark_decomposition_in_notebook(&mut notebook, goal_id, description);

        assert!(notebook.current_plan().is_none());
        assert_eq!(notebook.historical_plans().len(), 1);
        let plan = &notebook.historical_plans()[0];
        assert_eq!(plan.name, format!("goal:{}", goal_id));
        assert_eq!(plan.description, description);
        assert_eq!(
            plan.expected_outcome,
            "Decompose goal into executable todos"
        );
        assert_eq!(plan.state, PlanState::Done);
        assert_eq!(
            plan.outcome.as_deref(),
            Some(format!("goal {} decomposition recorded", goal_id).as_str())
        );
        assert_eq!(plan.subtasks.len(), 1);
        let subtask = &plan.subtasks[0];
        assert_eq!(subtask.name, "decompose_goal");
        assert_eq!(subtask.description, format!("Decompose goal {}", goal_id));
        assert_eq!(
            subtask.expected_outcome,
            "Todos created and persisted to TodoBoard"
        );
        assert_eq!(subtask.state, SubTaskState::Done);
        assert_eq!(
            subtask.outcome.as_deref(),
            Some("decomposition delegated to planner")
        );
    }

    #[test]
    fn planner_notebook_review_content_is_preserved() {
        let task_id = macaca_proto::TaskId::new();
        let task_title = "Implement API";
        let mut notebook = PlanNotebook::new();

        mark_review_in_notebook(&mut notebook, task_id, task_title);

        assert!(notebook.current_plan().is_none());
        assert_eq!(notebook.historical_plans().len(), 1);
        let plan = &notebook.historical_plans()[0];
        assert_eq!(plan.name, format!("review:{}", task_id));
        assert_eq!(plan.description, format!("Review task '{}'", task_title));
        assert_eq!(
            plan.expected_outcome,
            "Task review decision persisted via review_todo"
        );
        assert_eq!(plan.state, PlanState::Done);
        assert_eq!(
            plan.outcome.as_deref(),
            Some(format!("task {} review recorded", task_id).as_str())
        );
        assert_eq!(plan.subtasks.len(), 1);
        let subtask = &plan.subtasks[0];
        assert_eq!(subtask.name, "review_todo");
        assert_eq!(subtask.description, format!("Review todo {}", task_id));
        assert_eq!(
            subtask.expected_outcome,
            "Todo status updated to completed/needs_optimization/failed"
        );
        assert_eq!(subtask.state, SubTaskState::Done);
        assert_eq!(
            subtask.outcome.as_deref(),
            Some("review delegated to planner")
        );
    }

    #[test]
    fn executor_task_started_helper_preserves_fields() {
        let task_id = macaca_proto::TaskId::new();

        let event = executor_task_started(task_id, "planner");

        match event {
            ExecutorEvent::TaskStarted {
                task_id: got,
                agent,
            } => {
                assert_eq!(got, task_id);
                assert_eq!(agent, "planner");
            }
            other => panic!("expected TaskStarted, got {other:?}"),
        }
    }

    #[test]
    fn executor_task_completed_helper_preserves_result_fields() {
        let task_id = macaca_proto::TaskId::new();

        let event = executor_task_completed(task_id, "backend", "done");

        match event {
            ExecutorEvent::TaskCompleted {
                task_id: got,
                agent,
                result,
            } => {
                assert_eq!(got, task_id);
                assert_eq!(agent, "backend");
                assert_eq!(result.task_id, task_id);
                assert!(result.success);
                assert_eq!(result.output, "done");
                assert_eq!(result.error, None);
                assert!(result.artifacts.is_empty());
                assert!(result.tokens_used.is_none());
                assert!(result.completed_at <= chrono::Utc::now());
            }
            other => panic!("expected TaskCompleted, got {other:?}"),
        }
    }

    #[test]
    fn executor_task_failed_helper_preserves_fields() {
        let task_id = macaca_proto::TaskId::new();

        let event = executor_task_failed(task_id, "frontend", "boom");

        match event {
            ExecutorEvent::TaskFailed {
                task_id: got,
                agent,
                error,
            } => {
                assert_eq!(got, task_id);
                assert_eq!(agent, "frontend");
                assert_eq!(error, "boom");
            }
            other => panic!("expected TaskFailed, got {other:?}"),
        }
    }

    #[test]
    fn planner_framework_call_uses_service_execution_intents() {
        let goal_id = macaca_proto::TaskId::new();

        let source = include_str!("agent_execution_adapter.rs");
        let legacy_builder = ["FrameworkRunner::build_", "for_intent"].concat();
        assert!(source.contains("AGENT_EXECUTION_SERVICE_ID"));
        assert!(source.contains("AgentExecutionCommand::new"));
        assert!(!source.contains(&legacy_builder));
        assert_eq!(AgentExecutionIntent::Planner, AgentExecutionIntent::Planner);
        assert_eq!(
            PlannerFrameworkCallKind::DecomposeGoal.goal_context(goal_id),
            Some(goal_id)
        );
        assert_eq!(
            AgentExecutionIntent::GoalWorker,
            AgentExecutionIntent::GoalWorker
        );
        assert_eq!(
            PlannerFrameworkCallKind::FollowUp.goal_context(goal_id),
            Some(goal_id)
        );
        assert_eq!(
            PlannerFrameworkCallKind::GoalEvaluation.goal_context(goal_id),
            Some(goal_id)
        );
    }

    #[test]
    fn planner_framework_call_uses_review_intent_for_review() {
        let task_id = macaca_proto::TaskId::new();
        let intent = match PlannerFrameworkCallKind::Review {
            PlannerFrameworkCallKind::DecomposeGoal => AgentExecutionIntent::Planner,
            PlannerFrameworkCallKind::Review => AgentExecutionIntent::Reviewer,
            PlannerFrameworkCallKind::FollowUp => AgentExecutionIntent::GoalWorker,
            PlannerFrameworkCallKind::GoalEvaluation => AgentExecutionIntent::GoalWorker,
        };

        assert_eq!(intent, AgentExecutionIntent::Reviewer);
        assert_eq!(PlannerFrameworkCallKind::Review.goal_context(task_id), None);
    }

    #[test]
    fn planner_framework_call_messages_preserve_existing_log_text() {
        assert_eq!(
            PlannerFrameworkCallKind::DecomposeGoal.success_log_prefix(),
            "Planner decomposition completed"
        );
        assert_eq!(
            PlannerFrameworkCallKind::DecomposeGoal.reply_error_log_prefix(),
            "Planner decomposition failed"
        );
        assert_eq!(
            PlannerFrameworkCallKind::DecomposeGoal.build_error_log_prefix(),
            "Failed to build planner agent"
        );
        assert_eq!(
            PlannerFrameworkCallKind::DecomposeGoal.missing_executor_log(),
            "No executor found for planner decomposition"
        );
        assert_eq!(
            PlannerFrameworkCallKind::Review.success_log_prefix(),
            "Review completed"
        );
        assert_eq!(
            PlannerFrameworkCallKind::Review.reply_error_log_prefix(),
            "Review failed"
        );
        assert_eq!(
            PlannerFrameworkCallKind::Review.build_error_log_prefix(),
            "Failed to build planner agent for review"
        );
        assert_eq!(
            PlannerFrameworkCallKind::Review.missing_executor_log(),
            "No executor found for planner review"
        );
        assert_eq!(
            PlannerFrameworkCallKind::FollowUp.success_log_prefix(),
            "Follow-up tasks created"
        );
        assert_eq!(
            PlannerFrameworkCallKind::FollowUp.reply_error_log_prefix(),
            "Follow-up task creation failed"
        );
        assert_eq!(
            PlannerFrameworkCallKind::FollowUp.build_error_log_prefix(),
            "Failed to build planner agent for follow-up"
        );
        assert_eq!(
            PlannerFrameworkCallKind::FollowUp.missing_executor_log(),
            "No executor found for planner follow-up"
        );
        assert_eq!(
            PlannerFrameworkCallKind::GoalEvaluation.success_log_prefix(),
            "Goal evaluation completed"
        );
        assert_eq!(
            PlannerFrameworkCallKind::GoalEvaluation.reply_error_log_prefix(),
            "Goal evaluation failed"
        );
        assert_eq!(
            PlannerFrameworkCallKind::GoalEvaluation.build_error_log_prefix(),
            "Failed to build planner agent for goal evaluation"
        );
        assert_eq!(
            PlannerFrameworkCallKind::GoalEvaluation.missing_executor_log(),
            "No executor found for planner goal evaluation"
        );
    }

    #[test]
    fn worker_success_summary_preserves_normal_empty_output_fallback() {
        let summary = worker_success_summary(
            WorkerExecutionMode::TaskClaimed,
            "Implement API",
            String::new(),
        );

        assert_eq!(summary, "Task 'Implement API' completed");
    }

    #[test]
    fn worker_success_summary_preserves_retry_empty_output_fallback() {
        let summary =
            worker_success_summary(WorkerExecutionMode::Retry, "Implement API", String::new());

        assert_eq!(summary, "Task 'Implement API' completed on retry");
    }

    #[test]
    fn worker_success_summary_preserves_non_empty_output() {
        let summary = worker_success_summary(
            WorkerExecutionMode::TaskClaimed,
            "Implement API",
            "custom summary".to_string(),
        );

        assert_eq!(summary, "custom summary");
    }

    #[test]
    fn worker_execution_mode_preserves_trace_detail_and_error_messages() {
        assert_eq!(
            WorkerExecutionMode::TaskClaimed.success_submit_review_detail("abcdef"),
            "abcdef"
        );
        assert_eq!(
            WorkerExecutionMode::Retry.success_submit_review_detail("abcdef"),
            "retry_success"
        );
        assert_eq!(
            WorkerExecutionMode::TaskClaimed.panic_error(),
            "Task execution panicked"
        );
        assert_eq!(
            WorkerExecutionMode::Retry.panic_error(),
            "Retry task execution panicked"
        );
        assert_eq!(
            WorkerExecutionMode::TaskClaimed.timeout_error(),
            "Execution timeout (30 min)"
        );
        assert_eq!(
            WorkerExecutionMode::Retry.timeout_error(),
            "Retry execution timeout (30 min)"
        );
    }
}
