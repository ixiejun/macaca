# Macaca Agent OS — System Audit Report

> **Scope:** 这是一份 **current-state execution audit / refactor-action** 文档，服务于后续重构执行，不是系统定义的 canonical 来源。
>
> 系统是什么、解决什么问题、核心模块与任务执行链路，请先看 [`SYSTEM_OVERVIEW.md`](./SYSTEM_OVERVIEW.md)。
>
> 本文档聚焦：当前实现证据、优先级风险、以及与系统原则相连的重构行动依据。

> 审计日期: 2026-04-07 | 总代码量: ~39,000 行 Rust + Next.js 前端

<a id="audit-context"></a>
## Audit Context

### 当前实现快照

```
CLI (macaca-cli) -> Web (macaca-web) -> Kernel (macaca-kernel) -> Runtime (macaca-runtime)
                                     -> Task (macaca-task)     -> LLM (macaca-llm)
                                     -> App (macaca-app)       -> Persist (macaca-persist)
                                     -> Tools (macaca-tools)   -> Proto (macaca-proto)
```

### 当前可见执行路径（用于审计上下文，不是 canonical 系统定义）

1. **即时委派**: User -> `post_chat` -> Coordinator AgenticLoop -> `delegate_task` -> Fork-Join -> Worker AgenticLoop
2. **目标级**: User -> `create_goal` -> PlanLoop 分解 -> TodoStore -> WorkerLoop claim -> AgenticLoop 执行 -> PlanLoop review

### 审计关注点

本次审计不重新定义系统目标，而是检查：
- 当前实现是否违背了系统 overview 中的关键原则
- 哪些结构性问题会把系统继续推向难维护、高耦合、低可观测性的方向
- 哪些重构建议最值得优先推进

---

<a id="crate-by-crate-audit"></a>
## Crate-by-Crate Audit

| Crate | 用途 | 完成度 | 行数 | 当前主要问题 |
|-------|------|--------|------|-------------|
| **macaca-proto** | 共享类型、配置、错误 | 100% | ~1,200 | `orchestration.rs:243` 硬编码 agent 名 |
| **macaca-persist** | redb KV 存储、EventLog | 100% | ~500 | 无 |
| **macaca-llm** | LLM 抽象、多 provider 路由、降级 | 100% | ~1,500 | 定价表硬编码 |
| **macaca-runtime** | AgenticLoop、上下文窗口、循环检测 | 100% | ~1,200 | 3 个 loop 实现约 60% 重复 |
| **macaca-task** | TodoBoard、PlanLoop、WorkerLoop、分解器 | 100% | ~1,800 | TaskTracker 可能是死代码；TaskQueue 与 kernel ExecutionQueue 重叠 |
| **macaca-kernel** | Executor、ForkManager、审计、告警 | 100% | ~2,500 | 重复 TaskId/DelegatedTask 类型；orchestrator 硬编码 `coordinator` |
| **macaca-tools** | 内置工具、编排工具、Todo 工具 | 100% | ~800 | 无 |
| **macaca-web** | Axum HTTP、SSE、agent runner | 100% | ~5,600 | `routes.rs` 近 5,000 行；AppState 27 字段；30+ `coordinator` 硬编码 |
| **macaca-app** | 应用模型、加载器、工作流 | 95% | ~800 | L2 WASM 存根 |
| **macaca-cli** | CLI 入口 | 100% | ~300 | 无 |
| **macaca-gateway** | IM 网关 (Telegram/Discord) | 30% | ~400 | 未接入主服务启动路径 |
| **macaca-agent** | Agent trait、状态机 | 100% | ~300 | 无 |
| **macaca-memory** | 三层记忆系统 | 50% | ~600 | 未接入 Agent 执行路径 |
| **macaca-ipc** | 进程间通信 | 40% | ~300 | 未接入主执行路径 |
| **macaca-mcp** | MCP 客户端 | 20% | ~200 | 未集成到工具加载 |
| **macaca-sdk** | 声明式 Agent SDK | 100% | ~400 | 无 |
| **macaca-skill** | 技能发现与注册 | 100% | ~500 | `discovery.rs` 有 `dead_code` 注解 |
| **macaca-driver** | 驱动框架 + Claude Code | 100% | ~600 | `dangerously_skip_permissions()` 安全隐患 |
| **macaca-integration-tests** | 集成测试 | 80% | ~400 | ScriptedLlm 干运行覆盖较好 |

