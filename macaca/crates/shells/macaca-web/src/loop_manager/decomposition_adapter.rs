//! Goal decomposition fallback chain and decomposition status transitions.
//!
//! When planner LLM delegation fails or returns no todos, capability-ordered
//! fallback tasks are synthesized without hardcoding application agent names.

use std::sync::Arc;

use macaca_proto::{ApplicationId, CreateTaskAssignmentCommand, TraceContext};
use macaca_sdk::ServiceBackedTaskBoardDataSource;

use super::agent_execution_adapter::list_goal_todos_for_scope;
use super::execution_control_adapter::session_loop_coordinator;
use crate::session_loop_shell_adapter::{
    wake_worker_loops_and_notify_local, REASON_SESSION_LOOP_GOAL_DECOMPOSITION_READY,
};
use crate::state::AppState;

/// Wake worker loops after recording an auditable execution-control checkpoint.
pub(crate) async fn wake_worker_loops(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: Option<&str>,
    reason_code: &str,
    detail: Option<String>,
) {
    let coordinator = session_loop_coordinator(state);
    wake_worker_loops_and_notify_local(
        state,
        &coordinator,
        app_id,
        session_id,
        reason_code,
        detail,
    )
    .await;
}

pub(crate) async fn mark_goal_decomposition_ready(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: Option<&str>,
    goal_id: macaca_proto::TaskId,
    task_count: usize,
) {
    state
        .persist
        .todo_store
        .update_goal_status(app_id, &goal_id, macaca_proto::TodoGoalStatus::InProgress)
        .await;
    crate::run_trace::emit_for_scope(
        &state.persist.run_tracer,
        session_id,
        app_id,
        "plan.goal_decomposition_ready",
        "plan_loop",
        crate::run_trace::status::OK,
        Some(format!("tasks={task_count}")),
        None,
        Some(goal_id.to_string()),
        None,
    )
    .await;
    wake_worker_loops(
        state,
        app_id,
        session_id,
        REASON_SESSION_LOOP_GOAL_DECOMPOSITION_READY,
        Some(format!("tasks={task_count}")),
    )
    .await;
}

pub(crate) async fn mark_goal_decomposition_failed(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: Option<&str>,
    goal_id: macaca_proto::TaskId,
    error: &str,
) {
    let mut cancelled = 0usize;
    let mut tasks = list_goal_todos_for_scope(state, app_id, session_id, goal_id).await;
    for task in &mut tasks {
        if matches!(
            task.status,
            macaca_proto::TodoStatus::Pending
                | macaca_proto::TodoStatus::Blocked
                | macaca_proto::TodoStatus::Assigned
        ) {
            task.status = macaca_proto::TodoStatus::Cancelled;
            task.updated_at = chrono::Utc::now();
            state.persist.todo_store.save_todo(task).await;
            cancelled += 1;
        }
    }
    state
        .persist
        .todo_store
        .update_goal_status(app_id, &goal_id, macaca_proto::TodoGoalStatus::Failed)
        .await;
    crate::run_trace::emit_for_scope(
        &state.persist.run_tracer,
        session_id,
        app_id,
        "plan.goal_decomposition_failed",
        "plan_loop",
        crate::run_trace::status::ERROR,
        Some(error.chars().take(200).collect::<String>()),
        None,
        Some(goal_id.to_string()),
        Some(serde_json::json!({ "cancelled_partial_todos": cancelled })),
    )
    .await;
}

