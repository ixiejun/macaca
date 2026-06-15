//! Stage 5: AgenticLoop claim → start → submit → review_todo.
//!
//! Seeds a fixed task id, then drives the full board workflow via scripted tool calls.

use std::sync::Arc;
use std::time::Duration;

use macaca_context::ContextBudget;
use macaca_proto::types::{
    AgentId, ApplicationId, LlmMessage, LlmOptions, Permission, PermissionLevel, TaskId, TodoItem,
    TodoStatus, ToolCall,
};
use macaca_runtime::{AgenticLoop, RuntimeConfig};
use macaca_task::{TaskBoard, TaskSpace, TodoStore};
use macaca_tools::{ClaimTaskTool, ReviewTodoTool, StartTaskTool, SubmitTaskForReviewTool};
use serde_json::json;
use tracing::info;
use uuid::Uuid;

use crate::scripted_llm::ScriptedLlm;

use crate::pipeline_dry_run::agentic::run_agentic_traced;
use crate::pipeline_dry_run::config::PipelineDryRunConfig;
use crate::pipeline_dry_run::fixtures::{ASSIGNEE_AGENT, COORDINATOR_AGENT};
use crate::pipeline_dry_run::llm::{response_text, response_with_tools};
use crate::pipeline_dry_run::report::{ok, record, PipelineReport};
use crate::pipeline_dry_run::store::LocalToolSet;
use crate::pipeline_dry_run::trace::{trace, trace_stage, trace_stage_end};

/// Run agentic worker submit/review stage and record the outcome.
pub async fn run(
    config: PipelineDryRunConfig,
    app_id: ApplicationId,
    store: Arc<TodoStore>,
    report: &mut PipelineReport,
) {
    trace_stage(
        config,
        "5/5 AgenticLoop (claim → start → submit → review_todo)",
    );
    info!(target: "pipeline_dry_run", "stage_agentic_worker_review_start");

    let res = async {
        let fixed = TaskId(Uuid::from_u128(0x0f11_e22e_33d4_4556_8777_8899_aabb_ccdd));
        let mut item = TodoItem::new(
            app_id,
            Some("sess-worker".into()),
            ASSIGNEE_AGENT,
            COORDINATOR_AGENT,
            "Seeded",
            "seeded body",
            5,
        );
        item.id = fixed;
        item.sequence_number = 1;
        store.save_todo(&item).await;
        trace(
            config,
            format!(
                "  pre-seeded todo id={fixed} status={:?} session=sess-worker",
                item.status
            ),
        );

        let board = Arc::new(TaskBoard::for_agent(
            app_id,
            ASSIGNEE_AGENT,
            Some("sess-worker".into()),
            store.clone(),
        ));
        let space = Arc::new(TaskSpace::for_session(
            app_id,
            Some("sess-worker".into()),
            store.clone(),
        ));

        let tools = LocalToolSet::new(vec![
            Box::new(ClaimTaskTool {
                board: board.clone(),
            }),
            Box::new(StartTaskTool {
                board: board.clone(),
            }),
            Box::new(SubmitTaskForReviewTool {
                board: board.clone(),
            }),
            Box::new(ReviewTodoTool {
                space: space.clone(),
                on_reviewed: None,
            }),
        ]);

        let tid = fixed.to_string();
        let llm = ScriptedLlm::new(
            "script",
            vec![
                response_with_tools(vec![ToolCall {
                    id: "w1".into(),
                    name: "claim_task".into(),
                    arguments: json!({}),
                }]),
                response_with_tools(vec![ToolCall {
                    id: "w2".into(),
                    name: "start_task".into(),
                    arguments: json!({ "task_id": &tid }),
                }]),
                response_with_tools(vec![ToolCall {
                    id: "w3".into(),
                    name: "submit_task_for_review".into(),
                    arguments: json!({ "task_id": &tid, "summary": "ship it" }),
                }]),
                response_with_tools(vec![ToolCall {
                    id: "w4".into(),
                    name: "review_todo".into(),
                    arguments: json!({
                        "task_id": &tid,
                        "agent": ASSIGNEE_AGENT,
                        "passed": true,
                        "feedback": "lgtm",
                    }),
                }]),
                response_text("workflow finished"),
            ],
        );

        let perm = Permission {
            level: PermissionLevel::System,
            allowed_tools: vec![],
            allowed_paths: vec![],
            network_access: false,
        };
        let loop_ = AgenticLoop::new(RuntimeConfig {
            max_iterations: 12,
            tool_timeout: Duration::from_secs(5),
            context_engine: "passthrough".into(),
            context_fallback_engine: "passthrough".into(),
            context_budget: ContextBudget::default(),
            context: macaca_proto::config::ContextConfig::default(),
        });
        let agent_id = AgentId::new();
        trace(
            config,
            "  scripted sequence: claim_task → start_task → submit_task_for_review → review_todo → final text",
        );

        let run_result = run_agentic_traced(
            config,
            "worker_review",
            &loop_,
            &agent_id,
            &llm,
            &tools,
            vec![LlmMessage::user("execute board task")],
            &LlmOptions::default(),
            &perm,
        )
        .await;
        let _ = run_result.map_err(|e| e.to_string())?;

        llm.assert_drained();

        let done = store
            .get_todo(&app_id, &Some("sess-worker".into()), ASSIGNEE_AGENT, &fixed)
            .await
            .ok_or_else(|| "reload after review".to_string())?;
        trace(
            config,
            format!("  final todo status={:?} (expect Completed)", done.status),
        );
        if done.status != TodoStatus::Completed {
            return Err(format!(
                "expected Completed after review, got {:?}",
                done.status
            ));
        }
        ok()
    }
    .await;

    trace_stage_end(config, res.is_ok(), None);
    record("agentic_loop_worker_submit_review_scripted", res, report);
    info!(target: "pipeline_dry_run", "stage_agentic_worker_review_end");
}
