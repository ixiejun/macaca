# macaca-kernel 设计模式渐进式重构 Brainstorm 设计记录

## 背景

`macaca-kernel` 是 Agent OS 的系统协调中枢，当前负责 agent registry、kernel facade、scheduler、status tracker、orchestrator、executor、worker、fork manager、event bus、callback dispatcher 等能力。

根据 `macaca/docs/design-pattern-refactor-plans/refactor-order.md`，`macaca-kernel` 位于阶段 4：应用语义与系统协调层。它依赖多个已逐步重构过的底层 crate，并被 `macaca-web`、`macaca-app`、`macaca-sdk`、`macaca-cli` 和 integration tests 消费。因此 kernel 的任何重构都必须小切片、可回滚、行为 1:1 还原。

当前源码观察：

- `macaca/crates/macaca-kernel/src/kernel.rs`：`Kernel` 已经是对外 facade，但构造时固定 `SimpleScheduler`，没有 builder/factory 注入边界。
- `macaca/crates/macaca-kernel/src/scheduler.rs`：已有 `Scheduler` trait 和 `SimpleScheduler` strategy，但缺少 `SchedulerFactory` 和可扩展 selection contract。
- `macaca/crates/macaca-kernel/src/status.rs`：`AgentStatusTracker` 集中更新 agent lifecycle/activity，但 transition policy 由调用方随意调用 helper，缺少显式状态机。
- `macaca/crates/macaca-kernel/src/executor/mod.rs`：定义 `ExecutorEvent`、`TaskResult`、`AgentRunner` 等核心事件/执行 contract。
- `macaca/crates/macaca-kernel/src/executor/app_executor.rs`：1268 行，集中承担 worker supervisor、command handling、event forwarding、TaskResult 构造、fork resume、broadcast 等职责，超过项目 500 行约束。
- `macaca/crates/macaca-kernel/src/executor/fork_manager.rs`：921 行，超过项目 500 行约束，但本轮不优先拆，避免和 executor lifecycle 重构交叉。
- `macaca/crates/macaca-web/src/loop_manager.rs` 里已经有 web-local `executor_task_started` / `executor_task_completed` / `executor_task_failed` helper，说明 lifecycle event 构造已经从 kernel 外泄到了 web 层。

当前消费关系：

- `Kernel::new` 被 `macaca-web`、`macaca-app`、`macaca-sdk`、`macaca-cli`、`macaca-kernel` tests、integration tests 多处调用。
- `ApplicationExecutor` / `ApplicationExecutorRegistry` 被 `macaca-web` 作为执行与 SSE/EventLog 桥接基础使用。
- `ExecutorEvent` 被 `macaca-web` 的 SSE、event persistence、loop manager、session restore 等路径消费。
- `SimpleScheduler` 当前主要在 kernel 内部构造和 scheduler 单测中使用。

GitNexus 观察：

- 图谱能定位 `Kernel`、`ApplicationExecutor`、`SimpleScheduler`、`AgentStatusTracker` 等核心 symbol，但部分最新未提交变更可能未被索引覆盖。
- 后续实施前仍必须对每个要修改的 symbol 跑 `gitnexus_impact`；尤其 `Kernel::new` 和 `ApplicationExecutor` 预计风险较高。

## 设计模式适配

本轮计划采用已有文档指定的模式，不发明新的复杂架构：

- **Facade**：保持 `Kernel` 是唯一对外内核门面，逐步减少上层对 registry/scheduler/status 内部细节的直接理解。
- **Factory Method / Builder**：为 `ExecutorEvent` / `TaskResult` lifecycle payload 建立工厂，后续再为 scheduler/kernel 构造建立 factory/builder。
- **Strategy**：把 scheduler selection 从固定 `SimpleScheduler` 收敛到 `SchedulerFactory`，当前只保留 simple 策略，未来支持 priority/dependency/resource-aware。
- **State**：为 agent runtime status 引入 transition policy，先测试现有状态行为，再把状态变化规则收口。
- **Mediator**：让 orchestrator/executor 更偏协调，不再散落 payload 补字段和状态细节。
- **Observer**：保护 `ExecutorEvent` broadcast 作为 SSE/EventLog/trace 的观察者通道，重构不得丢事件、重复事件或改变 payload。
- **Memento**：本轮不直接重构 session/resume 存储，但任何 executor/task event 改动都必须保持 EventLog 恢复兼容。

## 可选方案

### 方案 A：先做低风险 helper/factory 切片，再逐步迁移调用方

做法：

- 先在 `macaca-kernel` 内新增 `ExecutorEventFactory` / `TaskResult` helper，不改变 `ExecutorEvent` enum shape。
- 把 `macaca-web` 的 web-local executor event helper 迁移到 kernel helper。
- 再做 `SchedulerFactory`、`AgentStatusTransitionPolicy`、executor payload 下沉和 kernel facade/builder。
- 每个切片单独 OpenSpec、单独测试、单独 commit。

优点：

