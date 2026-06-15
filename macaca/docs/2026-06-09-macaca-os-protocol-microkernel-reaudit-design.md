# Macaca OS 协议微内核重审计与整改设计基线

日期：2026-06-09
审计快照：`main` / `d6ae898`
范围：当前 `macaca/` Rust workspace，重点覆盖 AgentScope 2.0 改写后对 `refactor-unified-call-path-microkernel` 目标的影响。
性质：只读审计 + 目标设计基线；本文不代表实现已完成。

## 1. 审计结论

结论必须直接回答问题：**当前实现没有 100% 达到“唯一协议调用路径 + 纯净微内核 + 非内核能力全部服务化/模块化 + 历史债务清零”的目标。**

当前状态比 `2026-06-07-macaca-os-unified-call-path-audit.md` 明显前进：主执行路径、YAML delegate、WASM host import、AgentScope 2.0 framework 边界、shell 依赖和 kernel 生产依赖 gate 都已经更接近目标态。但“可执行 gate 通过”不等于“历史债务清零”。当前仍存在生产代码兼容锚点、shell 构建钩子、runtime-host deprecated facade、kernel 内可替换能力实现，以及一个文件大小宪法 gate 失败。

| 目标 | 当前判断 | 说明 |
| --- | --- | --- |
| 唯一协议调用路径 | **接近达成，但未 100%** | `/api/chat/v2`、YAML delegate、WASM host import 的 contract tests 已证明主链路收敛到 `service.agent_execution` / `ServiceRuntime`。但 shell 仍持有 framework construction adapter、本地 loop/waker/channel 和兼容 route。 |
| 纯净微内核 | **未 100%** | `macaca-kernel` 生产依赖 gate 通过，但内核仍有 `AlertManager` 直接构造 HTTP webhook client，并保留 `AgentOrchestrator` 这类 task/agent 编排语义模块。 |
| 非内核能力服务化/模块化 | **部分达成** | LLM、agent execution、execution control、application execution、WASM host import、optional Web3/EVM 等主能力已服务化；MCP、entitlement/store、shell memory/tool/runtime anchors 仍有 deprecated/compat facade。 |
| 历史债务清零 | **未达成** | 生产代码仍存在 `deprecated`、`legacy`、`compat`、`Route C migration`、SDK `shell_provider_bridge`、legacy `/api/chat` re-export、runtime-host deprecated facades。 |
| 开发宪法遵守 | **部分遵守** | 依赖 purity gate 多数通过；但 `os_layer_file_size_gate` 当前失败，`crates/runtime/macaca-framework/src/tool.rs` 为 504 行，超过 500 行上限。 |

## 2. 审计依据

稳定治理宪法：

- `docs/macaca-os-architecture-governance.md`
- `docs/macaca-os-microkernel-boundaries.md`
- `docs/macaca-os-serviceization-allowlist.md`

目标设计与旧审计：

- `docs/2026-06-07-macaca-os-unified-call-path-audit.md`
- `docs/2026-06-07-macaca-os-protocol-microkernel-target-design.md`

关键设计模式依据：

- `docs/design_patterns.md`

本次审计按以下模式判断架构归属：

- **Facade**：shell/SDK 对外只应进入 `SystemFacade` 或 focused SDK clients。
- **Command**：跨边界必须是 typed command/result。
- **Adapter/Bridge**：WASM host import、framework construction、provider/module 接入只能作为边界适配器存在。
- **Decorator**：trace、policy、resource、entitlement、metering、audit 必须在 service boundary 执行。
- **Observer/Memento**：trace、audit、event log、checkpoint 必须可订阅和可 replay。
- **Specification**：dependency/file-size/static path gate 必须成为可执行约束。

## 3. 已达标或明显改善的部分

### 3.1 主协议路径已经存在并且是正确方向

当前 canonical service path 仍然成立：

```text
应用 / Shell / WASM host import / SDK client
  -> ServiceRouter.route(ServiceRouteRequest)
  -> ServiceRuntime.call()
  -> ServiceBus
  -> SystemServiceBusHandler
  -> ServiceCallExecutor
  -> SystemService.call(ServiceCommand)
  -> concrete provider
```

