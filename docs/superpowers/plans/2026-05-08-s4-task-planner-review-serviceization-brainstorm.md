# S4 Task/Planner/Review 服务化 Brainstorm

## 背景

本次 S4 来自 `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`，目标是把 Task/Planner/Review 从 Web 事实协调层迁移为可替换、可审计、可通过 ServiceRuntime 托管的 Task Service。

必须遵守：

- `macaca/docs/agent-os-microkernel-boundaries.md`
- `macaca/docs/route-c-serviceization-allowlist.md`
- `macaca/docs/route-c-architecture-governance.md`

当前诊断：

- `macaca-task` 已拥有 `TaskBoard`、`TaskSpace`、`PlanLoop`、`WorkerLoop`、`GoalEvaluator`、`task_service_descriptor()` 等基础原语。
- `macaca-runtime-host` 已拥有 host-owned `ServiceRuntime`、provider factory、trace/policy decorators、snapshot/event sink。
- `macaca-sdk` S3 已提供 `SystemTaskClient`、`TaskBoardQueryCommand`、`SystemFacade` 上层边界。
- `macaca-web::loop_manager` 仍是事实上的 orchestration hub，集中持有 planner agent 构建、goal decomposition、review、worker delegate、SSE/EventLog、coordinator resume 和 fallback task creation。

## 设计模式候选

### Facade

`TaskServiceFacade` 或 `TaskServiceProvider` 对外提供 create goal、query board、claim task、submit review、subscribe events、snapshot 等统一入口。上层 Web/CLI/SDK 不再直接拼装 TaskBoard/PlanLoop/WorkerLoop。

适用原因：

- 隐藏 TaskSpace、TaskBoard、PlanLoop、WorkerLoop、review/resume 的内部组合。
- 给 S3 的 `SystemTaskClient` 一个未来 runtime-backed adapter。
- 避免 Web route 继续扩张为系统协调层。

风险：

- 如果 facade 做得太厚，会变成新的 monolith。
- 需要明确 facade 只协调 task 生命周期，不执行 LLM/Memory/Context provider 逻辑；S5 才迁这些 provider。

### Mediator

Task Service 作为 Mediator，协调 goal、planner、worker、review、resume、event emission。Web 只发送 command，agent execution 通过可替换 executor strategy 完成。

适用原因：

- 当前 `loop_manager` 已经是隐式 Mediator，但位置错误。
- Task Service 需要负责 task lifecycle 的系统语义。
- coordinator resume 必须与 review/goal completion 状态绑定并可 trace。

风险：

- Mediator 容易吸收过多跨领域能力，需要用 trait boundary 限制职责。
- agent 执行仍依赖 framework/web 当前上下文，第一阶段只能通过 adapter 注入，不能一步迁完。

### State

Goal、Todo、Review、Worker execution 都应显式状态化。现有 `TodoStatus`、`TodoGoalStatus` 和 lifecycle policy 可以继续保留，但 Task Service 需要把状态迁移变成可审计 command result。

适用原因：

- Route C 要求 task primitive 是 State/Mediator。
- review/resume 不能靠 Web 临时分支判断。
- 状态转换必须能写入 trace/event log。

风险：

- 修改状态机可能破坏已有 task board 和 resume 行为。
- 应 additive-first，先包装现有状态转换，不改变语义。

### Observer

Task Service 通过 `TaskServiceEvent` 观察并发布 goal ready、task claimed、review needed、goal completed、resume requested 等事件。Web/SSE/EventLog 是订阅者或 adapter，不再拥有事件语义。

适用原因：

- Route C 明确 task lifecycle 必须有 trace。
- 当前 SSE/EventLog 逻辑散在 Web，难以复用。
- ServiceRuntime 已有 event sink 模型，可复用思路。

风险：

- 若事件模型过早替换现有 SSE，容易破坏前端。
- 第一阶段应桥接到现有 Web broadcaster/EventLog，保持响应 shape。

### Strategy

Planner selection、review execution、worker assignment、fallback decomposition、resume policy 都应是 Strategy，而不是 Web 内部硬编码流程。

适用原因：

- Macaca 是 agent OS，不应写 application-specific workflow。
- 上层 application 多元，需要可替换 planner/reviewer/worker assignment。
- 后续可以替换为自定义 task service 或远程 task service。

风险：

- 过度抽象会拖慢实现。
- S4 应只抽必须的 execution strategy interfaces，默认实现继续复用当前行为。

### Command

所有入口变为 typed command：`CreateGoalCommand`、`QueryTaskBoardCommand`、`StartTaskRuntimeCommand`、`TaskReviewCommand`、`TaskServiceSnapshotCommand`、`ResumeCoordinatorCommand`。

适用原因：

- S3 已把 SDK facade 收敛到 command/client。
- ServiceRuntime call 本身也是 command/envelope。
- Command 可序列化、可 trace、可审计。

风险：

- 命令字段设计不完整会导致后续破坏性修改。
- 首版只建稳定核心字段：app_id、session_id、goal_id/task_id、agent_name、trace、limit/cursor、reason。

### Adapter / Bridge

Web 的 `loop_manager` 在迁移期变成 adapter：把 `AppState`、executor registry、SSE/EventLog、framework runner 适配为 Task Service 所需的 executor/event sink/resume sink。

适用原因：

- 不能一次性把 `macaca-framework`、LLM、Memory/Context 都迁入服务，S5 才处理。
- 保持 `/api/chat/v2`、task board、trace、resume 不退化。
- 逐步删除 allowlist 债务。

风险：

- Adapter 如果继续承载语义，就只是换名。
- OpenSpec/tasks 必须写清哪些逻辑迁出，哪些只是兼容 adapter。

### Specification

