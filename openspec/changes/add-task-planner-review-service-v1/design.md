## Context

S4 的核心问题不是“task 能不能跑”，而是 task/ planner/ review/ resume 的语义归属仍然卡在 `macaca-web::loop_manager`。这让 Web 同时承担 HTTP/SSE adapter、executor 编排、review 推送、coordinator resume、fallback decomposition 以及 task lifecycle 观察者。

`macaca-task` 已经拥有 task domain primitives，`macaca-runtime-host` 已经拥有 service runtime 与 decorator 基础，`macaca-sdk` 已经有 `SystemTaskClient`。因此 S4 最合理的落点不是再造一个新框架，而是把 task 语义收束成 Task Service，并让 Web 成为 adapter。

## Goals / Non-Goals

### Goals

- 将 task/planner/review/resume 语义收敛到 Task Service
- 保持 task board session-scoped 读取和当前外部行为
- 提供显式 command/event/snapshot 边界
- 为后续 `ServiceRuntime`-backed task execution 留出替换点
- 让 Web 只保留 adapter、renderer、trace/event bridge 职责

### Non-Goals

- 不迁移 LLM / Memory / Context provider
- 不重写 `FrameworkRunner`
- 不删除旧 loop/compat API
- 不在本轮更改 task schema 或回归矩阵

## Decisions

### 1. 用 Facade + Mediator 组织 Task Service

Task Service 通过一个 facade 暴露 goal/task/review/snapshot command surface，而内部由 Mediator 协调 TaskBoard、TaskSpace、PlanLoop、WorkerLoop 以及 resume sink。

原因：

- 这是最直接的微内核边界收敛方式
- 可以把 `loop_manager` 的系统语义迁出 Web
- 便于 future runtime service 化和 remote adapter 化

### 2. 用 Command + Observer 描述 task 生命周期

Task operation 都应以 typed command 进入服务，生命周期变化以事件输出。

原因：

- 可 trace、可审计、可重放
- 适合 Web/SSE/EventLog 的适配模型
- 与 S3 的 `SystemFacade` 命令边界一致

### 3. 用 Strategy 注入 planner / reviewer / worker / resume policies

Task Service 不直接硬编码具体执行策略，而是通过可替换 strategy 适配当前 `FrameworkRunner` / `PlanLoop` / `WorkerLoop` 兼容行为。

原因：

- 能逐步替换 Web 中的具体执行实现
- 不会把 S4 变成新的 monolith
- S5 才是 LLM/Memory/Context 的服务化阶段

### 4. Web 以 Adapter 方式迁移

`macaca-web::loop_manager` 先保留兼容逻辑，但语义控制点逐步转成 Task Service command adapter、event sink 和 resume sink。

原因：

- 现有行为可保留
- 可分 slice 实施和回滚
- 避免一次性迁移导致 `/api/chat/v2`、trace、resume 退化

## Risks / Trade-offs

- **Risk: loop_manager 过渡期仍然很重。**
  - Mitigation: 先拆 command/event/sink，再拆 execution 逻辑。
- **Risk: review/resume 语义改变。**
  - Mitigation: 先写 regression spec，保留旧路径直到新路径验证通过。
- **Risk: S4 侵入 S5 的 provider 责任。**
  - Mitigation: 只定义 execution strategy seam，不迁移 LLM/Memory/Context provider 实现。

## Migration Strategy

1. 先建立 task service command/event/snapshot contract。
2. 再建立 task service runtime/provider skeleton。
3. 再把 Web loop manager 迁成 adapter。
4. 最后用 SDK client 或 ServiceRuntime 连接上层调用。

## Open Questions

- `TaskService` 是否需要直接对接 `ServiceRuntime`，还是先保留 local host runtime skeleton？
  - 推荐先保留 local host runtime skeleton，再逐步切换。
- review/resume 的最小事件集是否需要新增专门 trace event？
  - 推荐新增明确的 task lifecycle event，而不是让 Web SSE 事件继续承担全部语义。