硬证据：

- `crates/runtime/macaca-runtime-host/src/service_router.rs:75-191`：`ServiceRouter::route` 执行 contract resolution、policy decision、provider strategy、retry/timeout 和 audit event。
- `crates/runtime/macaca-runtime-host/src/service_runtime.rs:51-75`：`ServiceRuntime` 安装 Trace、Policy、Resource、Entitlement、Metering、Audit decorator chain。
- `crates/runtime/macaca-runtime-host/src/service_runtime.rs:178-214`：`ServiceRuntime::call` 只通过 typed `ServiceCommand` 和 service bus dispatch。
- `crates/kernel/macaca-kernel/src/service_call.rs:30-47`：`TraceRequiredMiddleware` 保证 no trace, no call。
- `crates/kernel/macaca-kernel/src/service_call.rs:77-154`：`ServiceCallExecutor` 统一记录 accepted/completed/failed trace evidence。

判断：协议路径本身符合开发宪法，应继续作为唯一目标路径固化。

### 3.2 Web shell 的直接 workspace 依赖已清理到终态

硬证据：

- `crates/shells/macaca-web/Cargo.toml:10-12`：web shell 生产 workspace direct dependency 仅剩 `macaca-proto` 和 `macaca-sdk`。
- `crates/tests/macaca-integration-tests/tests/route_c_dependency_boundaries/allowlist.rs`：allowlist 为 `vec![]`。
- `crates/tests/macaca-integration-tests/tests/os_layer_file_size_gate/allowlist.rs`：file-size allowlist 为 `vec![]`。

已通过命令：

```text
cargo test -p macaca-integration-tests --test route_c_dependency_boundaries
cargo test -p macaca-integration-tests --test kernel_purity_gate
cargo test -p macaca-integration-tests --test shell_dependency_purity_gate
```

结果：全部通过。

判断：直接依赖层面比 6 月 7 日审计显著改善。但 SDK bridge 仍隐藏 provider crate alias，见 §4.4。

### 3.3 `/api/chat/v2`、YAML delegate、WASM 主执行链已收敛到单链路

已通过命令：

```text
cargo test -p macaca-web unified_audit_replay_convergence_tests
cargo test -p macaca-web unified_delegation_path_tests
```

结果：

- `unified_audit_replay_convergence_tests`：6 passed。
- `unified_delegation_path_tests`：7 passed。

这些 static contract tests 证明：

- `/api/chat/v2` 主线程进入 `service.agent_execution`。
- YAML workflow delegate 不再保留 direct kernel/framework 并行 backend。
- WASM session 通过 host import bridge 汇入 `ServiceRuntime`。
- `delegate_task` 通过 `ServiceDelegatedTaskDispatcher` 进入 `service.agent_execution`。
- fork-join、goal lifecycle、session loop wake/register/shutdown 先经过 `service.execution_control`。

判断：主执行路径已经从“多条并行运行路径”收敛为“服务路径为主，本地 adapter 辅助”的形态。

### 3.4 hosted application execution 已删除旧的 authoritative 分支

6 月 7 日旧审计里最危险的一组 `legacy_unmarked / authoritative_seen / non_authoritative` 分支，在当前 hosted application execution 生产实现里已经消失。

硬证据：

- `crates/runtime/macaca-runtime-host/src/application_execution_hosted/host_signal_translator.rs:11-16` 明确说明统一调用路径下每条 host command row 都同等权威。
- `crates/runtime/macaca-runtime-host/src/application_execution_hosted/host_signal_translator.rs:63-104` 对 `queued|pending|completed|ok|failed` 做统一终态判断，不再按 compatibility graph owner 分流。

判断：这是正确修复，必须保留。

### 3.5 AgentScope 2.0 framework 生产源不再保留 1.0/compat fallback 标记

已通过命令：

```text
cargo test -p macaca-framework --test agentscope2_framework_boundaries
```

结果：2 passed。

硬证据：

