# Macaca OS 目标架构设计：协议化微内核 + 单一调用路径

日期：2026-06-07
状态：目标态设计（Target State）。本文定义"应该长成什么样"，配套重构方案见 `2026-06-07-macaca-os-debt-elimination-refactor-plan.md`。
依据：`macaca-os-architecture-governance.md`、`macaca-os-microkernel-boundaries.md`、`macaca-os-serviceization-allowlist.md`、`docs/design_patterns.md`。

## 1. 北极星（North Star）

```
Microkernel（系统不变量）
  + Single Protocol Call Path（唯一协议调用路径）
  + Service Runtime（可替换能力的统一承载）
  + Application ABI（YAML/WASM/GenUI/headless 统一应用入口）
  + Plugin / Optional Module Ecosystem（按需装载，缺席可结构化降级）
```

一句话：**任何上层应用、任何能力调用，最终都收敛到同一条 `service.call` 协议路径；内核只守护不变量，其余皆为可替换服务/模块。**

## 2. 唯一调用路径（The One Path）

所有服务能力调用（LLM / tool / driver / skill / MCP / task / agent execution / memory / context / payment / web3 / evm / gateway …）**必须且只能**经过：

```
[应用/Shell/插件]
  → SystemFacade / focused SDK client        (facade 层，唯一对外入口)
  → ServiceRouter.route(ServiceRouteRequest) (contract 校验 → policy → 路由 → retry/timeout → audit)
  → ServiceRuntime.call()                    (admission decorator 链)
      Trace → Policy → Resource → Entitlement → Metering → Audit
  → ServiceBus（IPC，transport 抽象）
  → SystemServiceBusHandler                  (envelope → ServiceCommand)
  → ServiceCallExecutor                      (TraceRequired 中间件 + trace/audit 发射)
  → SystemService.call(ServiceCommand) -> ServiceCallResult
  → 具体 Provider（built-in / plugin / remote / mock / unavailable）
```

设计要点：
1. **协议化**：跨边界一律 `ServiceCommand`/`ServiceCallResult`/`ServiceError`（已存在于 `macaca-proto`），不允许直接函数调用旁路。
2. **强制 trace**：`TraceRequiredMiddleware` + `TraceRequiredRuntimeDecorator`，no trace → 直接拒绝。
3. **side-effect 前 policy**：Policy decorator 在任何副作用前裁决。
4. **统一 audit/replay**：所有调用产出 service-call evidence，可按 trace_id / session_id replay。
5. **provider 可替换**：consumer 不感知实现来源（built-in/plugin/remote/mock/unavailable）。

> 不存在"第二条路"。kernel `Agent::run(llm,tools)`、web `agent.reply()`、`framework_toolkit` 直读 runtime、kernel executor 直驱 task —— 全部并入此路径或删除。

## 3. 分层与依赖方向（修正后）

依赖严格向下，跨层只能经 facade：

```
┌─────────────────────────────────────────────┐
│ Shells: macaca-web / macaca-cli / gateway     │  只依赖 macaca-sdk
├─────────────────────────────────────────────┤
│ Applications: macaca-app（YAML/WASM/GenUI ABI）│
├─────────────────────────────────────────────┤
│ Facade/SDK: macaca-sdk（SystemFacade + clients）│  不构造 provider
├─────────────────────────────────────────────┤
│ Service Runtime / Host: macaca-runtime-host    │  唯一 composition root
│   + macaca-framework / macaca-runtime          │
├─────────────────────────────────────────────┤
│ System Services: llm/memory/context/driver/    │
│   skill/gateway/tools/task/scheduler/payment/  │
│   web3/evm/...（均为可替换 provider）           │
├─────────────────────────────────────────────┤
│ Microkernel: macaca-kernel（仅不变量）          │
├─────────────────────────────────────────────┤
│ Foundation: macaca-proto / macaca-ipc /        │
│   macaca-persist（纯契约/传输/持久化）           │
└─────────────────────────────────────────────┘
```

