# Macaca OS 统一调用路径与协议化/微内核化全面审计

日期：2026-06-07
范围：`macaca/`（Rust workspace，25 个 crate）
性质：只读审计，不改动代码。

## 1. 审计目标

本次审计聚焦一个核心问题，并由此延伸到三项治理升级：

1. **单一调用路径（North Star）**：无论上层应用是 YAML app、WASM app、GenUI app 还是 headless app，最终调度/调用任何服务能力（LLM、tool、driver、skill、task、MCP、agent execution、payment、web3…）都必须收敛到**同一条协议化路径**。任何"第二条路"都是缺陷。
2. **协议化（Protocol）**：跨边界操作必须是 typed command/result，经过强制 trace、policy、audit，禁止直接函数调用旁路。
3. **微内核化（Microkernel）**：内核只持有系统不变量；一切可替换能力服务化/模块化。
4. **回归清爽架构**：删除历史债务代码、兼容代码、application 专有/硬编码逻辑。

## 2. 审计依据

三部稳定治理宪法（baseline）：

- `macaca/docs/macaca-os-architecture-governance.md`
- `macaca/docs/macaca-os-microkernel-boundaries.md`
- `macaca/docs/macaca-os-serviceization-allowlist.md`

以及前序审计 `macaca/docs/2026-05-16-macaca-os-serviceization-implementation-audit.md`（本文是其延续与深化，结论一致并补充"多调用路径"专项分析）。

## 3. 执行摘要（结论）

| 维度 | 结论 | 严重度 |
|------|------|--------|
| 单一调用路径 | **未达成**。协议化的 canonical 路径已存在，但与之并存的 legacy 执行路径仍在生产代码中运行，靠 `graph_owner / authoritative / legacy_unmarked / suppress_executor_lifecycle` 等标记"和稀泥"协调终态。 | P0 |
| 微内核纯净 | **未达成**。内核反向依赖 application/facade/service 层；内核内置 web3 / evm / a2a / payment / executor(worker-loop) / provider_compat 等非内核能力。 | P0 |
| 协议化覆盖 | **部分达成**。`ServiceRuntime → ServiceBus → ServiceCallExecutor → SystemService` 是干净的协议路径，但 kernel `Agent::run(llm,tools)`、web `FrameworkRunner.agent.reply()`、`framework_toolkit` 直读 driver/MCP runtime 等旁路绕过它。 | P0 |
| Shell 瘦身 | **未达成**。`macaca-web` 仍是 execution / tools / session loop / provider runtime 的语义所有者；存在 2600+ 行巨型文件。 | P0 |
| 历史债务 | **大量保留**。`compat.rs`、`provider_compat.rs`、`kernel_builder.rs`、`loop_manager.rs` 等集中了 deprecated/legacy/migration 代码；依赖 gate 仍 allowlist 放行 10 条 forbidden edge。 | P0 |
| 设计模式 | **总体良好**。Command / Adapter / Bridge / Strategy / Decorator / Observer / Memento 使用得当，但被双路径稀释。 | P2 |
| 可trace可审计/日志 | **基本达成**。关键节点有 `tracing` + trace-bus + audit sink + replay；但旁路执行不产出 service-call evidence。 | P2 |
| 英文注释 | **覆盖良好**。核心模块注释详尽、解释意图与权衡；少量巨型文件可读性下降。 | P3 |

一句话总结：**Macaca OS 已经"建好了正确的那条路"，但还没有"拆掉错误的那几条路"。** 当前架构处在"协议化骨架 + legacy 双轨"的迁移中态，治理文档把这些 legacy 轨道定义为**债务而非可接受终态**。

---

## 4. 第一部分：调用路径全景与"多路径"问题（核心）

### 4.1 协议化的 canonical 路径（理想，已存在）

服务能力调用的唯一正确路径，链路如下（均有代码实现）：