---

<a id="frontend-audit"></a>
## Frontend Audit

### 组件状态

| 组件 | 用途 | 当前观察 |
|------|------|----------|
| `app/page.tsx` | 首页应用发现 | 正常 |
| `app/chat/[appId]/page.tsx` | 主聊天工作区 (~600 行) | 有 `console.log` 调试代码 |
| `components/AgentPanel.tsx` | Agent 状态面板 | 正常 |
| `components/ConversationTurn.tsx` | 对话轮次渲染 | `renderValue()` 重复 |
| `components/DelegatedAgentTrace.tsx` | 委派 Agent 追踪 | `renderValue()` 重复 |
| `components/TaskBoardModal.tsx` | 任务面板弹窗 | 正常 |
| `components/Sidebar.tsx` | Session 列表 | 正常 |
| `components/InputArea.tsx` | 输入区域 | 正常 |
| `components/Message.tsx` | 消息气泡 | 疑似死代码，被 `ConversationTurn` 取代 |

### 当前前端问题

1. `renderValue()` 在 `ConversationTurn` 与 `DelegatedAgentTrace` 中重复
2. `Message.tsx` 疑似死代码
3. `chat/[appId]/page.tsx` 仍有 debug `console.log`
4. Port `3001` 存在硬编码 fallback
5. 缺少 Error Boundary
6. 缺少 Loading Skeleton

---

<a id="cross-cutting-analysis"></a>
## Cross-Cutting Analysis

### 当前任务执行路径（实现证据）

| 路径 | 触发方式 | 状态追踪 | Session 感知 | 审计含义 |
|------|----------|----------|-------------|----------|
| **A: Fork-Join** | coordinator -> `delegate_task` 工具 | 有 (pause/resume) | 有 (`fork_to_session`) | 说明系统已经有委派/恢复语义，但入口与配置边界仍不稳定 |
| **B: Goal-Task** | coordinator -> `create_goal` -> PlanLoop | 有 (`goal_to_session`) | 部分 (跨 session) | 说明系统已具备任务级执行骨架 |
| **C: Workflow** | 配置驱动的顺序执行 | 无 | 无 | 表明 workflow 仍未形成统一、可观测的主路径 |

### 状态存储位置

| 状态 | 存储 | 持久性 | 审计含义 |
|------|------|--------|----------|
| Agent manifests | Kernel.registry (内存) | 重启丢失 | 注册能力与运行时状态未完全统一 |
| 对话历史 | AppState.sessions + RedbStore | 持久 | 会话历史已有持久化基础 |
| Todo items/goals | TodoStore -> RedbStore | 持久 | 任务系统已有 durable substrate |
| Fork contexts | ForkManager.forks (内存) | 重启丢失 | fork/rejoin 仍依赖内存态 |
| Active sessions | AppState.active_sessions (内存) | 重启丢失 | session 恢复语义不够稳固 |
| Event log | EventLog -> RedbStore | 持久 | 可观测性底座存在 |
| Agent status | AgentStatusTracker (内存) | 重启丢失 | 端到端状态一致性仍不够强 |
| Audit events | AuditLogger -> RedbStore | 持久 | 审计能力是真实实现，不是概念占位 |

### Session 隔离

- **Session 级**: TodoStore、对话历史、EventLog、cancel flags、active sessions
- **App 级**: Kernel、executor registry、tool set、LLM provider、plan/worker loop
- **全局**: RedbStore 实例、配置