- `crates/runtime/macaca-framework/tests/agentscope2_framework_boundaries.rs:10-25` 禁止 `macaca-compat`、`macaca-sdk`、`macaca-llm`、`macaca-tools`、`ReActAgent2`、`AgentRuntime2`、`AgentScope2RuntimeProvider`、`from_legacy_response`、`#[deprecated` 等进入 framework production source。

判断：`macaca-framework` 的 AgentScope 2.0 命名和兼容清理方向是正确的。

## 4. 未 100% 达标的硬证据

### 4.1 OS-layer 文件大小 gate 当前失败

已执行命令：

```text
cargo test -p macaca-integration-tests --test os_layer_file_size_gate
```

结果：失败。

失败原因：

```text
crates/runtime/macaca-framework/src/tool.rs: 504 lines (limit 500)
```

影响：

- 违反 `AGENTS.md` 与开发宪法的 “Rust / Macaca 默认规范”：OS-layer 生产文件不得超过 500 行。
- allowlist 已经是 0 行，不能通过新增 allowlist 掩盖；应拆分模块。

设计要求：

- 将 `tool.rs` 拆成 `tool/mod.rs`、`tool/runtime.rs`、`tool/registry.rs`、`tool/invocation.rs` 或等价按职责拆分。
- 拆分不能引入 AgentScope 1.0 fallback、`*2` 命名或 provider hardcoding。

### 4.2 Kernel 仍持有可替换告警传输实现

硬证据：

- `crates/kernel/macaca-kernel/Cargo.toml:9`：kernel 生产依赖 `reqwest`。
- `crates/kernel/macaca-kernel/src/alert.rs:131-167`：`WebhookAlertChannel` 在 kernel 内直接构造 `reqwest::Client` 并执行 HTTP POST。
- `crates/kernel/macaca-kernel/src/alert.rs:177-182`：`AlertManager::new` 根据 `webhook_url` 直接安装 webhook channel。

违宪点：

- HTTP webhook 是可替换通知/告警 service provider，不是 kernel invariant。
- kernel 可以保留 `Alert` identity、severity contract、dedup policy facade 或 trace/audit event，但不能持有 concrete HTTP transport。

设计要求：

- 将 webhook sender 移出 kernel，进入 `service.alert` 或 `service.notification` provider。
- kernel 只暴露 typed `AlertEvent` / `AlertPolicyDecision` / `AlertSinkPort`，默认实现应为 no-op 或 trace-only。
- runtime-host 作为 composition root 注册 log/webhook/plugin/remote/unavailable alert provider。

### 4.3 Kernel 仍保留 agent/task 编排语义模块

硬证据：

- `crates/kernel/macaca-kernel/src/orchestrator.rs:1-7`：模块说明为 “coordinates task delegation between agents”，包括 routing、parallel execution、result aggregation。
- `crates/kernel/macaca-kernel/src/orchestrator.rs:65-107`：`delegate_task` 创建和保存 delegated task。
- `crates/kernel/macaca-kernel/src/orchestrator.rs:159-195`：`find_best_agent` 基于 capability keyword 做 agent matching。
- `crates/kernel/macaca-kernel/src/orchestrator.rs:197-249`：`parse_command` 解析 `delegate_task`、`aggregate_results`、`report_to_coordinator` 工具语义。
- `crates/kernel/macaca-kernel/src/lib.rs:12` 和 `:38`：生产模块仍导出 `orchestrator::AgentOrchestrator`。

当前影响：

- 全局搜索显示 `AgentOrchestrator` 当前主要被导出和测试使用，主路径已通过 service-backed dispatcher 替代。
- 但只要生产模块仍在 kernel 中，历史债务就没有清零。

违宪点：

- task planning、delegation、worker-loop、tool-call parsing 属于 task/agent-execution service 或 application framework，不属于 kernel。
- kernel 不能解析工具名，也不能根据 prompt keyword 选择 agent。

设计要求：

- 删除 kernel `orchestrator` 生产导出。
- 如果仍需保留合同，迁移到 `macaca-proto` typed DTO + `service.agent_execution` / `service.task` provider。
- 测试 fixture 可迁入 test support，不得作为 kernel production API。

### 4.4 SDK `shell_provider_bridge` 仍是迁移型 facade，不是终态 facade

硬证据：

