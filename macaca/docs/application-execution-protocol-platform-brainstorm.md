# Macaca 应用执行协议平台 Brainstorm

## 背景

当前 `CODEX-WASM-WORKBENCH` 暴露出的核心问题不是单个应用 UI 形态问题，而是应用执行所有权问题。所有应用都应该只通过前端完成交互适配：发起任务、订阅 session events、渲染 replay/current state、发送 cancel/approve/resume 等控制命令。真实执行循环、事件持久化、状态投影、审计追踪和控制命令处理必须由后端执行面或应用后端承担，不能依赖浏览器是否打开、是否刷新、是否断线。

这次 brainstorm 的设计中心是：Macaca OS 本质上应该是一个协议平台。Macaca OS 不应该替上层应用写业务执行逻辑，而应该定义稳定、通用、可审计、可替换的协议、基础服务和能力边界。上层 application 只要遵循协议，并通过 Macaca 提供的基础设施模块和服务，就可以实现复杂的工业级产品。

## 必须遵守的架构约束

本 brainstorm 必须服从以下治理文档：

- `macaca/docs/macaca-os-architecture-governance.md`
- `macaca/docs/macaca-os-microkernel-boundaries.md`
- `macaca/docs/macaca-os-serviceization-allowlist.md`

关键约束如下：

- Web、CLI、frontend、gateway 都是 shell/adapter，只能解析输入、渲染状态、展示 approval/trace/diagnostics、订阅事件。
- Shell 不能成为 task、session、application execution、provider lifecycle、approval、replay 的语义所有者。
- Microkernel 只拥有身份、policy facade、service registry、capability registry、IPC/service-call facade、trace/audit bus、session/task primitives 等系统不变量。
- 可替换、可扩展、可外包、因租户或应用而变化的能力必须进入 service boundary。
- Application framework 拥有 manifest、ABI、lifecycle、app-scoped permissions、session envelope、UI surface metadata，但不能拥有具体业务域规则。
- Application 可以编排服务，但必须通过声明的 capability 和 service boundary。
- OS 层禁止硬编码 app name、provider name、model name、driver name、workflow name 或任何业务逻辑。
- 所有跨边界调用必须有 trace、policy、resource、entitlement、audit 和 structured error。
- 缺失 provider 或 optional module 必须返回 unavailable/disabled/denied，不能 crash、hang、fake success 或 silent fallback。

## 核心原则

所有应用都只能在前端侧做四类事情：

1. 发起任务。
2. 订阅 session events。
3. 渲染 replay/current state。
4. 发送 cancel、approve、reject、pause、resume、retry 等控制命令。

真实执行必须满足：

- 不依赖浏览器生命周期。
- 不依赖某个前端页面是否打开。
- 不把前端本地状态作为权威状态。
- 不让 browser bridge 承担真实执行持久化职责。
- 不绕开 Macaca EventLog、trace、audit、policy、capability、approval。
- 不向 Macaca OS 写入应用专用执行逻辑。

## 推荐方向

采用 `Protocol First + Provider Strategy` 架构，把 Macaca OS 定位为应用执行协议平台。

Macaca OS 提供统一的 `ApplicationExecutionSession` 协议面和服务边界，所有执行形态都必须遵循同一组协议：

- Application Execution Protocol
- Session Event Protocol
- Execution Control Protocol
- Executor Provider Protocol
- Gateway Ingress Protocol
- Replay/Checkpoint Protocol
- Capability/Policy/Audit Protocol

不同执行形态通过 provider strategy 接入：

```text
ApplicationExecutionProvider
├── macaca_hosted
├── external_app_backend
└── remote_agent
```

Macaca OS 不关心应用是不是 Codex，不关心应用业务是什么，也不关心 provider 在哪里执行。Macaca OS 只校验协议是否被遵守，并提供稳定的 session、event、trace、audit、policy、capability、approval、workspace、replay 和 control 基础设施。

## Provider 形态

### macaca_hosted

`macaca_hosted` 表示 Macaca 后端托管执行。适合 WASM application、YAML application、GenUI/headless application、平台托管执行器和不希望自建后端的应用。

典型流程：

```text
UI/API -> Macaca start_execution
Macaca -> create session/run/workspace envelope
Macaca -> policy/capability/resource/entitlement checks
Macaca -> choose macaca_hosted provider
Runtime Host -> load app execution component
Runtime Host -> execute application loop
Runtime Host -> append events to EventLog
UI/API -> subscribe/replay/control only
```