#[derive(Clone, Debug)]
pub(crate) struct PlannerWorkerDossier {
    pub(crate) name: String,
    pub(crate) capabilities: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum FallbackTaskPhase {
    Research = 0,
    Analyze = 1,
    Produce = 2,
    Validate = 3,
    Finalize = 4,
    Execute = 5,
}

pub(crate) fn fallback_phase_for(capabilities: &[String]) -> FallbackTaskPhase {
    let text = capabilities.join(" ").to_lowercase();
    if text.contains("research")
        || text.contains("source")
        || text.contains("web")
        || text.contains("discovery")
    {
        FallbackTaskPhase::Research
    } else if text.contains("analysis")
        || text.contains("analyze")
        || text.contains("architecture")
        || text.contains("design")
        || text.contains("spec")
        || text.contains("planning")
        || text.contains("interface")
    {
        FallbackTaskPhase::Analyze
    } else if text.contains("write")
        || text.contains("draft")
        || text.contains("implement")
        || text.contains("build")
        || text.contains("code")
        || text.contains("artifact")
        || text.contains("deliverable")
        || text.contains("scaffold")
        || text.contains("component")
    {
        // Capability keywords only — never hardcode application agent role names
        // (e.g. fullstack "frontend"/"backend"); persona manifests declare specialists.
        FallbackTaskPhase::Produce
    } else if text.contains("fact")
        || text.contains("verify")
        || text.contains("validation")
        || text.contains("quality")
    {
        // Validation is intentionally after production in fallback mode. A
        // planner-authored graph may express richer parallel validation, but
        // this conservative synthetic chain must create the primary artifact
        // before asking a validator or reviewer to judge it.
        FallbackTaskPhase::Validate
    } else if text.contains("edit")
        || text.contains("review")
        || text.contains("package")
        || text.contains("polish")
        || text.contains("test")
        || text.contains("qa")
    {
        FallbackTaskPhase::Finalize
    } else {
        FallbackTaskPhase::Execute
    }
}

fn fallback_task_template(
    phase: FallbackTaskPhase,
    agent_name: &str,
    goal_description: &str,
) -> (String, String, Vec<String>) {
    let clipped_goal = goal_description.chars().take(500).collect::<String>();
    let title = match phase {
        FallbackTaskPhase::Research => "Collect source material and context",
        FallbackTaskPhase::Validate => "Validate facts, assumptions, and constraints",
        FallbackTaskPhase::Analyze => "Produce specialist analysis and handoff brief",
        FallbackTaskPhase::Produce => "Create the primary specialist deliverable",
        FallbackTaskPhase::Finalize => "Review, polish, and package final deliverables",
        FallbackTaskPhase::Execute => "Complete specialist contribution",
    };
    let description = format!(
        "Planner LLM timed out before creating todos. Fallback task for `{agent_name}`.\n\n\
         Goal:\n{clipped_goal}\n\n\
         Use your declared capabilities and allowed tools to produce the durable deliverable \
         expected for this stage. Save outputs under the shared workspace when files are needed, \
         then submit a concise review summary with exact paths."
    );
    let criteria = match phase {
        FallbackTaskPhase::Research => vec![
            "Collect reliable source material or project context relevant to the goal".to_string(),
            "Separate confirmed facts from assumptions or unresolved questions".to_string(),
            "Save a durable handoff artifact in the shared workspace when applicable".to_string(),
        ],
        FallbackTaskPhase::Validate => vec![
            "Validate important claims, assumptions, dependencies, or constraints".to_string(),
            "Flag unsupported, risky, contradictory, or incomplete information".to_string(),
            "Save findings and safe wording or implementation guidance when applicable".to_string(),
        ],
        FallbackTaskPhase::Analyze => vec![
            "Create a clear specialist brief, plan, architecture, or analysis for downstream work"
                .to_string(),
            "Identify dependencies, risks, and handoff requirements".to_string(),
            "Save the brief in the shared workspace when applicable".to_string(),
        ],
        FallbackTaskPhase::Produce => vec![
            "Produce the primary deliverable requested by the goal for this specialist area"
                .to_string(),
            "Use upstream handoff artifacts if they exist".to_string(),
            "Save durable output files or a detailed completion summary".to_string(),
        ],
        FallbackTaskPhase::Finalize => vec![
            "Review upstream deliverables for completeness, correctness, and user fit".to_string(),
            "Polish or package final outputs without hiding uncertainty".to_string(),
            "Submit exact output paths and remaining caveats".to_string(),
        ],
        FallbackTaskPhase::Execute => vec![
            "Complete a useful specialist contribution toward the goal".to_string(),
            "Create durable output or a clear handoff summary".to_string(),
            "Submit exact paths, decisions, and unresolved blockers".to_string(),
        ],
    };
    (title.to_string(), description, criteria)
}

pub(crate) async fn create_fallback_decomposition_tasks(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: Option<&str>,
    goal_id: macaca_proto::TaskId,
    plan_agent_name: &str,
    goal_description: &str,
    workers: &[PlannerWorkerDossier],
    initial_dependency: Option<macaca_proto::TaskId>,
    planner_error: &str,
) -> Vec<macaca_proto::TodoItem> {
    let mut ordered = workers.to_vec();
    ordered.sort_by_key(|worker| {
        (
            fallback_phase_for(&worker.capabilities),
            worker.name.clone(),
        )
    });
    ordered.truncate(5);

    if ordered.is_empty() {
        return Vec::new();
    }

    tracing::warn!(
        goal_id = %goal_id,
        planner_error = %planner_error,
        fallback_tasks = ordered.len(),
        "Planner produced no todos; creating capability-based fallback task chain"
    );

    let task_client = ServiceBackedTaskBoardDataSource::new(state.system_facade.service_client());
    let mut created = Vec::new();
    let mut previous: Option<macaca_proto::TaskId> = initial_dependency;

    for (index, worker) in ordered.iter().enumerate() {
        let phase = fallback_phase_for(&worker.capabilities);
        let (title, description, acceptance_criteria) =
            fallback_task_template(phase, &worker.name, goal_description);
        let depends_on = previous.into_iter().collect::<Vec<_>>();
        let mut trace = TraceContext::new(format!(
            "task-fallback-assignment-{}-{}",
            goal_id,
            uuid::Uuid::new_v4()
        ));
        trace.session_id = session_id.map(str::to_string);
        trace.task_id = Some(goal_id.to_string());
        let item = match task_client
            .create_task_assignment(CreateTaskAssignmentCommand {
                app_id: app_id.clone(),
                session_id: session_id.map(str::to_string),
                agent_name: worker.name.clone(),
                created_by: plan_agent_name.to_string(),
                title,
                description,
                acceptance_criteria,
                priority: 8u8.saturating_sub(index.min(3) as u8),
                depends_on,
                parent_task: Some(goal_id),
                // Fallback tasks are generic application-execution records owned by Task Service.
                graph_owner: macaca_proto::TaskGraphOwner::ApplicationExecution,
                graph_id: None,
                trace: Some(trace),
            })
            .await
        {
            Ok(item) => item,
            Err(error) => {
                tracing::error!(
                    goal_id = %goal_id,
                    agent = %worker.name,
                    error = %error,
                    "task service failed to create fallback decomposition task"
                );
                break;
            }
        };
        previous = Some(item.id);
        created.push(item);
    }

    crate::run_trace::emit_for_scope(
        &state.persist.run_tracer,
        session_id,
        app_id,
        "plan.goal_decomposition_fallback_ready",
        "plan_loop",
        crate::run_trace::status::INFO,
        Some(format!(
            "planner_error={}; fallback_tasks={}",
            planner_error.chars().take(160).collect::<String>(),
            created.len()
        )),
        None,
        Some(goal_id.to_string()),
        Some(serde_json::json!({
            "task_count": created.len(),
            "planner_error": planner_error,
            "agents": created.iter().map(|task| task.assigned_agent.clone()).collect::<Vec<_>>(),
        })),
    )
    .await;

    created
}

#[cfg(test)]
mod tests {
    use super::{fallback_phase_for, FallbackTaskPhase};

    #[test]
    fn fallback_phase_orders_primary_production_before_validation_or_review() {
        let producer = vec![
            "code_change_planning".to_string(),
            "build_artifact".to_string(),
        ];
        let reviewer = vec!["quality_review".to_string(), "qa_validation".to_string()];

        assert_eq!(fallback_phase_for(&producer), FallbackTaskPhase::Produce);
        assert_eq!(fallback_phase_for(&reviewer), FallbackTaskPhase::Validate);
        assert!(
            fallback_phase_for(&producer) < fallback_phase_for(&reviewer),
            "synthetic fallback chains must produce the primary artifact before validation/review"
        );
    }
}

pub(crate) fn terminal_goal_task(tasks: &[macaca_proto::TodoItem]) -> Option<macaca_proto::TaskId> {
    let dependency_ids = tasks
        .iter()
        .flat_map(|task| task.depends_on.iter().copied())
        .collect::<std::collections::HashSet<_>>();
    tasks
        .iter()
        .rev()
        .find(|task| !dependency_ids.contains(&task.id))
        .map(|task| task.id)
        .or_else(|| tasks.last().map(|task| task.id))
}