- `crates/facade/macaca-sdk/src/shell_provider_bridge.rs:1-27` 明确说明这是 presentation shell migration bridge。
- `crates/facade/macaca-sdk/src/shell_provider_bridge.rs:29-93` re-export `driver`、`llm`、`memory`、`skill`、`task`、`tools`、`kernel`、`agent`、`context`、`framework`、`app`、`runtime_host`。
- `crates/facade/macaca-sdk/Cargo.toml:12-34` SDK 直接依赖大量 capability、application、runtime-host、framework crate。

判断：

- 这使 shell 的 `Cargo.toml` gate 通过，但从架构终态看，SDK 仍然不是纯 provider-neutral facade。
- 当前 bridge 是明确可审计的过渡层，不应长期存在。

设计要求：

- SDK 只保留 provider-neutral clients、commands、results、Null Object/unavailable behavior。
- 逐步删除 shell 对 provider alias 的消费点。
- provider construction、runtime-host bootstrap、framework construction 均留在 runtime-host 或明确 host composition root，不通过 SDK re-export 给 shell。

### 4.5 Shell 仍保留 framework construction adapter 和本地执行通道

硬证据：

- `crates/shells/macaca-web/src/framework_agent_construction_shell_adapter.rs:1-10` 明确说明这是唯一仍 reach `FrameworkRunner` 的 web 入口，迁移到 runtime-host/framework 是 task 4.3.2。
- `crates/shells/macaca-web/src/framework_agent_construction_shell_adapter.rs:97-105` 仍调用 `FrameworkRunner::build_runtime_agent_from_context_snapshot_with_execution_policy(...)`。
- `crates/runtime/macaca-runtime-host/src/framework_runtime_agent_service.rs:1-10` 说明 runtime-host execution step 已服务化，但 construction 仍委托 shell port。
- `crates/shells/macaca-web/src/state.rs:187-214`：`ActiveSession` 仍保留 deprecated local pause flag、resume channel、SSE sender。
- `crates/shells/macaca-web/src/state.rs:234-246`：`LoopState` 仍持有 PlanLoop/WorkerLoop/Scheduler handles 和 wakers。
- `crates/shells/macaca-web/src/state.rs:382-389`：`tools`、`executor_registry`、`workspace_memory` 等仍是 compatibility/default construction anchors。

判断：

- 当前主执行服务路径已正确，shell 不再直接实现 `FrameworkRuntimeAgentPort`。
- 但 shell 仍是 framework runtime agent construction adapter 和非序列化本地通道的所有者，所以不能称为 100% thin shell。

设计要求：

- 将 `FrameworkRunner` construction 从 web shell 下沉到 runtime-host/framework construction service。
- shell 只保留 SSE rendering/subscription endpoint，不保留 pause/resume channel 的语义所有权。
- PlanLoop/WorkerLoop waker ownership 迁入 task/execution-control service；shell 只订阅事件。

### 4.6 Runtime-host 仍有 deprecated compatibility facade

硬证据：

- `crates/runtime/macaca-runtime-host/src/mcp_runtime/manager.rs:32-63`：`McpRuntimeManager` 标记 deprecated，但仍是实际 manager 结构。
- `crates/runtime/macaca-runtime-host/src/mcp_runtime/facade.rs`、`registration.rs`、`manager_invoke.rs` 仍通过 `#[allow(deprecated)]` 或 deprecated manager 运行。
- `crates/runtime/macaca-runtime-host/src/entitlement.rs:124-127`：`EntitlementRuntimeFacade` deprecated，提示改用 `EntitlementSystemServiceProvider` + `SystemEntitlementClient`。
- `crates/runtime/macaca-runtime-host/src/route_c_bootstrap.rs:31-40`、`:110-179`：Route C optional bootstrap 仍通过 deprecated entitlement facade 注册 Store/Entitlement、Payment、Web3、EVM。

构建/测试输出也出现大量 deprecation warning：

- `EntitlementRuntimeFacade` 仍被 entitlement/store/route_c/public_api 使用。
- `McpRuntimeManager` 仍被 MCP facade/registration/invoke 使用。

判断：