命令校验、scope 校验、session 必填、pagination limit、trace required、review outcome 合法性由 Specification 承担。

适用原因：

- 避免 Web/CLI 各自定义 task 语义。
- 支持依赖门禁和 regression matrix。
- 便于后续 remote task service 使用同样契约。

风险：

- 过严可能破坏现有 global/cross-session 兼容路径。
- 首版应保留 explicit global scope，但不能默认 app-wide 扫描。

## 方案 A：一次性把 `loop_manager` 整体搬到 `macaca-task`

做法：

- 把 `loop_manager.rs` 中 planner/review/worker/resume 逻辑整体移动到 `macaca-task`。
- Web 只调用 Task Service。

优点：

- 短期看起来最彻底。
- Web 快速瘦身。

缺点：

- `loop_manager` 依赖 `AppState`、`FrameworkRunner`、`ApplicationExecutor`、SSE、EventLog、RunTrace、session resume、framework session store。
- 直接移动会把 Web/framework/provider 依赖拖进 `macaca-task`，违反微内核边界。
- 高概率引入循环依赖和巨型文件。

结论：拒绝。

## 方案 B：先做 Task Service contract + Runtime shell，再用 Web adapter 执行现有逻辑

做法：

- 在 `macaca-task` 新增 service contract、command、event、snapshot、provider skeleton。
- `TaskServiceProvider` 托管 TaskSpace/TaskBoard/PlanLoop/WorkerLoop lifecycle，但执行 planner/review/worker 时调用注入的 Strategy traits。
- Web 提供 `WebTaskExecutionAdapter`、`WebTaskEventSink`、`WebCoordinatorResumeSink`，内部暂时复用 `FrameworkRunner`、SSE/EventLog、session resume。
- `loop_manager` 逐步缩成 command adapter 和 compatibility shim。

优点：

- 遵守 Route C：系统语义进入 Task Service，Web 只是 adapter。
- 保留现有行为，降低 `/api/chat/v2`、task board、trace、resume 回归风险。
- 为后续 S5 LLM/Memory/Context 服务化留下 seam。
- 可分 slice 实施和回滚。

缺点：

- 第一阶段仍有 Web adapter 依赖现有执行逻辑，allowlist 不会立刻归零。
- 需要非常清楚地区分 service semantics 与 Web compatibility adapter。

结论：推荐。

## 方案 C：直接让 SDK `SystemTaskClient` 调 ServiceRuntime，跳过 `macaca-task` provider

做法：

- 在 SDK task client 中构造 service call，直接调用 `ServiceRuntime` task descriptor。
- `macaca-task` 只保留 store/board。

优点：

- 上层接口统一得快。
- 与 S3 facade 衔接简单。

缺点：

- Task Service 语义没有落在 task crate，SDK 会变成 service semantics owner。
- 违反 `macaca-task` 目标归属是 task service。
- SDK 不应成为 provider factory 或 runtime coordinator。

结论：拒绝。

## 方案 D：只把 task board 查询服务化，planner/review 留到后面

做法：

- S4 只新增 task board service client/provider。
- PlanLoop/WorkerLoop/Review 暂不迁。

优点：

- 低风险。
- 文件变更小。

缺点：

- 不满足 S4 核心目标：PlanLoop/WorkerLoop/review/goal completion/coordinator resume 迁出 Web。
- 不能消除最关键的 `macaca-web -> macaca-task` 语义债务。

结论：可作为第一 slice，但不能作为完整 S4。

## 推荐方案

采用方案 B，并以方案 D 作为第一低风险切片。

核心路线：

1. 在 `macaca-task` 建立 Task Service contract、commands、events、snapshot、provider skeleton。
2. 把 TaskBoard query/create/claim/review/snapshot 先包装为 service operations。
3. 把 PlanLoop/WorkerLoop lifecycle 放入 Task Service runtime controller，但 planner/reviewer/worker execution 通过 Strategy traits 注入。
4. Web `loop_manager` 分阶段变为 adapter：先调用 Task Service command，再承载 execution adapter，最后只保留 compatibility shim。
5. SDK `SystemTaskClient` 增加 runtime-backed adapter，使 Web/CLI/Gateway 上层走 `SystemFacade`。

## 风险清单

- `loop_manager` 高耦合风险：必须先拆 command/event/execution/resume sink，不直接大搬家。
- `/api/chat/v2` 回归风险：不得改变 create_goal、planner decomposition、worker delegate、review、resume 的外部时序。
- Task board session scope 风险：禁止重新引入 application-wide 全量读取作为默认路径。
- Trace 缺失风险：所有 task lifecycle command 必须携带或生成 trace context，缺 trace 的 service call 应拒绝或明确记录 compatibility exception。
- Policy 缺失风险：ServiceRuntime 路径必须经过 trace/policy decorator；本阶段可使用 allow-all policy，但必须显式。
- S5 边界风险：不要在 S4 中迁移 LLM/Memory/Context provider；只定义 execution Strategy seam。
- 文件膨胀风险：新增 `commands.rs`、`events.rs`、`provider.rs`、`runtime.rs`、`snapshot.rs` 等小文件，单文件不得超过 500 行。
- Allowlist 风险：S4 目标是减少 `macaca-web -> macaca-task` 语义债务，但可能不会立刻移除 Cargo 依赖；文档必须说明过期条件。

## 验证方向

- `cargo test -p macaca-task`
- `cargo test -p macaca-web loop_manager`
- `cargo test -p macaca-sdk task_client`
- `cargo test -p macaca-runtime-host service_runtime`
- `cargo test -p macaca-integration-tests route_c_dependency_boundaries`
- `cargo test -p macaca-integration-tests --test route_c_baseline`
- `cargo check --workspace`
- `npx gitnexus detect-changes -r agent`