```
应用/Shell
  → ServiceRouter.route()            (runtime-host/service_router.rs：contract→policy→retry→timeout→audit)
  → ServiceRuntime.call()            (runtime-host/service_runtime.rs)
  → ServiceBus                       (foundation/macaca-ipc)
  → SystemServiceBusHandler.handle() (kernel/service_bus_bridge.rs：envelope→command)
  → ServiceCallExecutor.execute()    (kernel/service_call.rs：TraceRequired 中间件 + trace 发射)
  → SystemService.call()             (具体 service provider)
```

证据：
- `macaca/crates/runtime/macaca-runtime-host/src/service_runtime.rs:42`（`ServiceRuntime` 明确"hosted in runtime-host, not in the kernel"）。
- `macaca/crates/runtime/macaca-runtime-host/src/service_runtime.rs:115`（`ServiceRuntime` 注册 handler 时**复用** kernel 的 `SystemServiceBusHandler`，证明二者不是平行实现，而是分层叠加）。
- `macaca/crates/kernel/macaca-kernel/src/service_call.rs:31`（`TraceRequiredMiddleware`：no trace, no call）。
- WASM 应用确实走这条路：`macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/host_import_bridge.rs:238` 的所有 host import 都经 `self.router.route(...)`。

**结论**：协议路径本身是优雅的（Command + Chain of Responsibility + Decorator），应作为目标态保留。

### 4.2 实际并存的"第二/第三条路"（问题）

| # | 路径 | 入口 | 是否经 canonical service.call | 证据 |
|---|------|------|------------------------------|------|
| P-WASM | WASM 应用 → host import → ServiceRouter | `host_import_bridge.rs` | ✅ 是 | `host_import_bridge.rs:238` |
| P-KERNEL | Kernel `execute_agent` → `AgentExecutionPort` → legacy `Agent::run(llm,tools)` | `kernel.rs:85` | ❌ 否（直连 provider） | `kernel.rs:98-101`、`provider_compat.rs:65-71` |
| P-WEBEXEC | Web `WebAgentExecutionBackend` → `FrameworkRunner` → `agent.reply()` | `agent_execution_backend.rs:701` | ⚠️ 部分（context/控制走 service，**模型与工具执行不走**） | `agent_execution_backend.rs:836-849` |
| P-DELEGATE | 协调者 `delegate_task` 工具 → kernel `ApplicationExecutor`/`ForkManager` → Worker AgenticLoop | `macaca-tools/src/orchestration.rs`、`kernel/executor/app_executor.rs`、`web/loop_manager.rs` | ❌ 否 | `kernel/executor/fork_manager.rs`、`web/loop_manager.rs` |
| P-TOOLKIT | Web `framework_toolkit` 直读 driver/MCP runtime 收集工具 | `framework_toolkit.rs` | ❌ 否 | 前序审计 `framework_toolkit.rs:107`(`driver_runtime.collect_tools()`)、`:274`(`mcp_runtime.definitions()`) |
| P-YAML | YAML workflow → `macaca-app/src/workflow.rs` → web `agent_runner.rs` | `macaca-app/src/workflow.rs`、`web/agent_runner.rs` | ⚠️ 取决于其落到 P-WEBEXEC 还是 P-DELEGATE | `macaca-app/src/workflow.rs` |

> 关键判断：**同一个"运行一个 agent / 调用一个工具"的能力，存在 P-KERNEL、P-WEBEXEC、P-DELEGATE 三种执行入口**。这就是用户要找的"多条调用路径"。它们对 LLM/tool 的实际调用并非都经过 `ServiceCallExecutor`，因此 trace/policy/audit 的强制性被打破。

### 4.3 "多路径兼容标记"证据清单（最直接的证据）

这些标记之所以存在，正是因为系统在试图让多条路径的**终态**对齐，而不是消灭多余路径：

1. `application_execution_hosted.rs:633-660`：
   > "During the migration to a single execution path, older component host rows may not yet carry a graph-owner marker. If no row is marked, preserve the previous behavior by treating all rows as authoritative."
   - 引入 `authoritative_seen / authoritative_completed / authoritative_failed / non_authoritative_failed`（行 546-695）来区分"真正的 application_execution 路径"与"compatibility/diagnostic 路径"。
   - `graph_owner.unwrap_or("legacy_unmarked")`（行 627）。

