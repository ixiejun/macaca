---
name: macaca-dev-guide
description: "Macaca Agent OS 项目权威开发指南。在编辑任何 macaca-* Rust crate、frontend/ Next.js 应用、fullstack-autodev 示例时必须使用此 skill。触发场景：编辑 macaca 代码、调试 agent 执行、添加 OS 平台功能、前端 UI 开发、任务系统改动、LLM 提供商集成、理解执行流程、任何关于 Macaca 系统如何工作的问题。做架构决策前务必先查阅此文档。"
---

# Macaca Agent OS — 开发指南

> 本项目所有开发工作的唯一事实来源。
> 详细的 crate 审计见 `references/architecture.md`，API/前端规范见 `references/api-and-frontend.md`。

## 快速定位

Macaca 是一个**通用 Agent 操作系统平台**（不是某个特定应用）。`macaca-*` crate 是 OS 底座，必须支持任意应用。`examples/apps/` 中的 `fullstack-autodev` 只是一个测试应用。

```
agent/                          # 项目根目录
├── frontend/                   # 前端 — Next.js 16 + React 19 + Tailwind CSS（端口 3000）
├── macaca/                     # 后端 — Rust workspace（端口 3001）
│   ├── crates/                 # 20 个 crate — OS 底座
│   ├── config/default.toml     # 运行时配置（提供商、模型、端口）
│   └── examples/apps/          # 测试应用（fullstack-autodev）
└── openspec/                   # 规范驱动开发
```

## 黄金准则

**绝对禁止在 OS crate 中硬编码应用专有逻辑。** 不允许出现 agent 名称（"coordinator"、"backend"、"architect"）、路由策略、prompt、模型名。自检："如果别人用这套底座做量化交易应用而不是 fullstack-autodev，这段代码还能工作吗？"

## 执行模型 — 3 条委派路径

### 路径 A：Fork-Join（即时委派）
```
用户 → post_chat → 协调者 AgenticLoop → delegate_task 工具
  → DelegateTaskTool 回调 → ApplicationExecutor.delegate_task
  → ForkManager.create_fork → Worker AgenticLoop 执行
  → ForkValidated 钩子 → HookConsumer → ResumeReason → 协调者恢复
```
- **暂停/恢复**: 协调者在 delegate_task 后设 `pause_signal=true`，阻塞在 `resume_rx.recv()`
- **代码位置**: `routes.rs` post_chat、`hook_consumer.rs`、`fork_manager.rs`

### 路径 B：目标-任务（项目级）
```
用户 → 协调者调用 create_goal → TodoStore → PlanLoop 检测目标
  → PlanEvent::GoalReady → 委派给 planner → planner 调用 create_todo
  → 创建 TodoItem → WorkerLoop 认领任务 → Worker 执行
  → submit_for_review → PlanLoop ReviewNeeded → planner 审核
  → GoalEvaluator → GoalCompleted → 协调者恢复
```
- **暂停/恢复**: 协调者在 create_goal 后暂停，GoalCompleted 时通过 `goal_to_session` 映射恢复
- **代码位置**: `plan_loop.rs`、`worker_loop.rs`、`todo_board.rs`、`routes.rs` ensure_plan_and_worker_loops

### 路径 C：工作流（配置驱动）
```
应用 manifest 定义工作流步骤 → routes.rs 中 execute_workflow_steps
  → 按工作流配置顺序执行 agent
```

## 任务系统

### 顺序执行
- `TodoItem.sequence_number: u32` — agent+session 内的执行顺序（从 1 开始）
- `TaskBoard.claim_next()` 按序号排序，**每个 session 独立**强制顺序
- 不同 session 互不阻塞
- `TaskSpace.create_and_assign()` 自动分配递增序号

### 任务生命周期
```
Pending → Assigned → InProgress → PendingReview → Completed
                                                 → NeedsOptimization（重试）
                                                 → Failed（超过最大尝试次数）
Blocked（依赖未满足）→ Pending（依赖完成后解除）
Cancelled（TERMINATE 或 skip_task）
```

### PlanLoop 去重机制
- `review_emitted: HashSet<TaskId>` — ReviewNeeded 每个任务只 emit 一次
- `goal_retry_emitted: HashSet<TaskId>` — 无任务的 InProgress 目标只重试一次
- `last_failed_count` — AnomalyDetected 仅在失败数变化时触发

### Worker → PlanLoop → Worker 唤醒链
- Worker 完成 → `submit_for_review` → `PlanLoopWaker::wake()`（即时）
- PlanLoop review 委派完成 → `WorkerLoopWaker::wake()`（即时）
- 正常流程无 5 秒心跳延迟

## 状态管理

### AppState（27 字段 → 即将重构为子结构分组）

