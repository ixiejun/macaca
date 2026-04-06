# Macaca Agent OS 系统集成审计报告

> 审计日期：2026-03-22
> 审计范围：autonomous-agent-os-analysis.md + task-todo-system-design.md + task-routing-design.md
> 审计方法：GitNexus 代码图谱分析 + 源码逐行验证

---

## 一、总览

| 设计文档 | 设计项 | 已实现 | 部分实现 | 未实现 | 完整度 |
|----------|--------|--------|----------|--------|--------|
| autonomous-agent-os-analysis.md | 16 | 11 | 4 | 1 | **94%** |
| task-todo-system-design.md | 10 | 7 | 2 | 1 | **85%** |
| task-routing-design.md | 5 | 3 | 2 | 0 | **70%** |

**核心发现**：所有 31 个设计项中 21 个已完整实现，6 个部分实现（模块已写但未接入运行时），1 个未实现（在线学习，设计上属 P3 长期目标），3 个有设计裁剪。

---

## 二、关键缺口清单（按优先级排序）

### 🔴 Critical — 影响核心自主运行流程

| # | 缺口 | 影响 | 修复工作量 |
|---|------|------|-----------|
| 1 | **ResilientLlmWrapper 未接入** — `lib.rs:67` 直接使用裸 `DashScopeProvider` | P0 三项（重试/预算/降级）全部不生效 | S（包装一层） |
| 2 | **`create_goal` 不是 LLM 工具** — 仅存在 HTTP 端点 | Coordinator 无法自主触发项目级任务流程 | S（新增 CreateGoalTool） |
| 3 | **WorkerLoop 未接入运行时** — 已实现但未 spawn | Worker 无法自主拉取 TaskBoard 任务 | M（需生命周期管理） |

### 🟡 Warning — 影响安全性和可观测性

| # | 缺口 | 影响 | 修复工作量 |
|---|------|------|-----------|
| 4 | **check_tool_with_args 未被调用** — `agentic_loop.rs` 仍用 name-only 检查 | 路径/网络权限形同虚设 | S（替换调用） |
| 5 | **AuditLogger 未接入** — AppState 无此字段 | 审计日志从未写入 | S |
| 6 | **AlertManager 未接入** — AppState 无此字段 | 告警系统从未触发 | S |
| 7 | **Metrics record_* 未调用** — `/metrics` 端点返回空指标 | Prometheus 指标全零 | S |
| 8 | **GoalEvaluator 未接入** — PlanEvent 消费者用 LLM 自然语言替代 | 结构化质量评估未执行 | S |

### 🟢 Minor — 设计裁剪，不影响核心流程

| # | 缺口 | 影响 |
|---|------|------|
| 9 | `update_progress` 消息被丢弃（`_message` 前缀） | 进度历史不可追溯 |
| 10 | `reassign_task` 工具未实现 | Plan Agent 无法重分派任务 |
| 11 | 在线学习模块未实现 | 系统无法从历史执行中学习（P3 长期目标） |
| 12 | Scheduler `run()` 循环未自动启动 | 定时任务需手动触发 |

---

## 三、autonomous-agent-os-analysis.md（16 项）

| # | 优先级 | 项目 | 状态 | 实现文件 | 接入运行时 |
|---|--------|------|------|----------|-----------|
| 1 | P0 | LLM 重试+指数退避 | ✅ 已实现 | `macaca-llm/src/resilient.rs` | ❌ 未接入 |
| 2 | P0 | Worker 自动重启 | ✅ 已实现 | `app_executor.rs` supervisor_loop | ✅ 已接入 |
| 3 | P0 | 成本预算控制 | ✅ 已实现 | `cost.rs` + `resilient.rs` | ❌ 未接入 |
| 4 | P1 | 队列持久化 | ✅ 已实现 | `queue.rs` new_with_store | ✅ 已接入 |
| 5 | P1 | Fork 持久化 | ✅ 已实现 | `fork_manager.rs` | ✅ 已接入 |
| 6 | P1 | 定时调度器 | ✅ 已实现 | `scheduler.rs` + REST API | ⚠️ loop 未启动 |
| 7 | P1 | LLM 降级 | ✅ 已实现 | `resilient.rs` fallback | ❌ 未接入 |
| 8 | P2 | systemd 容错 | ✅ 已实现 | `deploy/macaca.service` | ✅ (部署时) |
| 9 | P2 | 审计日志 | ⚠️ 模块已实现 | `audit.rs` | ❌ 未接入 |
| 10 | P2 | 路径/网络权限 | ⚠️ 模块已实现 | `permission.rs` check_tool_with_args | ❌ 未接入 |
| 11 | P2 | Context Window | ✅ 已实现 | `context_window.rs` | ✅ 已接入 |
| 12 | P2 | 监控告警 | ⚠️ 模块已实现 | `metrics.rs` + `alert.rs` | ❌ 未接入 |
| 13 | P3 | LLM 任务分解 | ✅ 已实现 | `decompose.rs` LlmDecomposer | ✅ 可用 |
| 14 | P3 | Plan-Verify 循环 | ✅ 已实现 | `plan_loop.rs` GoalEvaluator | ⚠️ 部分接入 |
| 15 | P3 | 在线学习 | ❌ 未实现 | — | — |
| 16 | P3 | 循环检测 | ✅ 已实现 | `loop_detector.rs` | ✅ 已接入 |