优点：

- 与 Macaca OS 的审计、replay、session recovery 最一致。
- 事件持久化、控制命令、provider health 和 failure semantics 最容易统一。
- 最适合作为 CODEX-WASM-WORKBENCH 的首个真实验证路径。

风险：

- Runtime Host 需要支持长期运行任务、checkpoint、resume、cancel、approval blocking、tool-call loop 和 workspace binding。
- 执行器不能写成 Codex 专用逻辑，必须通过 application ABI 或通用 execution component 接入。

### external_app_backend

`external_app_backend` 表示应用拥有自己的后端执行系统，但必须通过 Macaca gateway 写入事件、接收控制命令，并遵守 Macaca 的 session/event/control 协议。

典型流程：

```text
UI/API -> Macaca start_execution
Macaca -> create session/run/workspace envelope
Macaca -> policy/capability/resource/entitlement checks
Macaca -> call external backend /start
External Backend -> execute application-owned loop
External Backend -> call Macaca gateway append_event
Macaca -> persist EventLog + publish realtime
Macaca -> forward cancel/approve/resume to external backend
UI/API -> subscribe/replay/control only
```

优点：

- 给复杂应用最大灵活性。
- 已有商业系统可以通过协议接入 Macaca OS，而不是被迫迁入 Macaca runtime。
- Macaca OS 保持基础设施和协议平台定位。

风险：

- 如果协议不严，容易变成每个应用一套执行系统。
- 必须强制 external backend 不绕过 Macaca EventLog。
- 需要 signed callback、scoped token、idempotency、heartbeat lease、schema validation、structured failure、audit-safe payload 等约束。

### remote_agent

`remote_agent` 表示远程 agent/runtime 执行。它更像受 Macaca 调度和治理的远程执行节点，适合分布式 agent、云端 worker、跨机器 runtime、隔离计算环境和未来 agent marketplace。

典型流程：

```text
UI/API -> Macaca start_execution
Macaca -> create session/run/workspace envelope
Macaca -> policy/capability/resource/entitlement checks
Macaca -> broker dispatch to remote agent
Remote Agent -> execute under lease
Remote Agent -> append events/checkpoints
Macaca -> monitor heartbeat/lease
Macaca -> send cancel/approve/resume over control channel
UI/API -> subscribe/replay/control only
```

优点：

- 支持真正分布式、可替换、可调度的 agent runtime。
- 与未来远程算力、隔离执行、agent marketplace、跨租户执行有自然扩展关系。

风险：

- 需要更强的 lease、heartbeat、capability delegation、resource accounting、resume 和 failure recovery。
- 远程 agent 不能直接获得 host 权限，必须通过 capability 和 policy gate。

## 协议面设计

### 1. Execution Start Protocol

负责启动应用执行会话。输入应包含：

```text
app_id
session_id
run_id
task_input
workspace_ref
requested_capabilities
provider_preference
trace_context
policy_context
idempotency_key
```

输出应包含：

```text
accepted | denied | unavailable | failed
session_id
run_id
provider_id
event_cursor
control_endpoint
structured_error
```

### 2. Session Event Protocol

所有执行事实必须进入 Macaca EventLog。事件类型应覆盖：

```text
session.started
execution.accepted
provider.assigned
provider.heartbeat
llm.requested
llm.completed
tool.call.requested
tool.call.dispatched
tool.call.completed
approval.requested
approval.resolved
checkpoint.created
execution.completed
execution.failed
execution.cancelled
```

每个事件都应包含：

```text
session_id
run_id
seq
timestamp
event_type
trace_id
actor
provider_id
visibility
sanitized_payload
causality
```

EventLog 是 durable source of truth，realtime 只是 EventLog 的观察投影。

### 3. Execution Control Protocol

控制命令必须是 typed command，而不是 UI 本地状态修改：

```text
cancel
approve
reject
pause
resume
retry
inject_input
```

每个控制命令都必须：

- 进入 audit。
- 关联 session、run、trace。
- 经过 policy gate。
- 支持 idempotency。
- 返回 structured result。
- 可以转发给 `macaca_hosted`、`external_app_backend` 或 `remote_agent`。

### 4. Provider Protocol

三个 provider 都必须实现共同 contract：

```text
start
control
health
snapshot
resume
shutdown
capabilities
```

provider 差异通过 adapter 处理，不能让 OS 业务层根据 app name、provider name 或 model name 写硬编码分支。

### 5. Replay/Current State Protocol

