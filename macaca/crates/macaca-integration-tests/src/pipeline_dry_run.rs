//! 无真实 LLM 的全链路干跑诊断：持久化 → TaskSpace / TaskBoard → 依赖与 review →
//! `AgenticLoop` + [`crate::scripted_llm::ScriptedLlm`] 驱动真实工具。
//!
//! 详细执行轨迹（推荐）：`cargo test -p macaca-integration-tests pipeline_dry_run -- --nocapture`
//!
//! 未覆盖：HTTP 路由、`ApplicationExecutor::delegate_task`、真实 Driver；这些可在
//! `macaca-web` / `macaca-kernel` 中单独加 `ScriptedLlm` 集成测试扩展。

use std::sync::Arc;
use std::time::Duration;

use macaca_persist::RedbStore;
use macaca_proto::error::MacacaResult;
use macaca_proto::types::{
    AgentExecutionEvent, AgentId, ApplicationId, LlmMessage, LlmOptions, LlmResponse, Permission,
    PermissionLevel, TaskId, TodoItem, TodoReviewResult, TodoStatus, TokenUsage, ToolCall,
};
use macaca_runtime::{AgenticLoop, LoopResult, RuntimeConfig};
use macaca_task::{TaskBoard, TaskSpace, TodoStore};
use macaca_tools::{
    ClaimTaskTool, CreateTodoTool, ReviewTodoTool, StartTaskTool, SubmitTaskForReviewTool, Tool,
    ToolSet,
};
use serde_json::json;
use tempfile::tempdir;
use uuid::Uuid;

use crate::scripted_llm::ScriptedLlm;

/// 干跑输出粒度：`verbose=true` 时向 stderr 打印分阶段与 Agentic 事件流。
#[derive(Debug, Clone, Copy)]
pub struct PipelineDryRunConfig {
    pub verbose: bool,
}

impl Default for PipelineDryRunConfig {
    fn default() -> Self {
        Self { verbose: false }
    }
}

impl PipelineDryRunConfig {
    pub fn verbose() -> Self {
        Self { verbose: true }
    }
}

fn trace(cfg: PipelineDryRunConfig, msg: impl AsRef<str>) {
    if cfg.verbose {
        eprintln!("{}", msg.as_ref());
    }
}

fn trace_stage(cfg: PipelineDryRunConfig, title: &str) {
    if cfg.verbose {
        eprintln!();
        eprintln!("┌── {} ──", title);
    }
}

fn trace_stage_end(cfg: PipelineDryRunConfig, ok: bool, detail: Option<&str>) {
    if cfg.verbose {
        let status = if ok { "OK" } else { "FAIL" };
        if let Some(d) = detail {
            eprintln!("└── {} — {}", status, d);
        } else {
            eprintln!("└── {}", status);
        }
    }
}

fn format_agent_event(ev: &AgentExecutionEvent) -> String {
    match ev {
        AgentExecutionEvent::Thinking { iteration, content } => {
            if let Some(c) = content {
                format!("iteration {iteration} | thinking | {c}")
            } else {
                format!("iteration {iteration} | thinking | (start LLM round)")
            }
        }
        AgentExecutionEvent::ToolCall {
            tool_name,
            tool_input,
            call_id,
        } => {
            let args = serde_json::to_string(tool_input).unwrap_or_else(|_| tool_input.to_string());
            let id = call_id.as_deref().unwrap_or("-");
            format!("tool_call | {tool_name} | id={id} | args={args}")
        }
        AgentExecutionEvent::ToolResult {
            tool_name,
            output,
            is_error,
        } => {
            let err = is_error.unwrap_or(false);
            let out = if output.len() > 240 {
                format!("{}…", &output[..240])
            } else {
                output.clone()
            };
            format!("tool_result | {tool_name} | error={err} | {out}")
        }
        AgentExecutionEvent::Assistant { content } => {
            format!(
                "assistant_text | {}",
                content.chars().take(120).collect::<String>()
            )
        }
        AgentExecutionEvent::CcTrace { .. } => format!("cc_trace | {ev:?}"),
        AgentExecutionEvent::Completed { success, error } => {
            format!("completed | success={success} | error={error:?}")
        }
    }
}