目标依赖规则（boundary gate 终态，allowlist 清零）：
- `macaca-kernel` 仅依赖 `macaca-proto`、`macaca-ipc`。**不再**依赖 agent/sdk/driver/gateway/skill/task/tools。
- `macaca-persist` **不再**依赖 `macaca-context`（持久化契约不依赖能力服务）。
- `macaca-web` / `macaca-cli` 仅依赖 `macaca-sdk`（+ proto DTO）。**不再**直依 kernel/runtime-host/具体 service。
- 具体 service crate 之间不互相直依实现（如 `macaca-tools → macaca-task` 应改为经契约/service client，按需评估）。
- composition root 唯一：`macaca-runtime-host` 负责装配所有 provider；其它 crate 不构造 provider。

## 4. 微内核纯净化：内核只保留这些

依 `microkernel-boundaries.md §What The Kernel May Own`，`macaca-kernel` 目标内容：

| 保留（内核不变量） | 对应现有文件 |
|--------------------|--------------|
| identity（app/agent/session/task/service/capability/package/tenant） | `registry.rs`、`service_registry.rs`、`capability_registry.rs` |
| service registry / capability registry | `service_registry.rs`、`capability_registry.rs` |
| IPC / service-call facade（typed 路由 + 强制 trace） | `service_call.rs`、`service_bus_bridge.rs`、`facade.rs` |
| policy facade（抽象裁决） | `policy.rs` |
| trace / audit bus（append-only evidence） | `facade.rs`(TraceEventBus)、`audit.rs`、`trace_service_adapter.rs` |
| scheduler primitive（公平/唤醒语义） | `scheduler.rs`(去 deprecated)、`scheduler_factory.rs` |
| resource manager facade | `resource.rs` |
| session primitive（lifecycle/pause/resume/checkpoint id/cancel） | （抽象保留） |
| task primitive（goal/task/review 状态契约，不含 planner 实现） | `status.rs`、`status_transition.rs` |
| package runtime guard（签名/版本/权限/entitlement 准入） | `plugin_registry.rs`、`service_lifecycle.rs` |
| agent execution **port**（typed 抽象，不接 provider） | `kernel.rs` 的 `AgentExecutionPort`（强化） |

**必须移出内核**：

| 移出项 | 去向 | 理由 |
|--------|------|------|
| `web3.rs / web3_event.rs` | optional module（`macaca-web3` 服务或插件） | 链能力非内核 |
| `evm.rs / evm_adapter.rs / evm_event.rs` | optional module（EVM 服务/插件，见 `optional-evm-substrate-frontier-adapter-boundary.md`） | EVM 非内核 |
| `a2a.rs / a2a_event.rs` | payment/A2A service（runtime-host provider） | 支付非内核 |
| `payment_policy.rs` | payment service policy strategy | 支付策略非内核 |
| `provider_compat.rs`（`LegacyLlmProvider/LegacyToolCatalog/KernelProviderCompat`） | **删除** | provider 不入内核 |
| `executor/`（`ApplicationExecutor/ForkManager/AgentRunner/TaskRouter/WorkerSupervisor/CallbackDispatcher`） | task/execution service（runtime-host/service 层） | worker-loop/编排非内核 |
| `KernelServiceClientCompat`、`scheduler` deprecated、`persistence` deprecated payment store | **删除** | 兼容债 |

> 内核保留 `AgentExecutionPort` 是关键设计：内核只持有"执行一个 agent"的 typed 抽象端口，真正的执行（模型/工具/loop）由 runtime-host 的 Agent Execution Service 实现。端口契约收紧为**只接 service client / typed handle，禁止接 provider trait**。

## 5. 非内核能力服务化清单（统一进入 service.call）