- runtime-host 是正确的 composition root，但 deprecated facade 未清零。
- 这不是 kernel 违宪，但属于“历史债务未清零”和“服务化不彻底”。

设计要求：

- MCP：把 manager 内部状态降为 private implementation detail，公共入口只保留 `McpRuntimeFacade` 和 service provider typed command。
- Entitlement/Store：删除 `EntitlementRuntimeFacade` 作为 app authorizer 的兼容入口，全部走 entitlement service client 或 provider-internal repository。
- Route C bootstrap：改名为 stable optional-service bootstrap，不再带 Route C/migration/deprecated 语义。

### 4.7 旧 `/api/chat` 兼容 route 仍以 deprecated 形式暴露

硬证据：

- `cargo test -p macaca-web unified_*` 编译期间出现 warning：`chat_orchestrator::route_legacy::post_chat` deprecated，提示 use `post_chat_v2`; legacy `/api/chat` is removed。
- `crates/shells/macaca-web/src/chat_orchestrator/mod.rs` re-export deprecated `post_chat`。
- `crates/shells/macaca-web/src/chat_orchestrator/route_legacy.rs` 仍存在生产函数。

判断：

- 如果兼容 route 不注册，仅保留未使用函数，也仍然是历史债务。
- “历史债务清零”要求删除兼容 route 或迁入测试 fixture，不应留在 production module export。

设计要求：

- 删除 `route_legacy::post_chat` production export。
- 保留 `/api/chat/v2` 作为唯一 chat route。
- 如消费代码仍调用旧符号，必须通过 OpenSpec 标记 breaking/migration，而不是继续兼容。

### 4.8 Application/package 层仍有 legacy API 和兼容 fallback

硬证据：

- `crates/application/macaca-app/src/lib.rs` 编译 warning 显示仍 re-export deprecated `app_agent_base_prompt`、`app_entry_agent_name_or`、`legacy_app_task_planning_contract`。
- `crates/application/macaca-app/src/workflow/engine.rs` 编译 warning 显示 `WorkflowEngine` 仍保留 `kernel`、`llm` 字段作为 future wiring，占据 direct provider-looking contract。
- `crates/application/macaca-app/src/consumption.rs` 中仍存在 legacy task planning contract 的 deprecated API。

判断：

- application framework 可以拥有 manifest compatibility checking，但不能把旧 YAML/task planning fallback API 作为长期 production surface。
- 这不直接破坏主调用链，但不满足“历史债务清零”。

设计要求：

- application framework 只保留 manifest v1/v2 projection 和 typed service capability projection。
- 删除 legacy prompt/task planning helpers 或迁移到 test-only fixtures。
- Workflow execution 必须进入 Application ABI + service path，不再保留 direct kernel/llm fields。

## 5. 对三部开发宪法的逐条对照

### 5.1 `macaca-os-architecture-governance.md`

| 条款 | 当前状态 | 证据/判断 |
| --- | --- | --- |
| Microkernel + Service Runtime + Application ABI + Plugin/Module Ecosystem | 部分满足 | ServiceRuntime/Application ABI 已建立；runtime-host deprecated facade 和 shell construction adapter 未清零。 |
| Dependencies must point downward, or cross boundaries through facades | 部分满足 | web/kernel production dependency gate 通过；SDK bridge 仍 re-export provider/runtime-host/app/framework。 |
| Web/CLI are adapters, not semantic owners | 部分满足 | shell direct execution 已收敛；但 shell 仍持有 framework construction、本地 wakers/channels、legacy route。 |
| Every OS-layer change must pass dependency-boundary and audit replay checks | 部分满足 | 多数 gate 通过；file-size gate 失败。 |
| No provider/app/model hardcoding below application layer | 部分满足 | 主路径静态 tests 禁止常见 role names；kernel `orchestrator` 仍解析 tool names。 |

### 5.2 `macaca-os-microkernel-boundaries.md`