| 分组 | 字段 | 持久性 |
|------|------|--------|
| 核心 | kernel, llm, tools, executor_registry | 内存 |
| 持久化 | session_store, todo_store, event_log, audit_logger, run_tracer | redb（持久） |
| 循环 | plan_loop_handles, worker_loop_handles, plan/worker_loop_wakers, scheduler_handles | 内存 |
| 会话 | active_sessions, cancel_flags, fork_to_session, goal_to_session, sessions | 内存 |
| 配置 | app_dirs, app_workspaces, default_model, skills_catalog, alert_manager | 内存 |

### Session 隔离
- **Session 级**: TodoStore 键、对话历史、EventLog、cancel flags
- **App 级**: Kernel、executor、工具集、LLM 提供商、PlanLoop/WorkerLoop
- **全局**: RedbStore 实例、配置

## 编码规范

### 必须遵守
1. OS crate 中禁止硬编码 agent 名称
2. 每次编辑后 `cargo check`，实现后 `cargo test -p macaca-<crate>`
3. TodoStore 中每次状态变更 → 立即 `save_todo()`（崩溃安全）
4. EventLog append 必须在 SSE send 之前（持久性保证）
5. 所有 LLM HTTP 客户端使用 `no_proxy()`（绕过系统代理）

### 调试清单
功能不工作时按此顺序排查：
1. 模块已实现？（源文件存在）
2. 模块已导出？（lib.rs 中 `pub mod` + `pub use`）
3. 模块已接入运行时？（AppState / agent_runner.rs）
4. 工具已注入？（`build_agent_toolset()` 中的 extra）
5. LLM 能看到工具？（`to_definitions()` 返回值包含）
6. Persona 引导正确？（该 agent 的 TOOLS.md）
7. 事件被消费？（PlanEvent/WorkerEvent 消费者存在）
8. PlanLoop/WorkerLoop 在运行？（`ensure_plan_and_worker_loops` 已调用）
9. TERMINATE 之后？（handles 已移除，新循环可以启动）

### TERMINATE 行为
- 设置 cancel_flags（协调者停止）
- executor.shutdown()（委派任务停止）
- 移除 PlanLoop/WorkerLoop handles（允许重启）
- 重置所有 agent 状态为 Idle
- 取消所有非终态任务（Pending/Blocked/InProgress → Cancelled）
- 取消所有 InProgress/Pending 目标

## 关键设计模式

| 模式 | 位置 | 用途 |
|------|------|------|
| ResilientLlmWrapper | `macaca-llm/src/resilient.rs` | 重试 + 退避 + 预算 + 降级链 |
| LoopDetector | `macaca-runtime/src/loop_detector.rs` | SHA256 哈希检测重复工具调用 |
| ContextWindowManager | `macaca-runtime/src/context_window.rs` | Token 估算 + 截断 |
| WorkerSupervisor | `macaca-kernel/src/executor/app_executor.rs` | 自动重启 Worker（最多 5 次） |
| Fork-Join 暂停/恢复 | `routes.rs` + `hook_consumer.rs` | 协调者等待委派完成 |
| EventLog 先于 SSE | 所有事件发射点 | 持久性：先持久化再流式传输 |
| 拉取式 WorkerLoop | `macaca-task/src/worker_loop.rs` | Agent 主动拉取任务（K8s 模式） |

## 端口与网络配置

| 服务 | 端口 |
|------|------|
| 前端（Next.js） | 3000 |
| 后端（Macaca Rust API） | 3001 |

- 前端通过 `NEXT_PUBLIC_API_BASE` 代理到后端 3001 端口
- 所有 LLM HTTP 客户端使用 `reqwest::Client::builder().no_proxy().build()` 绕过系统代理
- 配置文件: `config/default.toml` — 提供商、模型、API 密钥

## 已知技术债务

详见 `references/architecture.md` 的"技术债务优先级"章节。摘要：
- `routes.rs` 4,993 行 — 拆分计划已提案（refactor-core-architecture）
- AppState 27 字段 — God Object，子结构分组已计划
- 30+ 处 "coordinator" 硬编码 — entry_agent 动态查找已计划
- 重复 `TaskId`/`DelegatedTask` 定义（proto vs kernel）
- AgenticLoop 3 个 run 变体 60% 代码重复
- 4 个未接入模块：memory、ipc、mcp、gateway

## 常用命令

```bash
# 后端
cd macaca && cargo check                       # 快速编译检查
cd macaca && cargo test -p macaca-<crate>       # 测试特定 crate
cd macaca && cargo run --bin macaca -- web      # 启动后端（端口 3001）

# 前端
cd frontend && npx next dev --port 3000        # 启动前端开发服务器
cd frontend && npx next build                  # 生产构建

# 重启后端（后台运行）
ps aux | grep "macaca web" | grep -v grep | awk '{print $2}' | xargs kill 2>/dev/null
sleep 1 && cargo run --bin macaca -- web 2>/tmp/macaca-backend.log &

# GitNexus
npx gitnexus analyze                           # 重建代码索引
```