每个能力必须满足 `serviceization-allowlist.md §Service Admission Conditions`：稳定 service_id + descriptor + command/result/error 类型 + lifecycle（register/start/health/pause/resume/shutdown）+ 每次调用带 session/task/app/tenant/trace + side-effect 前 policy/resource/budget/entitlement + 脱敏 trace/audit + 结构化 unavailable/unsupported/denied/failure + provider 可替换（built-in/plugin/remote/mock/unavailable）。

| 能力 | 目标 service | 当前位置 → 目标 |
|------|--------------|----------------|
| LLM | `service.llm` | 已 service；provider/model 映射移入 config/descriptor（去除代码内 name 分支） |
| Memory/Context | `service.memory`/`service.context` | 已 service；web 直读 runtime 删除 |
| Tool/Driver/Skill/MCP | `service.tools`/`service.driver`/`service.skill`/`service.mcp` | 工具收集统一经 service snapshot，删除 `framework_toolkit` 直读 |
| Task（goal/task/review/recover/retry） | `service.task` | 已 service；kernel executor 与 web loop 下沉至此 |
| Agent execution | `service.agent_execution` | provider 实现移到 runtime-host，web 仅做 SSE/DTO 适配 |
| Execution control（pause/resume/checkpoint/replay） | `service.execution_control` | 已 service；删除 web `legacy_*` policy 分支 |
| Payment / A2A | `service.payment` | 从 kernel 移出至 runtime-host provider |
| Web3 / EVM / wallet / chain | optional module | 从 kernel 移出；缺席返回结构化 unavailable |
| Gateway 入口 | `service.gateway` | kernel 不再直依 gateway |
| Store/entitlement/license/metering | service | runtime-host provider |

## 6. 应用框架统一入口（YAML = WASM = GenUI = headless）

所有应用类型经**同一** Application ABI 进入，最终都落到 §2 单路径：

```
应用 manifest（YAML/WASM/GenUI）
  → Application Service（macaca-app + runtime-host application_service_provider）
  → MacacaHostedApplicationExecutionProvider（generic 执行信封）
  → ApplicationHostRuntime.dispatch(ApplicationHostCommand)
  → WasmHostImportBridge / 等价 ABI 适配
  → ServiceRouter.route() → ServiceRuntime → ...（§2 单路径）
```

要点：
- YAML workflow 是**应用适配器**，不是内核特性；WASM 的灵活性是 runtime 特性，不是绕过 policy 的许可（governance §Application Rules）。
- `application_execution_hosted.rs` 已把 `app:start` 表示为 generic WASM invoke，并声称"lets any WASM/YAML/GenUI application reuse the same replay and audit correlation path"——目标态下此承诺对所有应用类型**真实成立**（不再有 legacy 旁路）。
- 删除 `graph_owner / authoritative / legacy_unmarked` 区分：当只剩单路径，所有 task 天然 authoritative，终态判定回归简单确定。

## 7. 设计模式映射（保留并强化，禁止过度设计）

| 模式 | 用途 | 落点 |
|------|------|------|
| Facade | 唯一对外入口 | `SystemFacade` + focused SDK clients |
| Command | 跨边界 typed 操作 | `ServiceCommand/ServiceCallResult` |
| Chain of Responsibility | 调用前校验链 | `ServiceCallExecutor` middleware |
| Decorator | 边界横切（trace/policy/resource/entitlement/metering/audit） | `ServiceRuntime` decorator 链 |
| Adapter / Bridge | provider/transport/ABI/插件适配 | `service_bus_bridge`、`host_import_bridge`、ABI adapter |
| Strategy | provider/policy/routing/assignment/payment/chain 可替换 | execution adapter、policy engine、scheduler factory |
| State | application/task/service/payment/package 生命周期 | 状态契约模块 |
| Observer | trace/audit/event/task/service 事件可订阅 | `TraceEventBus`、`ServiceRuntimeEventSink` |
| Memento | snapshot/checkpoint/audit 可重放 | EventLog replay、checkpoint_ref |
| Specification | 依赖门/准入/版本约束可执行 | dependency gate、package admission |
| Abstract Factory | provider 工厂/模块装配仅在 composition root | `ServiceProviderFactory`（runtime-host） |