2. `host_import_bridge.rs:43`：`APPLICATION_EXECUTION_GRAPH_OWNER = "application_execution"`，并在 `host_import_bridge.rs:279-293` 给 task 打 `graph_owner` 标记——目的就是"separate real run terminal facts from compatibility diagnostics"。

3. `agent_execution_backend.rs:151-163`：`should_emit_executor_lifecycle` / `suppress_executor_lifecycle`：
   > "Some legacy kernel/executor callers already emit task lifecycle events around the `AgentRunner` trait … duplicate started/completed events would make session traces noisy."
   - 即：因为存在 P-KERNEL 与 P-WEBEXEC 两条会各自发事件的路径，只能用抑制标记去重。

4. `agent_execution_backend.rs:386-513`：`legacy_execution_control_policy` / `"legacy_chat_main_thread_goal_pause"`：
   > "Until manifest projection is wired for legacy YAML chat sessions, ChatMainThread receives a deprecated compatibility policy."

5. `application_execution_hosted.rs:588-617`：对 host import 状态 `queued|pending|completed|ok|其他` 的兼容性归一化，理由是"diagnostic or compatibility evidence cannot be misclassified as a failure"。

**评估**：以上全部是"双轨协调"补丁。它们让系统能跑，但与"单一调用路径"治理目标直接冲突。一旦消灭 legacy 轨道，这些 `graph_owner / authoritative / suppress_executor_lifecycle / legacy_*` 标记应全部删除。

---

## 5. 第二部分：微内核边界违规

### 5.1 依赖方向越界（来自 `Cargo.toml`，硬证据）

宪法要求依赖向下，内核不得依赖上层/具体 service/facade。实测：

- `macaca-kernel`（kernel 层）依赖：`macaca-agent`(application)、`macaca-sdk`(facade)、`macaca-driver`/`macaca-gateway`/`macaca-skill`/`macaca-task`/`macaca-tools`(service)。
  - 证据：`macaca/crates/kernel/macaca-kernel/Cargo.toml:8,24-29`。
  - **违反**：`microkernel-boundaries.md`「kernel must not depend on concrete provider implementations / application-framework implementations」。
- `macaca-persist`（foundation 层）依赖 `macaca-context`（service 层）。
  - 证据：`macaca/crates/foundation/macaca-persist/Cargo.toml:7`。
  - **违反**：foundation 反向依赖 service。
- `macaca-web`（shell）直接依赖 `macaca-kernel`、`macaca-runtime-host` 及 `driver/llm/memory/persist/skill/task/tools` 等具体 crate。
  - 证据：`macaca/crates/shells/macaca-web/Cargo.toml:7-22`。
  - **违反**：shell 应只依赖 SDK/SystemFacade。

可执行 gate 现状：`route_c_dependency_boundaries` 仍以 allowlist **放行 10 条** forbidden edge（`tests/route_c_dependency_boundaries/allowlist.rs:12-115`）：
- kernel → driver / gateway / skill
- web → driver / llm / memory / persist / skill / task / tools

> gate 只能阻止"新增未登记债务"，不能证明已达治理终态。10 条 allowlist = 10 笔未偿还的架构债。

### 5.2 内核持有非内核能力（来自 `kernel/src/lib.rs`，硬证据）

`microkernel-boundaries.md` §"What The Kernel Must Not Own" 明确列出 Web3/EVM/wallets/chain、Payment/A2A、worker-loop/planner、provider 等为**非内核**。但内核实际包含：