- 风险最低，符合现有 `macaca-kernel.md` 的五个小步计划。
- 优先解决已经发生的职责外泄：web 在构造 kernel executor event。
- 不触碰 `ExecutorEvent` 的序列化 shape 和 SSE/EventLog consumer contract。
- 为拆 `app_executor.rs` 大文件创造低风险前置边界。

缺点：

- 短期仍会存在 `ApplicationExecutor` 大文件和内部复杂 supervisor loop。
- helper/factory 增加后，需要后续消费方迁移才能真正减少重复。

结论：推荐。

### 方案 B：直接拆分 `ApplicationExecutor` 大文件

做法：

- 立即把 `app_executor.rs` 拆成 supervisor、worker_loop、event_publisher、registry 等多个文件。
- 顺手迁移 event 构造和 worker health。

优点：

- 能直接解决最大文件超过 500 行的问题。
- 文件职责会立刻清晰很多。

缺点：

- `ApplicationExecutor` 是 web 任务执行、SSE/EventLog、fork resume 的关键路径，一次性拆文件风险高。
- 没有先建立 event factory / publisher boundary，拆分时容易搬动业务逻辑并引入事件重复或丢失。
- 不符合“每次只做一小点”的当前节奏。

结论：不推荐作为第一步；可作为第四或第五切片之后的后续提案。

### 方案 C：优先重构 Kernel facade 和构造 API

做法：

- 新增 `KernelBuilder` / `KernelRuntime`，把 registry/scheduler/status/services 全部藏到 facade 后面。
- 上层统一迁移到新 builder。

优点：

- 上层 API 会更干净。
- 能为 `macaca-app`、`macaca-web`、`macaca-sdk` 后续迁移提供稳定入口。

缺点：

- `Kernel::new` 消费点多，GitNexus 预计风险高。
- 如果 scheduler/status/executor 内部边界未先收敛，facade 只是把不稳定复杂性包起来，后续还要反复改 builder。

结论：适合作为第五切片，不适合最先做。

## 推荐方案

采用方案 A：先做 kernel 内部稳定 primitive，再迁移消费方。

推荐五个切片：

1. **ExecutorEvent lifecycle helper**：新增 kernel-owned event/result factory，迁移 web-local helper 和 kernel 内重复构造。
2. **SchedulerFactory**：保留 `SimpleScheduler` 行为，新增 factory/builder 入口，为未来 scheduler strategy 做准备。
3. **Agent status transition policy**：先补 transition tests，再引入显式状态/活动 transition helper。
4. **Executor event publisher / payload boundary**：让 `ApplicationExecutor` 只描述执行结果，由 helper/publisher 负责生成和广播事件。
5. **Kernel facade / builder 收口**：新增 additive `KernelBuilder` 或 `KernelRuntimeParts`，上层可逐步迁移，旧 `Kernel::new` 保留兼容。

## 风险与控制

- 风险：`ExecutorEvent` 是 SSE/EventLog/历史恢复的关键 contract，任何字段变化都会导致 UI trace 丢失或重复。
  控制：不改 enum shape；先做 factory snapshot/unit tests，再迁移调用点。

- 风险：`ApplicationExecutor` 同时影响 worker 执行、fork resume、broadcast、queue result，改动过大会引入卡死或漏事件。
  控制：第四切片前只下沉 payload 构造，不移动 supervisor loop。

- 风险：`Kernel::new` 消费点很多，builder 迁移可能影响 web/cli/app/sdk 启动。
  控制：第五切片仅 additive 引入 builder，旧 `Kernel::new` 不删除；消费方迁移另开提案。

- 风险：scheduler 行为改变会影响 agent 分配。
  控制：`SchedulerFactory` 第一版只返回现有 `SimpleScheduler`；用同一 task/registry fixture 比较 selection 结果。

- 风险：状态 transition policy 过度设计。
  控制：只覆盖已有 `Created`、`Running`、activity `Idle/Thinking/Working/Error` 行为，不新增复杂 lifecycle。

## 成功标准

- 每个切片都可以独立编译、测试、回滚。
- `ExecutorEvent` / `TaskResult` payload 字段保持 1:1。
- `macaca-web` 不再维护自己的 executor lifecycle event helper。
- `Kernel::new` 行为保持兼容，新增 builder 不强制迁移所有消费方。
- `SimpleScheduler` selection 结果保持不变。
- `AgentStatusTracker` 现有状态更新行为保持不变，并新增 transition policy 测试。
- 不引入 application name、workflow name、driver name、FULLSTACK/NEWSROOM 等业务硬编码。
- 不引入新第三方依赖。
- 所有后续实施前必须跑 GitNexus impact；提交前必须跑 GitNexus detect_changes。

## OpenSpec 提案边界

下一步 OpenSpec 应覆盖：

- kernel SHALL provide canonical executor lifecycle event construction helpers.
- kernel SHOULD provide scheduler factory primitives while preserving current simple scheduler behavior.
- kernel SHOULD expose explicit agent status transition helpers without changing current status payloads.
- application executor SHOULD delegate event payload construction to kernel primitives.
- kernel facade SHOULD gain additive builder/factory entry points while keeping `Kernel::new` compatible.