| 条款 | 当前状态 | 证据/判断 |
| --- | --- | --- |
| Kernel owns only invariants | 未 100% | kernel 保留 HTTP webhook alert transport 和 AgentOrchestrator task delegation semantics。 |
| Kernel must not construct concrete providers | 未 100% | `WebhookAlertChannel` 构造 `reqwest::Client`。 |
| Task planning/execution/review/recovery are non-kernel | 未 100% | `AgentOrchestrator` 在 kernel production source 中仍存在。 |
| Optional modules may be absent | 基本满足 | Web3/EVM 在 runtime-host bootstrap 中以 unavailable provider 注册；但 route_c/deprecated 命名未清理。 |
| Shells must not define task/session/application semantics | 部分满足 | execution path service-backed；local session loop/wakers/channels 仍在 shell。 |

### 5.3 `macaca-os-serviceization-allowlist.md`

| 条款 | 当前状态 | 证据/判断 |
| --- | --- | --- |
| Serviceization is ownership transfer, not file movement | 部分满足 | execution control 和 agent execution ownership 已迁移很多；MCP/entitlement still compatibility facade。 |
| Every service call carries trace context | 主路径满足 | `TraceRequiredMiddleware` 和 `TraceRequiredRuntimeDecorator` 存在；非 service compatibility surfaces 仍需清理。 |
| Structured unavailable/unsupported/denied/failure | 基本满足 | ServiceRuntime/optional modules 已结构化；legacy helpers 仍可能绕过服务错误模型。 |
| Built-in/plugin/remote/mock/unavailable replacement | 部分满足 | 多数 service provider 已有；alert webhook 在 kernel 内不可替换为 service provider。 |
| Active migration allowlist terminal state zero | 依赖 allowlist 满足 | route_c allowlist 为 0；file-size allowlist 为 0 但 gate 失败说明新超限未被 allowlist。 |

## 6. 新目标设计：从“主链路收敛”推进到“债务清零”

### 6.1 唯一路径定义

终态仍使用 6 月 7 日目标设计，但本轮更新重点是把“服务路径为主”升级为“只有服务路径”：

```text
Shell / SDK / Application ABI / WASM host import / plugin
  -> focused SDK client or Application ABI adapter
  -> ServiceRouter.route
  -> ServiceRuntime decorators
  -> ServiceBus
  -> SystemServiceBusHandler
  -> ServiceCallExecutor
  -> SystemService provider
```

例外只允许两类：

- Local no-side-effect presentation operation，例如 SSE subscribe/render，不产生 OS capability side effect。
- Test-only fixture，不进入 production source。

凡是会执行 agent、task、tool、driver、skill、MCP、memory、context、payment、web3、evm、alert、notification、application lifecycle 的行为，必须进入 service path。

### 6.2 Kernel 纯净化目标

kernel 最终只保留：

- identity：app、agent、session、task、service、capability、package、tenant。
- registry：service/capability/agent identity registry。
- policy facade：抽象裁决，不持有 provider。
- trace/audit primitive：append-only evidence contract。
- scheduler primitive：公平/唤醒语义，不实现业务 worker loop。
- resource primitive。
- session/task state primitive：状态契约，不实现 planner/executor。
- service call executor / service bus bridge。
- typed execution port identity：只能指向 service client adapter 或 unavailable adapter。

必须移出或删除：

- `AlertManager` 的 webhook HTTP sender。
- `AgentOrchestrator` production module。
- kernel production dependency `reqwest`。
- kernel tool-name parsing、prompt keyword routing、task result aggregation semantics。

### 6.3 Shell 终态

`macaca-web` / `macaca-cli` 只允许：

- parse HTTP/CLI input。
- call `SystemFacade` or focused SDK clients。
- render state、GenUI、approval、trace、diagnostics。
- subscribe to events。

必须迁出：

- framework agent construction。
- PlanLoop/WorkerLoop/Scheduler lifecycle ownership。
- pause/resume/waker/channel semantic state。
- workspace memory default provider anchors。
- executor registry ownership。
- deprecated `/api/chat` route。

Shell 可保留：

- SSE sender/socket handles。
- request DTO parsing。
- renderer/view model state。
- thin composition handle supplied by runtime-host, but not provider construction logic。

### 6.4 Runtime-host 终态

runtime-host 是唯一 provider/module composition root，但不能长期暴露 deprecated facade。