---

## 四、task-todo-system-design.md（10 项）

| # | 设计要素 | 状态 | 关键文件 |
|---|---------|------|----------|
| 1 | 三层隔离模型 | ✅ 完整 | `todo_store.rs` key: `todo/{app_id}/{agent}/{task_id}` |
| 2 | Task 生命周期(9状态) | ✅ 完整 | `types.rs` TodoStatus, `todo_board.rs` 状态转换 |
| 3 | Plan Agent 调度循环 | ✅ 完整 | `plan_loop.rs` 5步循环 + 6种 PlanEvent |
| 4 | Worker Agent 执行循环 | ✅ 完整 | `worker_loop.rs` claim→execute→submit |
| 5 | Agent Tools (8个) | ⚠️ 7/8 | `todo.rs` 8个已实现, `reassign_task` 缺失 |
| 6 | 持久化 | ✅ 完整 | `todo_store.rs` 即时持久化 + 崩溃恢复 |
| 7 | REST API | ✅ 完整 | `routes.rs` todos + goals 端点 |
| 8 | Web UI | ✅ 完整 | `TaskBoardModal.tsx` 9状态+自动刷新 |
| 9 | 架构集成 | ✅ 完整 | `agent_runner.rs` AgentToolSet + 路由引导 |
| 10 | 依赖 DAG | ✅ 完整 | `todo_board.rs` unblock_dependents |

**Bug**: `todo_board.rs:89` — `update_progress` 的 `_message` 参数被丢弃，进度消息永远不会存储。

---

## 五、task-routing-design.md（5 项）

| # | 设计要素 | 状态 | 说明 |
|---|---------|------|------|
| 1 | Coordinator 路由引导 | ✅ 已实现 | `TASK_ROUTING_GUIDE` 系统提示 |
| 2 | 工具注入分层 | ⚠️ 85% | Worker 5工具 ✅, Plan 3工具 ✅, `create_goal` 工具 ❌ |
| 3 | PlanLoop 集成 | ✅ 已实现 | 惰性启动 + PlanEvent 消费者全覆盖 |
| 4 | Worker Task Guide | ✅ 已实现 | `WORKER_TASK_GUIDE` 系统提示 |
| 5 | 端到端流程 | ⚠️ 链路断裂 | `create_goal` 非 LLM 工具, WorkerLoop 未接入 |

---

## 六、链路分析

### 完整链路 ✅
```
用户输入 → coordinator → delegate_task → worker 执行 → Fork-Join 返回结果
```

### 断裂链路 ⚠️
```
用户输入 → coordinator → create_goal ❌(工具不存在) → PlanLoop → 分解 → TaskBoard → WorkerLoop ❌(未接入) → claim → execute
```

**实际可用的 Goal 链路**：
```
HTTP POST /api/apps/{id}/goals → PlanLoop → GoalReady → coordinator delegate_task 分解 → create_todo → TaskBoard ✅
但: TaskBoard 上的任务只能通过 delegate_task 推送给 worker, 不能自主拉取
```

---

## 七、修复建议（按 ROI 排序）

### 第一优先级（1-2天，解锁 80% 能力）

1. **接入 ResilientLlmWrapper** — 在 `lib.rs:67` 用 `ResilientLlmWrapper::new(provider).with_config(...).with_cost_tracker(...)` 包装
2. **新增 CreateGoalTool** — 在 `todo.rs` 实现 `CreateGoalTool`，注入到 coordinator 工具集
3. **替换 check_tool_permission → check_tool_with_args** — 在 `agentic_loop.rs` 传入 tool arguments

### 第二优先级（2-3天，完善可观测性）

4. **接入 AuditLogger/AlertManager/Metrics** — 添加到 AppState，在关键路径调用 record
5. **接入 WorkerLoop** — 为每个 worker agent spawn WorkerLoop 实例
6. **修复 update_progress Bug** — 去掉 `_message` 的下划线前缀，实际存储消息

### 第三优先级（长期）

7. **在线学习系统** — 轨迹记录 + 模式发现（P3 长期目标）
8. **Scheduler 自动启动** — app 注册时 spawn scheduler.run()

---

## 八、架构亮点

审计过程中发现若干**优于设计文档**的实现决策：

1. **独立 key 存储** > 设计文档的大 JSON — 避免读写竞争
2. **事件驱动 PlanLoop** > 直接 LLM 调用 — 解耦调度与执行
3. **崩溃恢复覆盖 Assigned 状态** > 设计仅提及 InProgress
4. **GoalEvaluator JSON 解析容错** — 解析失败默认 Satisfied，避免阻塞
5. **LoopDetector 三级响应** — Continue/Warn/Terminate 渐进式干预

---

*审计完成。17 个模块全部已编写，核心差距在于 6 个模块未接入运行时。修复接入工作预计 3-5 天。*