---

## Technical Debt Summary

<a id="p0-risks"></a>
### P0 — 架构风险

| 问题 | 影响 | 位置 | Protects |
|------|------|------|----------|
| `routes.rs` 4,993 行巨型文件 | 不可测试、难扩展、耦合 HTTP 层与业务逻辑 | `macaca-web/src/routes.rs` | [`Principle 1 — Bounded module responsibility`](./SYSTEM_OVERVIEW.md#principle-1-bounded-module-responsibility), [`Principle 3 — Observable end-to-end execution`](./SYSTEM_OVERVIEW.md#principle-3-observable-end-to-end-execution) |
| AppState 27 字段 God Object | 所有状态混在一起，职责不清 | `macaca-web/src/state.rs:56-114` | [`Principle 1 — Bounded module responsibility`](./SYSTEM_OVERVIEW.md#principle-1-bounded-module-responsibility) |
| 30+ `coordinator` 硬编码 | 违反 OS 底座通用性原则 | `routes.rs`, `orchestrator.rs`, `decompose.rs` | [`Principle 2 — Config-driven entry and orchestration`](./SYSTEM_OVERVIEW.md#principle-2-config-driven-entry-and-orchestration) |

<a id="p1-duplication-and-redundancy"></a>
### P1 — 重复/冗余

| 问题 | 影响 | 位置 | Protects |
|------|------|------|----------|
| TaskId 重复定义 | 类型混淆、需要转换 | `macaca-proto::types` vs `macaca-kernel::executor::mod` | [`Principle 4 — Shared protocol and task primitives`](./SYSTEM_OVERVIEW.md#principle-4-shared-protocol-and-task-primitives) |
| DelegatedTask 重复定义 | 字段集不同但同名 | `macaca-proto::orchestration` vs `macaca-kernel::executor::mod` | [`Principle 4 — Shared protocol and task primitives`](./SYSTEM_OVERVIEW.md#principle-4-shared-protocol-and-task-primitives) |
| AgenticLoop 3 个 run 变体约 60% 重复 | 维护成本高 | `macaca-runtime/src/agentic_loop.rs` | [`Principle 3 — Observable end-to-end execution`](./SYSTEM_OVERVIEW.md#principle-3-observable-end-to-end-execution), [`Principle 4 — Shared protocol and task primitives`](./SYSTEM_OVERVIEW.md#principle-4-shared-protocol-and-task-primitives) |
| TaskTracker (可能死代码) | 被 TodoStore/TaskBoard 取代 | `macaca-task/src/tracker.rs` | [`Principle 1 — Bounded module responsibility`](./SYSTEM_OVERVIEW.md#principle-1-bounded-module-responsibility) |
| TaskQueue 与 ExecutionQueue 功能重叠 | 两套队列系统 | `macaca-task/src/queue.rs` vs `macaca-kernel/src/executor/queue.rs` | [`Principle 4 — Shared protocol and task primitives`](./SYSTEM_OVERVIEW.md#principle-4-shared-protocol-and-task-primitives) |

<a id="p2-not-integrated"></a>
### P2 — 未接入模块

| 模块 | 状态 | 说明 | Protects |
|------|------|------|----------|
| macaca-memory | 50% 实现，未接入 | Agent 执行时无记忆检索 | [`Principle 5 — Pluggable capabilities and platform surfaces`](./SYSTEM_OVERVIEW.md#principle-5-pluggable-capabilities-and-platform-surfaces) |
| macaca-ipc | 40% 实现，未接入 | LocalBus 可用但未使用 | [`Principle 5 — Pluggable capabilities and platform surfaces`](./SYSTEM_OVERVIEW.md#principle-5-pluggable-capabilities-and-platform-surfaces) |
| macaca-mcp | 20% 实现，未接入 | 未集成到工具加载管线 | [`Principle 5 — Pluggable capabilities and platform surfaces`](./SYSTEM_OVERVIEW.md#principle-5-pluggable-capabilities-and-platform-surfaces) |
| macaca-gateway | 30% 实现，未接入 | Telegram/Discord 未在 server 启动 | [`Principle 5 — Pluggable capabilities and platform surfaces`](./SYSTEM_OVERVIEW.md#principle-5-pluggable-capabilities-and-platform-surfaces) |

---

<a id="refactoring-recommendations"></a>
## Refactoring Recommendations

### 1. 拆分 `routes.rs`（高优先级）

**Protects:** [`Principle 1`](./SYSTEM_OVERVIEW.md#principle-1-bounded-module-responsibility), [`Principle 3`](./SYSTEM_OVERVIEW.md#principle-3-observable-end-to-end-execution)

目标: `routes.rs` 从约 5,000 行降到约 500 行

```
macaca-web/src/
├── routes.rs              # 薄路由处理 (~500 行)
├── chat_orchestrator.rs   # post_chat SSE 流 + AgenticLoop 驱动
├── loop_manager.rs        # PlanLoop/WorkerLoop 生命周期管理
├── sse.rs                 # SSE 事件转换 + 广播
├── workflow.rs            # 工作流步骤执行
└── session.rs             # Session CRUD + EventLog 重建
```

### 2. 消除 `coordinator` 硬编码

**Protects:** [`Principle 2`](./SYSTEM_OVERVIEW.md#principle-2-config-driven-entry-and-orchestration)

- 入口 Agent 名应从 app manifest 读取（`entry_agent` 配置）
- PlanLoop consumer 的 prompt 不应再硬编码 agent 列表（已部分修复）

### 3. 合并重复类型

**Protects:** [`Principle 4`](./SYSTEM_OVERVIEW.md#principle-4-shared-protocol-and-task-primitives)

- `TaskId` / `DelegatedTask` 统一到 `macaca-proto`
- kernel 通过 re-export 使用

### 4. 精简 AppState

**Protects:** [`Principle 1`](./SYSTEM_OVERVIEW.md#principle-1-bounded-module-responsibility)

```rust
pub struct AppState {
    pub kernel: Arc<Kernel>,
    pub llm: Arc<dyn LlmProvider>,
    pub tools: Box<dyn ToolSet>,
    pub persistence: PersistenceState,    // session_store, todo_store, event_log, audit_logger
    pub loops: LoopState,                 // plan_loop_handles, worker_loop_handles, wakers
    pub sessions: SessionState,           // active_sessions, cancel_flags, fork_to_session, goal_to_session
    pub executor_registry: Arc<ApplicationExecutorRegistry>,
    pub config: AppConfig,                // app_dirs, default_model, skills_catalog
}
```

### 5. 提取 AgenticLoop 共享逻辑

**Protects:** [`Principle 3`](./SYSTEM_OVERVIEW.md#principle-3-observable-end-to-end-execution), [`Principle 4`](./SYSTEM_OVERVIEW.md#principle-4-shared-protocol-and-task-primitives)

```rust
// 提取共享的迭代逻辑
async fn run_iteration(&self, ...) -> IterationResult { ... }

// 三个变体只处理各自的前/后处理
pub async fn run(&self, ...) { loop { self.run_iteration(...).await } }
pub async fn run_with_events(&self, ...) { loop { emit(event); self.run_iteration(...).await } }
pub async fn run_with_pause(&self, ...) { loop { check_pause(); self.run_iteration(...).await } }
```

### 6. 接入或标记未使用模块

**Protects:** [`Principle 5`](./SYSTEM_OVERVIEW.md#principle-5-pluggable-capabilities-and-platform-surfaces)

- `macaca-memory`: 在 agent_runner 的 build_agent_toolset 中注入记忆检索
- `macaca-ipc`: 在 PlanEvent/WorkerEvent 之上使用
- `macaca-mcp`: 集成到 CompositeToolSet 的工具加载
- `macaca-gateway`: 在 start_server 中条件启动
