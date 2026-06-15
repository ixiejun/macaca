//! Contract tests for plan-loop DTOs, goal evaluation parsing, and event shapes.

use super::*;

// ── TaskSummary ──

#[test]
fn test_task_summary_creation() {
    let summary = TaskSummary {
        title: "Implement API".into(),
        agent: "agent-alpha".into(),
        status: "Completed".into(),
        completion_summary: Some("All endpoints implemented and tested".into()),
    };
    assert_eq!(summary.title, "Implement API");
    assert_eq!(summary.agent, "agent-alpha");
    assert_eq!(summary.status, "Completed");
    assert!(summary.completion_summary.is_some());
}

#[test]
fn test_task_summary_no_completion_summary() {
    let summary = TaskSummary {
        title: "Design DB schema".into(),
        agent: "agent-beta".into(),
        status: "Failed".into(),
        completion_summary: None,
    };
    assert!(summary.completion_summary.is_none());
}

// ── GoalEvaluation parsing ──

#[test]
fn test_goal_evaluation_prompt_builder_preserves_contract() {
    let summaries = vec![TaskSummary {
        title: "Implement API".into(),
        agent: "agent-alpha".into(),
        status: "Completed".into(),
        completion_summary: Some("API endpoints implemented".into()),
    }];

    let prompt = GoalEvaluator::build_prompt("Build sample goal", &summaries, 1, 0);

    assert!(prompt.contains("Goal: Build sample goal"));
    assert!(prompt.contains("Task Results (1 completed, 0 failed):"));
    assert!(prompt
        .contains("- [Completed] Implement API (agent: agent-alpha): API endpoints implemented"));
    assert!(prompt.contains("\"satisfied\": true/false"));
    assert!(prompt.contains("\"summary\": \"brief evaluation summary\""));
    assert!(prompt.contains("\"suggestions\": [\"suggestion 1\", \"suggestion 2\"]"));
}

#[test]
fn test_goal_evaluation_satisfied_json() {
    let json =
        r#"{"satisfied": true, "summary": "All tasks passed, goal met.", "suggestions": []}"#;
    let result = GoalEvaluator::parse_eval_response(json);
    match result {
        GoalEvaluation::Satisfied { summary } => {
            assert_eq!(summary, "All tasks passed, goal met.");
        }
        GoalEvaluation::NeedsMoreWork { .. } => panic!("expected Satisfied"),
    }
}

#[test]
fn test_goal_evaluation_needs_work_json() {
    let json = r#"{
            "satisfied": false,
            "summary": "Missing error handling in API endpoints.",
            "suggestions": ["Add 400/500 error responses", "Add input validation"]
        }"#;
    let result = GoalEvaluator::parse_eval_response(json);
    match result {
        GoalEvaluation::NeedsMoreWork {
            reason,
            suggestions,
        } => {
            assert_eq!(reason, "Missing error handling in API endpoints.");
            assert_eq!(suggestions.len(), 2);
            assert_eq!(suggestions[0], "Add 400/500 error responses");
        }
        GoalEvaluation::Satisfied { .. } => panic!("expected NeedsMoreWork"),
    }
}

#[test]
fn test_goal_evaluation_needs_work_no_suggestions() {
    let json = r#"{"satisfied": false, "summary": "Incomplete implementation."}"#;
    let result = GoalEvaluator::parse_eval_response(json);
    match result {
        GoalEvaluation::NeedsMoreWork { suggestions, .. } => {
            assert!(suggestions.is_empty());
        }
        GoalEvaluation::Satisfied { .. } => panic!("expected NeedsMoreWork"),
    }
}

#[test]
fn test_goal_evaluation_fallback_on_bad_json() {
    let result = GoalEvaluator::parse_eval_response("not valid json at all");
    match result {
        GoalEvaluation::Satisfied { summary } => {
            assert!(summary.contains("fallback"));
        }
        GoalEvaluation::NeedsMoreWork { .. } => panic!("expected Satisfied fallback"),
    }
}

#[test]
fn test_goal_evaluation_fallback_on_empty_string() {
    let result = GoalEvaluator::parse_eval_response("");
    match result {
        GoalEvaluation::Satisfied { .. } => {}
        GoalEvaluation::NeedsMoreWork { .. } => panic!("expected Satisfied fallback"),
    }
}

#[test]
fn test_goal_evaluation_parses_markdown_code_block() {
    let json = "```json\n{\"satisfied\": true, \"summary\": \"Done\", \"suggestions\": []}\n```";
    let result = GoalEvaluator::parse_eval_response(json);
    match result {
        GoalEvaluation::Satisfied { summary } => assert_eq!(summary, "Done"),
        GoalEvaluation::NeedsMoreWork { .. } => panic!("expected Satisfied"),
    }
}

#[test]
fn test_goal_evaluation_parses_plain_code_block() {
    let json = "```\n{\"satisfied\": false, \"summary\": \"Needs work\", \"suggestions\": [\"fix tests\"]}\n```";
    let result = GoalEvaluator::parse_eval_response(json);
    match result {
        GoalEvaluation::NeedsMoreWork {
            reason,
            suggestions,
        } => {
            assert_eq!(reason, "Needs work");
            assert_eq!(suggestions[0], "fix tests");
        }
        GoalEvaluation::Satisfied { .. } => panic!("expected NeedsMoreWork"),
    }
}

// ── PlanEvent variants ──

#[test]
fn test_plan_event_evaluate_goal_completion_is_debug_clone() {
    let summary = TaskSummary {
        title: "T".into(),
        agent: "a".into(),
        status: "Completed".into(),
        completion_summary: None,
    };
    let event = PlanEvent::EvaluateGoalCompletion {
        goal_id: macaca_proto::TaskId::new(),
        goal_description: "Build auth".into(),
        completed_count: 3,
        failed_count: 0,
        task_summaries: vec![summary],
        session_id: None,
    };
    let cloned = event.clone();
    let debug_str = format!("{:?}", cloned);
    assert!(debug_str.contains("EvaluateGoalCompletion"));
}

#[test]
fn test_plan_event_goal_completed_is_debug_clone() {
    let event = PlanEvent::GoalCompleted {
        goal_id: macaca_proto::TaskId::new(),
        description: "Goal is done".into(),
    };
    let debug_str = format!("{:?}", event.clone());
    assert!(debug_str.contains("GoalCompleted"));
}