| 内核模块 | 应属层 | 证据 | 违反 |
|----------|--------|------|------|
| `web3.rs` / `web3_event.rs` | optional module / service | `lib.rs:37-38,86-92` 导出 `Web3Adapter/MockWeb3Adapter/UnavailableWeb3Adapter/Web3Facade` | Web3 入内核 |
| `evm.rs` / `evm_adapter.rs` / `evm_event.rs` | optional module / service | `lib.rs:8-10,43-47` 导出 `EvmAdapter/MockEvmAdapter` | EVM 入内核 |
| `a2a.rs` / `a2a_event.rs` | service (payment) | `lib.rs:97-104` 导出 `A2ACoordinator/A2APaymentFacade/LocalSimulatedA2AAdapter` | A2A 支付入内核 |
| `payment_policy.rs` | service (payment policy) | `lib.rs:54` | 支付能力入内核 |
| `provider_compat.rs` | （删除目标） | `lib.rs:65-69` 导出 `LegacyLlmProvider/LegacyToolCatalog/AgentExecutionPort` | LLM/tool provider 入内核 |
| `executor/`（`ApplicationExecutor/ForkManager/AgentRunner/TaskRouter/WorkerSupervisor`） | service (task/execution) | `lib.rs:105-111` | worker-loop/执行编排入内核 |

补充说明：
- `evm.rs`、`web3.rs`、`a2a.rs` 中确有 `Mock*`/`LocalSimulated*`/`Unavailable*` **具体 adapter 实现**，不仅是 policy facade，违反"kernel must not construct concrete providers"。
- `lib.rs:94-101` 注释自承："The A2A symbols are deliberately kept as deprecated compatibility anchors so downstream migrations can locate the old kernel-owned payment path"——即明确知道这是 legacy 锚点。

### 5.3 内核仍可携带直连 provider 执行

- `kernel.rs:42-53`：`Kernel::new(config, llm, tools)`（deprecated）仍接收 `LegacyLlmProvider/LegacyToolCatalog`。
- `kernel.rs:98-101`：`execute_agent` 经 `AgentExecutionPort.execute_registered_agent`，其 legacy 实现 `provider_compat.rs:65-71` 直接 `LegacyAgentExecutionAdapter::new(llm, tools)`，**不产出 service-call evidence**。
- `kernel.rs:103-106` 注释："The current compatibility behavior marks agents idle … Preserve that observable transition while the execution service is introduced." —— 又一处 compatibility 中态。

> 注：`AgentExecutionPort` 的引入（用 typed port 替代直传 provider）是**正确方向**，应保留并强化为"只接 service client，不接 provider trait"。

### 5.4 Shell 拥有系统语义（`macaca-web`）

- `agent_execution_backend.rs:836-849`：web shell 直接 `FrameworkRunner::build_runtime_agent…` 后 `agent.reply()`，由 shell 拥有"运行 agent"的语义与模型/工具执行——违反"shells must not define system semantics"。
- 前序审计已记录：`state.rs` 仍保留 deprecated `runtime/llm provider/router/memory runtime/mcp runtime/driver registry` 字段（`state.rs:292,346,360,367,370,373`）；`framework_toolkit.rs:107,274` 直读 driver/MCP runtime。
- 巨型文件（违反 500 行规则，且集中在本应最薄的 shell）：`loop_manager.rs`(~2629)、`framework_runner.rs`(~2484)、`chat_orchestrator.rs`(~1581)、`lib.rs`(~975)。

---

## 6. 第三部分：协议化程度审计

| 能力 | 是否有 typed command/result | 是否强制 trace | 是否有旁路 | 旁路证据 |
|------|------------------------------|----------------|------------|----------|
| service.call 通用 | ✅ `macaca-proto` ServiceCommand/Result/Error | ✅ TraceRequired | — | — |
| LLM | ✅ | ✅（经 service 时） | ✅ | kernel `Agent::run`、web framework agent 直连 |
| Tool | ✅ tool_service | ✅（经 service 时） | ✅ | `framework_toolkit` 直读 runtime |
| Driver/MCP | ✅ | ✅（经 service 时） | ✅ | `state.driver_runtime` / `state.mcp_runtime` 直读 |
| Task | ✅ task_service | ✅ | ⚠️ | kernel executor / web loop_manager 直接驱动 |
| Agent execution | ✅ `agent_execution_service` | ✅（context/控制） | ✅ | 模型/工具执行不经 service.call |
| Payment/A2A/Web3/EVM | ✅（已有 service provider in runtime-host） | ✅ | ✅ | kernel 内仍有 facade/adapter 入口 |

