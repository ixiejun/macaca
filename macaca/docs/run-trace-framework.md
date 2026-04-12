# Run trace framework（运行跟踪）

## 目的

在 **不替代** 现有 EventLog（`thinking` / `tool_call` / `delegated_*` …）的前提下，增加一层 **稀疏、结构化** 的检查点，便于回答：

- 当前卡在哪一阶段（聊天入口、工作流、等委派、子任务执行、LLM 失败）？
- 出问题后先看哪几条事件？

## 数据模型

- **存储**：与普通事件一样写入 `EventLog`，`event_type` 固定为 **`run_trace`**，`source` 为 `run_tracer`。
- **载荷**：`macaca_proto::RunTracePayload`（`phase`, `component`, `status`, `message`, `task_id`, `goal_id`, `extra`）。

## 标准 phase（OS 层）

定义见 `macaca-web/src/run_trace.rs` 模块 `phase::`，主要包括：

| Phase | 含义 |
|-------|------|
| `chat.request` | 用户一次 `/api/chat/v2` 任务开始 |
| `workflow.start` | 多步 workflow 开始执行 |
| `coordinator.loop_paused` | Coordinator 因委派进入等待（`waiting`） |
| `coordinator.loop_resumed` | 委派结束恢复 |
| `coordinator.llm_error` | Coordinator LLM 调用失败 |
| `coordinator.stopped` | 用户取消 |
| `coordinator.done` | 本轮 workflow 正常结束 |
| `workflow.error` | `execute_workflow_steps` 返回 Err |
| `delegate.task_start` / `complete` / `failed` | 执行器委派子任务生命周期 |
| `plan.loop_started` | PlanLoop 进程已 spawn |
| `plan.goal_ready` | 从队列取出 goal，准备分解 |
| `plan.goal_delegate` | 已 `delegate_task` 给 plan agent 写子任务 |
| `plan.review_needed` | 有待审任务（`waiting`） |
| `plan.review_delegate` | 已委派 planner 执行 `review_todo` 提示 |
| `plan.all_tasks_done` | 全局任务清空事件 |
| `plan.anomaly` | 失败任务数异常等 |
| `plan.evaluate_goal` | 开始对 goal 做 LLM 评估 |
| `plan.goal_satisfied` / `plan.goal_needs_work` / `plan.goal_eval_fallback` | 评估结果 |
| `plan.goal_completed` | Goal 流程闭环（含 coordinator resume 相关） |
| `goal.create_http` | `POST /api/apps/.../goals` |
| `goal.create_tool` | 工具 `create_goal` 已落库 |
| `worker.task_claimed` / `worker.delegate_start` | WorkerLoop 认领并开始委派执行 |
| `worker.task_success` / `worker.task_failed` | 子 agent 执行结果 |
| `worker.submit_review` | 已 `submit_for_review` |
| `worker.retry_start` / `worker.delegate_error` | 重试与委派失败 |
| `todo.review_decided` | `review_todo` 工具成功更新 store |

无 `session_id` 时写入合成 key：`_macaca_app_{app_uuid}`，便于只走应用级目标时仍能查轨迹。

应用可继续自定义 phase 字符串，无需改 crate。

## API

- `GET /api/sessions/{id}/run-trace?since=&limit=` — 仅返回 `run_trace` 事件（从 `since` 之后扫描，取最近 `limit` 条轨迹）。
- 全量仍用 `GET /api/sessions/{id}/events`。
- Prometheus：`run_trace_events_total{phase,status}`。

## 监控脚本

`macaca/scripts/trace_watch.py`：轮询 `run-trace` 与增量 `events`，打印最近检查点并从增量里扫 `tool_result` / `delegated_*` 错误。

## 与「应用配置」的配合

Coordinator 需在 `file_read` 工作流模板时使用 **应用 bundle 绝对路径**；`post_chat` / `run_agentic_stream_with_agent_for_step` 已在 system prompt 注入 **Application bundle root**，避免相对路径 `workflows/…` 解析到错误 cwd。
