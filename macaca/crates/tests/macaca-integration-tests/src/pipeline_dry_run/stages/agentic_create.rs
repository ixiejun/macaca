//! Stage 4: AgenticLoop + ScriptedLlm drives `create_todo`.
//!
//! Verifies that a scripted tool-call turn persists a todo via the real tool path.

use std::sync::Arc;
use std::time::Duration;

use macaca_context::ContextBudget;
use macaca_proto::types::{
    AgentId, ApplicationId, LlmMessage, LlmOptions, Permission, PermissionLevel, ToolCall,
};
use macaca_runtime::{AgenticLoop, RuntimeConfig};
use macaca_task::{TaskSpace, TodoStore};
use macaca_tools::CreateTodoTool;
use serde_json::json;
use tracing::info;

use crate::scripted_llm::ScriptedLlm;

use crate::pipeline_dry_run::agentic::run_agentic_traced;
use crate::pipeline_dry_run::config::PipelineDryRunConfig;
use crate::pipeline_dry_run::fixtures::{COORDINATOR_AGENT, PLANNER_AGENT};
use crate::pipeline_dry_run::llm::{response_text, response_with_tools};
use crate::pipeline_dry_run::report::{err_msg, ok, record, PipelineReport};
use crate::pipeline_dry_run::store::LocalToolSet;
use crate::pipeline_dry_run::trace::{trace, trace_stage, trace_stage_end};

/// Run agentic create_todo stage and record the outcome.
pub async fn run(
    config: PipelineDryRunConfig,
    app_id: ApplicationId,
    store: Arc<TodoStore>,
    report: &mut PipelineReport,
) {
    trace_stage(config, "4/5 AgenticLoop + ScriptedLlm (create_todo)");
    info!(target: "pipeline_dry_run", "stage_agentic_create_start");

    let res = async {
        let space = Arc::new(TaskSpace::for_session(
            app_id,
            Some("sess-agentic".into()),
            store.clone(),
        ));
        let create = CreateTodoTool {
            space: space.clone(),
            coordinator_name: COORDINATOR_AGENT.into(),
            disallowed_assignees: vec![],
            assignee_capabilities: std::collections::HashMap::new(),
            active_goal_id: None,
        };
        let tools = LocalToolSet::new(vec![Box::new(create)]);
        let llm = ScriptedLlm::new(
            "script",
            vec![
                response_with_tools(vec![ToolCall {
                    id: "call-1".into(),
                    name: "create_todo".into(),
                    arguments: json!({
                        "agent": PLANNER_AGENT,
                        "title": "from_scripted_llm",
                        "description": "no network",
                        "priority": 4u8,
                    }),
                }]),
                response_text("planning complete"),
            ],
        );

        let perm = Permission {
            level: PermissionLevel::System,
            allowed_tools: vec![],
            allowed_paths: vec![],
            network_access: false,
        };
        let loop_ = AgenticLoop::new(RuntimeConfig {
            max_iterations: 8,
            tool_timeout: Duration::from_secs(5),
            context_engine: "passthrough".into(),
            context_fallback_engine: "passthrough".into(),
            context_budget: ContextBudget::default(),
            context: macaca_proto::config::ContextConfig::default(),
        });
        let agent_id = AgentId::new();
        trace(
            config,
            format!("  scripted turns: ① tool create_todo ② final text (agent_id={agent_id})"),
        );

        let run_result = run_agentic_traced(
            config,
            "create_todo",
            &loop_,
            &agent_id,
            &llm,
            &tools,
            vec![LlmMessage::user("decompose into one todo")],
            &LlmOptions::default(),
            &perm,
        )
        .await;
        let _ = run_result.map_err(|e| e.to_string())?;

        llm.assert_drained();

        let list = store
            .list_agent_todos(&app_id, &Some("sess-agentic".into()), PLANNER_AGENT)
            .await;
        trace(
            config,
            format!(
                "  list_agent_todos({PLANNER_AGENT}) count={} titles={:?}",
                list.len(),
                list.iter().map(|t| t.title.as_str()).collect::<Vec<_>>()
            ),
        );
        if !list.iter().any(|t| t.title == "from_scripted_llm") {
            return err_msg("create_todo did not persist expected title");
        }
        ok()
    }
    .await;

    trace_stage_end(config, res.is_ok(), None);
    record("agentic_loop_create_todo_scripted", res, report);
    info!(target: "pipeline_dry_run", "stage_agentic_create_end");
}