原则：每个抽象必须服务于"至少一个现有 consumer + 一个明确未来扩展点"，否则不引入。

## 8. 可trace可审计 / 日志 / 注释规范（终态要求）

- **trace**：每次 service.call 必带 `TraceContext`；缺失即拒绝。
- **audit**：每次调用产出脱敏 service-call evidence，支持 by trace_id / by session_id replay；**无旁路盲区**。
- **日志**：关键节点（准入/路由/policy 裁决/调用起止/失败/拒绝/降级/lifecycle 变迁）均有 `tracing`，字段为 service_id/command/trace_id/reason_code 等 provider-neutral 维度，**不以 provider/model/app name 为主键**。
- **脱敏**：禁止 raw secret/prompt/manifest/WASM bytes/package bytes/private key/credential/raw signature/raw provider payload/unbounded output 进入日志/快照/trace（沿用 `sanitize_json`/`is_safe_metadata_key` 并全局化）。
- **英文注释**：所有代码详尽英文注释，解释功能 + 运行原理 + 设计权衡；配合 §9 文件拆分，使局部分支可被注释覆盖。

## 9. 架构卫生（文件/职责）

- 单文件 ≤500 行（AGENTS.md）。巨型文件按 ownership 拆分：
  - `loop_manager.rs` → execution-control/task service + 薄 SSE channel adapter。
  - `framework_runner.rs` → runtime/framework 层 agent 构造 service。
  - `chat_orchestrator.rs` → route adapter + application service 调用。
  - `lib.rs`(web) → 小型 shell composition bundle。
- 巨型文件视为"所有权不清"的信号，拆分即归位，而非格式化。

## 10. 可执行边界 Gate（终态）

- `route_c_dependency_boundaries` 的 allowlist **清零**（当前 10 条 → 0）。
- 新增/强化 gate（governance §Executable Gates）：
  1. kernel 不依赖任何具体 provider crate。
  2. SDK 不依赖 composition root / shell。
  3. shell 不成为系统语义所有者（禁止生产代码引用 deprecated direct fields / 直驱 task/loop）。
  4. service provider 不 import shell。
  5. optional module 不成为 base OS 必需依赖。
  6. 每个 workspace crate 归属明确层。
  7. "no direct provider call" 审计：每个已服务化能力，生产代码只能经 service client；旁路引用 → 测试失败。
  8. 生产代码无硬编码 agent/app/provider/model/driver/gateway/chain/payment 名（fixtures/tests 除外）。

## 11. 验收门（每次 OS 层变更必须证明）

沿用 governance §Acceptance Gates，并补充本设计专项：
- YAML / WASM / GenUI 应用仍可运行，且**经同一 service.call 路径**（可由 audit replay 证明只剩一条路径）。
- `/api/chat/v2` 会话创建与恢复不回归；任务板按 session 隔离。
- trace/audit refresh 后仍可重放；driver/skill/mcp/llm/memory/context/app/store/payment/web3/evm 失败均为结构化 unavailable/denied，而非崩溃。
- optional module 缺席时 base OS 仍可启动/执行/恢复/查询审计。
- 边界 gate 通过且 allowlist 为 0；无新增硬编码名称；日志/快照已脱敏。

## 12. 与现状的差异收敛（Definition of Done）

目标达成的判据（与审计报告一一对应）：
1. §4.2 的多路径全部消除，只剩 §2 单路径。
2. §4.3 的 `graph_owner/authoritative/suppress_executor_lifecycle/legacy_*` 协调补丁全部删除。
3. §5 的内核越界依赖与非内核能力全部移出；allowlist 清零。
4. §6 web 还原为 thin shell；巨型文件拆分达标。
5. §7 债务热点（compat.rs/provider_compat.rs/kernel_builder compat/mcp_runtime 等）删除或服务化。
6. OpenSpec baseline 反映上述终态。