/// 单阶段结果：名称 + 成功或错误信息（便于 `--nocapture` 下定位卡在哪一步）。
#[derive(Debug, Clone)]
pub struct PipelineReport {
    pub stages: Vec<(String, Result<(), String>)>,
}

impl PipelineReport {
    pub fn assert_all_ok(&self) {
        let failures: Vec<_> = self
            .stages
            .iter()
            .filter_map(|(n, r)| r.as_ref().err().map(|e| format!("{n}: {e}")))
            .collect();
        assert!(
            failures.is_empty(),
            "pipeline dry-run failures:\n{}",
            failures.join("\n")
        );
    }

    /// 人类可读摘要（测试失败或日志用）。
    pub fn summary(&self) -> String {
        self.stages
            .iter()
            .map(|(name, r)| match r {
                Ok(()) => format!("OK  {name}"),
                Err(e) => format!("ERR {name}: {e}"),
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn record(stage: &str, res: Result<(), String>, report: &mut PipelineReport) {
    report.stages.push((stage.to_string(), res));
}

fn ok() -> Result<(), String> {
    Ok(())
}

fn err_msg(msg: &str) -> Result<(), String> {
    Err(msg.to_string())
}

pub fn response_text(content: impl Into<String>) -> LlmResponse {
    LlmResponse {
        content: content.into(),
        model: "scripted".into(),
        usage: TokenUsage::default(),
        finish_reason: "stop".into(),
        tool_calls: None,
    }
}

pub fn response_with_tools(tool_calls: Vec<ToolCall>) -> LlmResponse {
    LlmResponse {
        content: String::new(),
        model: "scripted".into(),
        usage: TokenUsage::default(),
        finish_reason: "tool_calls".into(),
        tool_calls: Some(tool_calls),
    }
}

struct LocalToolSet(Vec<Box<dyn Tool>>);

impl ToolSet for LocalToolSet {
    fn tools(&self) -> &[Box<dyn Tool>] {
        &self.0
    }
}

fn open_temp_store() -> Result<(tempfile::TempDir, Arc<TodoStore>), String> {
    let dir = tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().join("pipeline_dry_run.redb");
    let store = RedbStore::open(path).map_err(|e| e.to_string())?;
    Ok((dir, Arc::new(TodoStore::new(Arc::new(store)))))
}

/// 与 [`run_full_pipeline_dry_run_with_config`] 相同，`verbose = false`（无控制台轨迹）。
pub async fn run_full_pipeline_dry_run() -> PipelineReport {
    run_full_pipeline_dry_run_with_config(PipelineDryRunConfig::default()).await
}

/// 顺序执行各阶段；任一阶段失败仍继续跑完后续（便于一次看到多个断裂点）。
///
/// `config.verbose == true` 时向 **stderr** 打印：阶段边界、数据面状态迁移、以及
/// Agentic 循环内 `Thinking` / `ToolCall` / `ToolResult` / `Assistant` / `Completed` 事件流。
pub async fn run_full_pipeline_dry_run_with_config(config: PipelineDryRunConfig) -> PipelineReport {
    let mut report = PipelineReport { stages: vec![] };

    if config.verbose {
        eprintln!();
        eprintln!("========== pipeline dry-run (no real LLM) ==========");
    }

    // 1) 持久化
    let (dir, store) = match open_temp_store() {
        Ok(x) => x,
        Err(e) => {
            record("redb_todo_store_open", Err(e.clone()), &mut report);
            trace_stage_end(config, false, Some(&e));
            return report;
        }
    };
    trace_stage(config, "1/5 redb + TodoStore");
    trace(
        config,
        format!(
            "  db_path = {}",
            dir.path().join("pipeline_dry_run.redb").display()
        ),
    );
    record("redb_todo_store_open", ok(), &mut report);
    trace_stage_end(config, true, Some("opened embedded redb, TodoStore ready"));
    let _keep = dir;

    let app_id = ApplicationId::new();
    trace(
        config,
        format!("  application_id = {app_id} (all stages share this app)"),
    );

    // 2) 会话隔离：同 agent、不同 session 各自 claim 自己的任务
    {
        trace_stage(config, "2/5 session_scoped_claim_isolation");
        let res = async {
            let sa = TaskSpace::new(app_id, Some("sess-a".into()), store.clone());
            let sb = TaskSpace::new(app_id, Some("sess-b".into()), store.clone());
            let ta = sa
                .create_and_assign("worker", "coord", "A", "da", vec![], 5, vec![], None)
                .await;
            let tb = sb
                .create_and_assign("worker", "coord", "B", "db", vec![], 5, vec![], None)
                .await;
            trace(
                config,
                format!(
                    "  TaskSpace.create_and_assign | sess-a | task_id={} title=A status={:?}",
                    ta.id, ta.status
                ),
            );
            trace(
                config,
                format!(
                    "  TaskSpace.create_and_assign | sess-b | task_id={} title=B status={:?}",
                    tb.id, tb.status
                ),
            );

            let board_a = TaskBoard::new(app_id, "worker", Some("sess-a".into()), store.clone());
            let board_b = TaskBoard::new(app_id, "worker", Some("sess-b".into()), store.clone());

            let ca = board_a
                .claim_next()
                .await
                .ok_or_else(|| "session A: expected claim".to_string())?;
            let cb = board_b
                .claim_next()
                .await
                .ok_or_else(|| "session B: expected claim".to_string())?;
            trace(
                config,
                format!(
                    "  TaskBoard.claim_next(sess-a) -> task_id={} status_after={:?}",
                    ca.id, ca.status
                ),
            );
            trace(
                config,
                format!(
                    "  TaskBoard.claim_next(sess-b) -> task_id={} status_after={:?}",
                    cb.id, cb.status
                ),
            );
            if ca.id != ta.id {
                return Err(format!(
                    "session A claimed wrong task: {:?} vs {:?}",
                    ca.id, ta.id
                ));
            }
            if cb.id != tb.id {
                return Err(format!(
                    "session B claimed wrong task: {:?} vs {:?}",
                    cb.id, tb.id
                ));
            }
            ok()
        }
        .await;
        trace_stage_end(config, res.is_ok(), None);
        record("session_scoped_claim_isolation", res, &mut report);
    }

    // 3) depends_on → review 通过后解除 Blocked
    {
        trace_stage(config, "3/5 depends_on + review_unblocks_next");
        let res = async {
            let space = TaskSpace::new(app_id, Some("sess-dep".into()), store.clone());
            let t1 = space
                .create_and_assign("worker", "coord", "first", "d1", vec![], 5, vec![], None)
                .await;
            let t2 = space
                .create_and_assign(
                    "worker",
                    "coord",
                    "second",
                    "d2",
                    vec![],
                    5,
                    vec![t1.id],
                    None,
                )
                .await;
            trace(
                config,
                format!(
                    "  created t1 id={} status={:?} | t2 id={} status={:?} depends_on=[{}]",
                    t1.id, t1.status, t2.id, t2.status, t1.id
                ),
            );
            if t2.status != TodoStatus::Blocked {
                return Err(format!("expected second task Blocked, got {:?}", t2.status));
            }
            trace(config, "  (t2 correctly Blocked until t1 completes)");

            let board = TaskBoard::new(app_id, "worker", Some("sess-dep".into()), store.clone());
            let c = board
                .claim_next()
                .await
                .ok_or_else(|| "claim first task".to_string())?;
            trace(
                config,
                format!("  claim_next -> {} status={:?}", c.id, c.status),
            );
            if c.id != t1.id {
                return err_msg("claimed task should be t1");
            }
            if !board.start_task(&t1.id).await {
                return err_msg("start_task t1");
            }
            trace(config, "  start_task(t1) -> InProgress");
            if !board.submit_for_review(&t1.id, "done summary".into()).await {
                return err_msg("submit_for_review t1");
            }
            trace(config, "  submit_for_review(t1) -> PendingReview");

            let ok_review = space
                .review_task(
                    &t1.id,
                    "worker",
                    TodoReviewResult {
                        passed: true,
                        feedback: "ok".into(),
                        verified_criteria: vec![],
                    },
                )
                .await;
            if !ok_review {
                return err_msg("review_task returned false");
            }
            trace(
                config,
                "  review_task(passed=true) -> t1 Completed, dependents unblocked",
            );

            let t2_after = store
                .get_todo(&app_id, &Some("sess-dep".into()), "worker", &t2.id)
                .await
                .ok_or_else(|| "reload t2".to_string())?;
            trace(
                config,
                format!("  reload t2 -> status={:?}", t2_after.status),
            );
            if t2_after.status != TodoStatus::Pending {
                return Err(format!(
                    "after t1 completed, t2 should be Pending, got {:?}",
                    t2_after.status
                ));
            }

            let c2 = board
                .claim_next()
                .await
                .ok_or_else(|| "claim second task after unblock".to_string())?;
            trace(config, format!("  claim_next -> {} (expect t2)", c2.id));
            if c2.id != t2.id {
                return err_msg("second claim should be t2");
            }
            ok()
        }
        .await;
        trace_stage_end(config, res.is_ok(), None);
        record("depends_on_review_unblocks_next", res, &mut report);
    }

    // 4) AgenticLoop：ScriptedLlm 调 create_todo
    {
        trace_stage(config, "4/5 AgenticLoop + ScriptedLlm (create_todo)");
        let res = async {
            let space = Arc::new(TaskSpace::new(
                app_id,
                Some("sess-agentic".into()),
                store.clone(),
            ));
            let create = CreateTodoTool {
                space: space.clone(),
                coordinator_name: "coord".into(),
                disallowed_assignees: vec![],
                assignee_capabilities: std::collections::HashMap::new(),
                active_goal_id: None,
            };
            let tools = LocalToolSet(vec![Box::new(create)]);
            let llm = ScriptedLlm::new(
                "script",
                vec![
                    response_with_tools(vec![ToolCall {
                        id: "call-1".into(),
                        name: "create_todo".into(),
                        arguments: json!({
                            "agent": "dry_run_agent",
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
                .list_agent_todos(&app_id, &Some("sess-agentic".into()), "dry_run_agent")
                .await;
            trace(
                config,
                format!(
                    "  list_agent_todos(dry_run_agent) count={} titles={:?}",
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
        record("agentic_loop_create_todo_scripted", res, &mut report);
    }

    // 5) AgenticLoop：claim → start → submit → review（任务 ID 预置，脚本可写死）
    {
        trace_stage(
            config,
            "5/5 AgenticLoop (claim → start → submit → review_todo)",
        );
        let res = async {
            let fixed = TaskId(Uuid::from_u128(0x0f11_e22e_33d4_4556_8777_8899_aabb_ccdd));
            let mut item = TodoItem::new(
                app_id,
                Some("sess-worker".into()),
                "worker",
                "coord",
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

            let board = Arc::new(TaskBoard::new(
                app_id,
                "worker",
                Some("sess-worker".into()),
                store.clone(),
            ));
            let space = Arc::new(TaskSpace::new(
                app_id,
                Some("sess-worker".into()),
                store.clone(),
            ));

            let tools = LocalToolSet(vec![
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
                            "agent": "worker",
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
                .get_todo(&app_id, &Some("sess-worker".into()), "worker", &fixed)
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
        record(
            "agentic_loop_worker_submit_review_scripted",
            res,
            &mut report,
        );
    }

    if config.verbose {
        eprintln!();
        eprintln!("========== stage summary ==========");
        eprintln!(
            "{}",
            PipelineReport {
                stages: report.stages.clone()
            }
            .summary()
        );
        eprintln!("====================================");
    }

    report
}

/// 跑 `AgenticLoop::run_with_events`：与事件 drain 并发，`verbose` 时打印每一跳。
async fn run_agentic_traced(
    cfg: PipelineDryRunConfig,
    label: &str,
    loop_: &AgenticLoop,
    agent_id: &AgentId,
    llm: &ScriptedLlm,
    tools: &LocalToolSet,
    initial_messages: Vec<LlmMessage>,
    options: &LlmOptions,
    permission: &Permission,
) -> MacacaResult<LoopResult> {
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<AgentExecutionEvent>(64);

    let label = label.to_string();
    let drain = async {
        while let Some(ev) = event_rx.recv().await {
            if cfg.verbose {
                eprintln!("  │ [{label}] {}", format_agent_event(&ev));
            }
        }
    };

    let run_fut = loop_.run_with_events(
        agent_id,
        llm,
        tools,
        initial_messages,
        options,
        permission,
        None,
        Some(event_tx),
    );

    let (run_res, ()) = tokio::join!(run_fut, drain);
    run_res
}