必须稳定化：

- `RouteCOptionalServicesBootstrap` 改为 stable `OptionalServiceBootstrap` 或同等命名。
- `EntitlementRuntimeFacade` 删除 public deprecated API，保留 `EntitlementSystemServiceProvider` + `SystemEntitlementClient`。
- `McpRuntimeManager` 私有化或删除 deprecated public surface。
- `FrameworkAgentConstructionPort` 从 shell adapter 下沉到 runtime-host/framework service。

runtime-host 可以持有：

- concrete provider factory。
- plugin/remote/mock/unavailable provider。
- service decorators。
- sanitized diagnostics。
- local durable repositories。
- optional module bootstrap。

### 6.5 SDK 终态

SDK 不应通过 `shell_provider_bridge` re-export provider crates 作为长期解法。

终态 SDK 只保留：

- provider-neutral commands/results/errors。
- focused clients。
- `SystemFacade`。
- Null Object / unavailable clients。
- test helpers 不进入 production public surface。

必须删除：

- `shell_provider_bridge` 中 provider/runtime-host/application/framework crate alias。
- SDK 对 runtime-host/app/framework 的生产依赖，除非经过明确 OpenSpec 批准为 facade type-only edge，并且没有 provider construction API 暴露。

## 7. 整改优先级

### P0：先恢复宪法 gate

1. 拆分 `crates/runtime/macaca-framework/src/tool.rs`，让 `os_layer_file_size_gate` 重新通过。
2. 增强 file-size gate 报告，把 `macaca-framework` AgentScope2 文件纳入长期监控。

验收：

```text
cargo test -p macaca-integration-tests --test os_layer_file_size_gate
cargo test -p macaca-framework --test agentscope2_framework_boundaries
```

### P0：清 kernel 非内核能力

1. 将 alert webhook transport 移到 runtime-host alert/notification service provider。
2. 删除 kernel `reqwest` 生产依赖。
3. 删除或迁移 kernel `AgentOrchestrator` production module。
4. 增强 kernel purity gate：禁止 `reqwest`、HTTP client、webhook transport、task orchestration/tool parsing production code 进入 kernel。

验收：

```text
cargo tree -p macaca-kernel --depth 1
cargo test -p macaca-integration-tests --test kernel_purity_gate
```

### P1：清 shell 兼容执行锚点

1. 将 `FrameworkRunner` runtime agent construction 移到 runtime-host/framework。
2. 删除 `WebFrameworkAgentConstructionPort` 或降为 test-only。
3. 删除 deprecated `/api/chat` production route/export。
4. 将 local loop/waker/channel 语义迁入 task/execution-control service，shell 只做 subscription。

验收：

```text
cargo test -p macaca-web unified_audit_replay_convergence_tests
cargo test -p macaca-web unified_delegation_path_tests
cargo test -p macaca-integration-tests --test shell_dependency_purity_gate
```

### P1：清 runtime-host deprecated facades

1. MCP public entry 只保留 `McpRuntimeFacade` + service provider，`McpRuntimeManager` 不再 public deprecated。
2. Entitlement/Store 全部走 service provider + SDK client，删除 `EntitlementRuntimeFacade` public deprecated surface。
3. Route C bootstrap 稳定命名，去除 migration/deprecated 语义。

验收：

```text
cargo test -p macaca-runtime-host
rg -n "#\\[deprecated|allow\\(deprecated\\)|Route C|compatibility facade" crates/runtime/macaca-runtime-host/src
```

### P2：清 SDK bridge

1. 为 shell 仍使用的每个 provider alias 建立替代 focused client 或 runtime-host construction API。
2. 删除 `shell_provider_bridge` alias。
3. SDK 依赖收敛为 proto + provider-neutral facade contracts。

验收：

```text
cargo tree -p macaca-sdk --depth 1
rg -n "shell_provider_bridge|pub use macaca_.* as" crates/facade/macaca-sdk/src
```

### P2：清 application legacy helpers

1. 删除 app prompt/task planning legacy helper 的 production re-export。
2. Workflow engine 删除 direct kernel/llm fields 或改为 Application ABI service client。
3. Manifest compatibility checker 保留为 package/application admission capability，不与 legacy execution fallback 混用。