结论：**协议 DTO 层完备且优雅**（`macaca-proto` 提供 provider-neutral 类型），缺口在"强制所有调用方都走协议、删除直连旁路"。这与 §4.2 的多路径问题同源。

---

## 7. 第四部分：历史债务 / 兼容代码清单（量化）

按 `deprecated|legacy|compat|migration|escape hatch|preserve the previous|TODO|FIXME` 扫描 `.rs`（生产 + 测试），热点（命中数）：

| 文件 | 命中 | 性质 |
|------|------|------|
| `web/loop_manager.rs` | 64 | session loop 债务集中地 |
| `runtime-host/src/mcp_runtime.rs` | 57 | 未充分服务化的 runtime |
| `runtime-host/src/compat.rs` | 44 | 显式 compatibility 模块 |
| `kernel/kernel_builder.rs` | 39 | 含 `KernelServiceClientCompat` 等 |
| `services/macaca-task/src/runtime.rs` | 32 | task 运行时债务 |
| `facade/macaca-sdk/src/task_client.rs` | 32 | task client 迁移痕迹 |
| `kernel/provider_compat.rs` | 25 | legacy provider 桥 |
| `kernel/src/a2a.rs` | 10 | deprecated 支付锚点 |
| `kernel/src/lib.rs` | 9 | `#[allow(deprecated)]` 导出 |
| `tests/serviceization_escape_hatches.rs` | 11 | 逃逸口冻结测试 |

> 说明：部分命中属正当业务（如 `application/macaca-app/src/compatibility_checker*` 是"应用包兼容性检查"功能，非债务；`wasm_supply_chain` 同理）。但 kernel/web/runtime-host 中 compat/legacy 的密集分布，与治理目标"回归清爽架构"直接冲突。

债务三大根：
1. **kernel provider/能力债**：`provider_compat.rs`、`web3/evm/a2a/payment_policy`、`kernel_builder` compat、`scheduler`(`#[allow(deprecated)]`)、`persistence`(deprecated payment store)。
2. **web shell 债**：direct provider fields、framework_toolkit、loop_manager、巨型文件。
3. **执行双轨债**：`graph_owner/authoritative/suppress_executor_lifecycle/legacy_*` 协调补丁。

---

## 8. 第五部分：设计模式 / 可trace可审计 / 日志 / 注释

### 8.1 设计模式（总体良好，符合 `design_patterns.md` 与治理"Required Design Patterns"）
- Command + Chain of Responsibility：`service_call.rs` middleware。
- Adapter/Bridge：`service_bus_bridge.rs`、`host_import_bridge.rs`、`ApplicationAbiHostedExecutionAdapter`。
- Strategy：`HostedApplicationExecutionAdapter`、`PolicyEngine`、scheduler factory、completion policy。
- Decorator：`service_runtime.rs:68-75` 的 admission decorator 链（Trace→Policy→Resource→Entitlement→Metering→Audit）。
- Observer：`TraceEventBus`、`ServiceRuntimeEventSink`、event mirror。
- Memento：`HostedRunState` + 持久 EventLog replay；checkpoint_ref。
- Repository：`ApplicationGenUiSurfaceStore`。

问题：模式应用本身优雅，但被"双路径"稀释——例如 Decorator 链只对走 service 的调用生效，旁路调用享受不到。

### 8.2 可trace可审计
- 强制 trace：`TraceRequiredMiddleware`（kernel）、`TraceRequiredRuntimeDecorator`（runtime-host）。
- audit replay：`host_import_bridge.rs:604-618` 支持按 trace_id / session_id replay service-call 审计链。
- 脱敏：`host_import_bridge.rs:1086-1122` 的 `sanitize_json/is_safe_metadata_key`（过滤 raw/prompt/secret/payload/token）。
- 缺口：旁路执行（P-KERNEL/P-WEBEXEC 模型与工具调用）不进入 service-call audit 链，**审计存在盲区**。