`current state` 不能来自前端本地状态。它应该由 EventLog 投影而来：

```text
events -> reducer/projector -> current_state
events -> replay stream
events -> UI timeline
events -> diagnostics
```

因此浏览器关闭、刷新、重连不会影响任务执行，也不会丢失 session history。

### 6. Gateway Ingress Protocol

`external_app_backend` 和 `remote_agent` 写入 Macaca 时必须经过 gateway ingress：

```text
append_event
report_heartbeat
report_snapshot
request_approval
report_completion
report_failure
```

gateway 必须执行：

- callback 身份校验。
- session/run 绑定校验。
- event schema 校验。
- payload sanitization。
- seq/idempotency 校验。
- trace/audit 注入。
- policy/resource/entitlement 检查。

## 设计模式

该方向明确使用以下设计模式：

- Strategy：三类 execution provider 可替换。
- Adapter：hosted runtime、external backend、remote agent 接入同一协议。
- Command：start/cancel/approve/resume/retry 等跨边界操作都是 typed command。
- Observer：EventLog、realtime、session subscription。
- State：session、run、provider、approval、checkpoint 生命周期显式建模。
- Memento：checkpoint、snapshot、replay、audit record。
- Decorator：policy、trace、audit、resource、entitlement、metering 包裹 provider 调用。
- Facade：SDK/SystemFacade 暴露稳定开发者接口。
- Specification：manifest capability、provider admission、event schema、version compatibility 可执行校验。
- Null Object：provider 缺失或不可用时返回 structured unavailable。

## 对应用开发者的意义

Macaca OS 应该让开发者通过协议获得能力，而不是要求开发者理解 OS 内部实现。

Macaca 托管应用：

```text
manifest declares capabilities
app exports execution entry
Macaca runs execution component
app emits protocol events
```

自带后端应用：

```text
manifest declares external backend
backend implements start/control/health/callback
backend writes events through Macaca gateway
UI subscribes Macaca session
```

远程 agent 应用：

```text
manifest declares remote agent provider
agent registers capabilities
Macaca assigns execution lease
agent writes events/checkpoints
Macaca sends control commands
```

这使 Macaca OS 成为协议平台和能力底座，而不是某一种应用形态的后端框架。

## 必须禁止的反模式

- 前端执行真实 long-running task。
- 前端本地 `state.events` 成为权威事件源。
- browser bridge 负责真实执行持久化。
- external backend 直接向前端推权威状态而绕过 Macaca EventLog。
- remote agent 直接获得 host 权限而不经过 capability/policy。
- OS 层出现 CODEX-WASM-WORKBENCH 或任何具体应用专用逻辑。
- OS 层根据 app name、model name、provider name、driver name、workflow name 做硬编码路由。
- provider 缺失时 fake success 或 silent fallback。
- trace、audit、snapshot、diagnostics 泄露 raw secrets、raw prompts、raw provider payloads、raw WASM/package bytes 或无界输出。

## 验收标准

该方向落地后，至少需要验证：

1. UI 发起 Workbench 编程任务后关闭浏览器，后端继续执行。
2. 重新打开 UI 后，可以通过 session id replay 完整 session events。
3. `macaca_hosted` provider 能按同一协议执行任务、写 EventLog、响应 control。
4. `external_app_backend` provider 能通过 Macaca gateway 写事件、上报 heartbeat、接收 cancel/approve/resume。
5. `remote_agent` provider 能注册/心跳/执行/写事件/响应控制命令。
6. `cancel`、`approve`、`reject`、`resume` 都经过统一 control protocol、policy 和 audit。
7. EventLog 是 durable source of truth；realtime 只是订阅投影。
8. provider 不可用时返回 structured unavailable，不 crash、不 hang、不 fake success。
9. Web UI 只发任务、订阅事件、渲染状态、发送控制命令。
10. Macaca OS 服务中没有任何 application 专有逻辑或硬编码业务分支。

## 建议的 OpenSpec Change

建议后续 OpenSpec change 命名为：

```text
add-application-execution-protocol-platform
```

该 change 的目标不是“给 Workbench 加一个后端”，而是建立 Macaca OS 的通用应用执行协议平台：

```text
Macaca OS becomes the protocol platform for application execution.
Providers are interchangeable protocol participants.
EventLog is the durable source of truth.
Frontend is only an interaction adapter.
```

后续 implementation plan 应同时覆盖三个 provider，而不是只实现 `macaca_hosted`：

- `macaca_hosted`
- `external_app_backend`
- `remote_agent`