验收：

```text
cargo test -p macaca-app
rg -n "legacy_|#\\[deprecated|allow\\(deprecated\\)" crates/application/macaca-app/src
```

## 8. 必须新增或增强的 gate

当前 gate 能发现直接 workspace 依赖和文件超限，但还不能完整证明“语义债务清零”。建议新增：

1. **kernel-no-network-provider-gate**
   - 禁止 kernel production source 使用 `reqwest`、HTTP webhook、RPC client、provider transport。

2. **kernel-no-orchestration-semantics-gate**
   - 禁止 kernel production source 包含 task delegation、tool-call parser、worker-loop、prompt keyword routing。

3. **sdk-no-provider-reexport-gate**
   - 禁止 SDK production source re-export concrete provider/runtime-host/application/framework crate alias 给 shell。

4. **runtime-host-no-deprecated-public-facade-gate**
   - 禁止 runtime-host public API re-export deprecated facade。

5. **shell-no-framework-construction-gate**
   - 禁止 shell production source 调用 `FrameworkRunner::build_runtime_agent*`。

6. **shell-no-legacy-route-gate**
   - 禁止 production route export deprecated `/api/chat`。

7. **no-production-deprecated-gate**
   - 对 OS-layer production source 中 `#[deprecated]`、`#[allow(deprecated)]` 建立 allowlist=0 的终态 gate；测试 fixtures 可例外。

## 9. 设计原则确认

后续实现必须继续遵守：

- 不写 application 专有代码。
- 不硬编码 application workflow、agent role、driver/provider/model/gateway/chain/payment name。
- 不通过 shell/provider shortcut 伪造 service success。
- Optional module 缺席返回 structured unavailable/disabled/denied。
- 关键执行节点必须有 `tracing`，字段使用 service_id、command、trace_id、reason_code 等 provider-neutral 维度。
- 日志、trace、snapshot 不写 raw secret、prompt、manifest、WASM bytes、package bytes、private key、credential、raw signature、raw provider payload、unbounded output。
- 所有非显然代码必须有英文注释解释功能、运行原理和设计权衡。
- Rust production file 不超过 500 行。

## 10. 最终 Definition of Done

只有同时满足以下条件，才能宣称 100% 达成目标：

1. `route_c_dependency_boundaries`、`kernel_purity_gate`、`shell_dependency_purity_gate`、`os_layer_file_size_gate` 全部通过。
2. `macaca-framework` AgentScope2 boundary gate 通过，且无超 500 行生产文件。
3. `/api/chat/v2`、session recovery、YAML、WASM、GenUI、trace replay、optional-provider unavailable 端到端场景全部通过。
4. kernel production deps 不包含 provider/runtime/application/facade/network transport。
5. kernel production source 不包含 alert webhook、agent task orchestration、tool parser、prompt routing。
6. shell production source 不调用 `FrameworkRunner::build_runtime_agent*`，不持有 loop/waker/channel 语义所有权。
7. runtime-host public API 不再 re-export deprecated compatibility facade。
8. SDK 不再通过 `shell_provider_bridge` re-export provider/runtime-host/application/framework。
9. OS-layer production source 中 `legacy`、`compat`、`Route C migration`、`#[deprecated]`、`#[allow(deprecated)]` 的债务 allowlist 为 0。
10. OpenSpec baseline 与上述事实一致。

## 11. 下一步 OpenSpec 建议

建议新建 OpenSpec change：

```text
remove-protocol-microkernel-residual-debt
```

影响 specs：

- `microkernel-boundaries`
- `service-runtime`
- `system-facade`
- `agent-execution`
- `execution-control`
- `application-framework`
- `runtime-host-bootstrap`
- `framework-agentscope2`

proposal 应明确：

- 这是债务清零，不是新增业务能力。
- 第一阶段先恢复 gate 和 kernel purity。
- 第二阶段迁出 shell construction/local loop ownership。
- 第三阶段删除 runtime-host/SDK/application deprecated surfaces。
- 每一步都必须有 contract test 和 boundary gate，不能靠注释声明完成。