### 8.3 日志
- 关键节点普遍有 `tracing::info!/warn!`（service_call 接受/完成/失败/拒绝；host import 准入/完成/拒绝；kernel 执行起止）。符合"关键执行节点有日志"。
- 缺口：旁路路径日志以 provider-name 为主（如 `provider_compat.rs:37` 记录 `llm_provider`），而非 service-call evidence。

### 8.4 英文注释
- 核心模块注释详尽，解释"功能 + 运行原理 + 为何如此"（如 `application_execution_hosted.rs` 大段意图注释、`host_import_bridge.rs` 模块级解释）。符合 AGENTS.md「所有代码都必须有详尽注释」。
- 缺口：巨型文件（loop_manager/framework_runner）因体量过大导致可维护性下降，注释难以覆盖局部复杂分支。

---

## 9. 违宪条款对照表

| 宪法条款 | 状态 | 关键证据 |
|----------|------|----------|
| governance「Dependencies must point downward」 | ❌ | kernel→agent/sdk/service；persist→context；web→一切 |
| microkernel「kernel must not own Web3/EVM/Payment/A2A」 | ❌ | kernel `web3/evm/a2a/payment_policy` |
| microkernel「kernel must not own worker-loop/planner」 | ❌ | kernel `executor::ApplicationExecutor/ForkManager/AgentRunner` |
| microkernel「providers not constructed outside composition roots」 | ❌ | kernel `provider_compat`、`Mock*/LocalSimulated*` adapter |
| allowlist「No service call without trace / policy」 | ⚠️ | 协议路径满足；旁路绕过 |
| allowlist「OS routing must not branch on provider/model/app name」 | ⚠️ | `macaca-llm/router.rs` 按 provider/model 名分支（service 层，较轻）；少量 `coordinator/planner/worker` fallback |
| governance「Shells must not be semantic owners」 | ❌ | web 拥有 execution/toolkit/loop/runtime |
| serviceization「single migration module for direct refs」 | ⚠️ | 已有 escape-hatch freeze，但旁路仍散落 |
| AGENTS.md「单文件 ≤500 行」 | ❌ | loop_manager/framework_runner/chat_orchestrator 严重超标 |

---

## 10. 风险与影响

- **审计盲区风险（高）**：旁路执行不产 service-call evidence，违背"7×24 零干预自治需要全链路可审计"的项目定位。
- **行为漂移风险（高）**：双路径终态靠标记协调，任一路径改动易引入 replay/终态不一致（`graph_owner` 逻辑已是脆弱补丁）。
- **可扩展性风险（中）**：新应用类型（量化交易等）若复用 legacy 轨道，会继承全部债务；治理自检"换个 application 还能工作吗"在 legacy 轨道上不成立。
- **删除风险（中）**：web3/evm/a2a/payment/executor 移出内核涉及面广，需分阶段、配合 boundary gate 与 OpenSpec。

---

## 11. 审计结论

Macaca OS 的协议化骨架（ServiceRuntime/ServiceRouter/ServiceCallExecutor/SystemService + provider-neutral proto + decorator 链 + audit replay）**设计正确且优雅**，应作为唯一目标路径固化。当前主要工作不是"再建抽象"，而是**收敛与删除**：

1. 把 P-KERNEL / P-WEBEXEC / P-DELEGATE / P-TOOLKIT 全部并入 canonical service.call 单路径；
2. 删除内核内非内核能力（web3/evm/a2a/payment/executor/provider_compat）与对应越界依赖；
3. 把 web 还原为 thin shell；
4. 删除 `graph_owner/authoritative/suppress_executor_lifecycle/legacy_*` 协调补丁与 10 条 allowlist 债务；
5. 对齐 OpenSpec baseline。

目标态与实施路径见同目录：
- `2026-06-07-macaca-os-protocol-microkernel-target-design.md`（目标架构设计）
- `2026-06-07-macaca-os-debt-elimination-refactor-plan.md`（重构方案）
