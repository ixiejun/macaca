## Context

本设计是 2026-06-07 审计三件套的执行蓝图，将 `2026-06-07-macaca-os-protocol-microkernel-target-design.md`（目标态）与 `2026-06-07-macaca-os-debt-elimination-refactor-plan.md`（重构方案）落成可被 `tasks.md` 逐条执行的技术决策。

约束（不可协商）：
- 三部宪法优先于实现便利：`macaca-os-architecture-governance.md`、`macaca-os-microkernel-boundaries.md`、`macaca-os-serviceization-allowlist.md`。
- 不写 application 专有代码、不硬编码 application 业务逻辑、不硬编码 agent/provider/model/driver/gateway/chain/payment/app 名（fixtures/tests 除外）。
- 所有新代码必须有详尽英文注释（功能 + 运行原理 + 设计权衡）。
- 关键执行节点必须有 `tracing` 日志，维度为 provider-neutral（service_id/command/trace_id/reason_code）。
- 全链路可 trace、可 audit、可 replay；不得有审计盲区。
- 对外 HTTP/SSE 契约、manifest 格式、session 隔离语义保持不变。

## Goals / Non-Goals

Goals：
- 唯一协议调用路径：所有应用类型、所有能力调用收敛到 `service.call`。
- 纯净微内核：内核仅持有系统不变量；非内核能力全部驱逐为 service/optional module。
- 历史债务清零：删除 compat/legacy/协调补丁；依赖 gate allowlist 归零。
- Web/CLI 还原为 thin shell；巨型文件拆分达标（≤500 行）。
- 终态固化进 OpenSpec baseline。

Non-Goals：
- 不重写已正确的协议骨架（ServiceRuntime/ServiceRouter/ServiceCallExecutor/SystemService/proto DTO）。
- 不改对外 HTTP/SSE 契约、不改前端、不改 manifest 格式。
- 不新增业务域能力；domain pack（finance/crypto）仅做"移出 base"而非增强。
- 不引入新外部依赖（除非把内核能力外置为 optional module crate，且该 crate 不得成为 base OS 必需依赖）。

## Decisions

### D1. 唯一执行路径模型（消灭 P-KERNEL / P-WEBEXEC / P-DELEGATE / P-TOOLKIT）
- 决策：所有 "运行一个 agent" 的语义统一由 `service.agent_execution`（runtime-host provider）拥有；所有 "调用一个能力"（llm/tool/driver/skill/mcp/task/memory/context/payment/web3/evm）统一经 `ServiceRouter.route → ServiceRuntime.call → ServiceBus → SystemServiceBusHandler → ServiceCallExecutor → SystemService`。
- 内核仅保留 `AgentExecutionPort`（typed 抽象端口），其唯一生产实现是"service-client execution adapter"（调用 `service.agent_execution`），删除 `LegacyAgentExecutionAdapter`(provider 直连)。
- Web 仅保留 SSE channel + HTTP DTO 适配；agent 执行的模型/工具/loop 实现迁入 runtime-host 的 Agent Execution Service provider。
- 工具可见性（driver/MCP/skill）统一经 service snapshot 命令获取，删除 `framework_toolkit` 直读 runtime。
- 模式：Facade（SystemFacade/SDK client）+ Command（ServiceCommand）+ Chain of Responsibility（middleware）+ Decorator（runtime decorator 链）+ Strategy（execution adapter / provider）。
- 替代方案：保留 web/kernel 各自执行入口并继续用标记协调 —— 拒绝，因为这正是审计判定的债务根源。

### D2. 删除多路径协调补丁
- 决策：单路径达成后，删除 `application_execution_hosted.rs` 的 `authoritative/non_authoritative/legacy_unmarked` 区分（约 546–695 行）、`host_import_bridge.rs` 的 `graph_owner`/`execution.graph_owner` 区分用途、`agent_execution_backend.rs` 的 `suppress_executor_lifecycle` 与 `legacy_chat_main_thread_goal_pause`。
- 删除门控：仅当 audit replay 证明某能力只剩单一 service.call 链后，才删除对应补丁（避免"删了没替身"）。
- 终态：所有 task 天然 authoritative；终态判定回归确定性简单逻辑。

### D3. 内核纯净化（驱逐非内核能力）
- 决策：将下列模块移出 `macaca-kernel`：
  - `web3.rs/web3_event.rs/web3_tests.rs` → optional module（`macaca-web3` 或现有 web3 service provider，runtime-host 装配）。
  - `evm.rs/evm_adapter.rs/evm_event.rs/evm_tests.rs` → optional EVM module（参考 `optional-evm-substrate-frontier-adapter-boundary.md`）。
  - `a2a.rs/a2a_event.rs/payment_policy.rs` → payment service（runtime-host `payment_service_provider.rs`/`payment_adapter.rs` 已存在）。
  - `executor/`（`ApplicationExecutor/ForkManager/AgentRunner/TaskRouter/WorkerSupervisor/CallbackDispatcher/ExecutionQueue/EventBus`）→ task/execution service（runtime-host/service 层）。
  - `provider_compat.rs`（`KernelProviderCompat/LegacyLlmProvider/LegacyToolCatalog`）→ 删除。
  - `kernel_builder.rs` 的 `KernelServiceClientCompat` compat 构造、`Kernel::new(llm,tools)`、`scheduler.rs`/`persistence.rs` 的 deprecated 项 → 删除。
- 内核保留：identity/registry（agent/service/capability）、service-call facade（`service_call.rs`/`service_bus_bridge.rs`/`facade.rs`）、policy facade、trace/audit bus、scheduler primitive、resource manager、session/task 状态契约、package runtime guard、`AgentExecutionPort` 抽象。
- 模式：Adapter/Bridge（IPC↔service）、Abstract Factory（provider 工厂只在 composition root）、State（生命周期契约）。
- 替代方案：保留 kernel 内 facade 但移除 adapter —— 拒绝，宪法明确 Web3/EVM/Payment/A2A 整类为非内核，不留半截。

### D4. 解除越界依赖
- 决策（终态依赖）：
  - `macaca-kernel` → 仅 `macaca-proto`、`macaca-ipc`。
  - `macaca-persist` → 不依赖 `macaca-context`（反转：context 依赖 persist，或抽取共享 proto 契约）。
  - `macaca-web`/`macaca-cli` → 仅 `macaca-sdk`（+ proto DTO）。
- 顺序：先把消费者迁到 service client / facade，`cargo metadata` 证明直接依赖边消失，再删 `Cargo.toml` 依赖与 allowlist 行，并同步 `macaca-os-serviceization-allowlist.md`。

### D5. Web/CLI thin shell + 文件拆分
- 决策：删除 `AppState` direct provider 字段（runtime/llm/router/memory/mcp/driver registry）；`framework_toolkit` 改 service snapshot；session loop 下沉到 `service.execution_control` + task service。
- 文件拆分（按 ownership，非格式化）：`loop_manager.rs`(2629)/`framework_runner.rs`(2484)/`chat_orchestrator.rs`(1581)/`lib.rs`(975) → 每文件 ≤500 行。
- CLI：server-start seam 移到小型 public bootstrap facade，删除 `macaca-cli → macaca-web` internals 依赖。

### D6. domain pack 外置
- 决策：`domain_pack_service_provider.rs`/`finance_live_data.rs` 移出 base runtime-host，注册为 plugin/package service provider（带 descriptor + policy metadata）；runtime-host 仅留 generic `ServiceProviderFactory` 与注册机制。缺席返回结构化 unavailable。

### D7. 逃逸口从"冻结"升级为"删除"
- 决策：扩展 `serviceization-escape-hatches` 与 `route_c_dependency_boundaries`，把"阻止新增"升级为"存量清零"：当某逃逸口对应 service client 全量替换后，删除 migration module 豁免，使任何引用（含旧引用）都 CI 失败。

### D8. 强制门（terminal gate）
- 决策：新增/强化可执行边界门：
  1. allowlist 行数 == 0（terminal assertion）。
  2. no-direct-provider-call：每个已服务化能力，生产代码只能经 service client；旁路引用 → 失败。
  3. no-hardcoded-name：生产代码无硬编码 agent/app/provider/model/driver/gateway/chain/payment 名（fixtures/tests 除外）。
  4. shell-not-semantic-owner：shell 不得直驱 task/loop、不得引用 deprecated direct fields、不得拥有 agent 执行实现。
  5. kernel-purity：kernel 仅依赖 proto/ipc。
  6. file-size：OS 层无 >500 行源文件。

## 影响备忘录（GitNexus，非阻塞）

按规则编辑高扇出符号前应记录 impact，本次仅备忘不阻塞（用户指令）：
- `macaca-kernel::executor::*`、`Kernel::execute_agent`、`AgentExecutionPort`、`ServiceRuntime`、`ServiceRouter`、`AppState` 字段、`application_execution_hosted::*` 预计为 HIGH/CRITICAL。
- 每个高风险 task 在 `tasks.md` 标注 `[impact-memo]`，执行时运行 `gitnexus_impact` 并把 blast radius 记录到该 task 旁注，但不因告警停工。

## Risks / Trade-offs

- R1 删除 kernel 能力造成编译/运行回归 → 缓解：先建替身（optional module/service）并通过 absent 降级测试，再删 kernel 实现；分阶段独立 OpenSpec change 可单独 revert。
- R2 单路径收敛改变事件/终态时序 → 缓解：以 audit replay + route-c 回归矩阵为门；补丁删除以 replay 单链为前置条件。
- R3 巨型文件拆分引入隐性行为变化 → 缓解：拆分为纯结构移动 + 单测覆盖，先 `cargo test` 绿再拆下一处。
- R4 优化为单路径后第三方 WASM/YAML 应用兼容性 → 缓解：Application ABI 不变，host import 契约不变，仅后端实现收敛。

## Migration Plan

阶段 P0→P5（见 `tasks.md`）。每阶段：brainstorm（如需）→ 本 change 内细化 task → 改代码 → `cargo check`/targeted test/boundary gate/audit replay → 删除对应 allowlist/逃逸口豁免 → 同步治理文档。回退：每阶段独立提交，删实现前替身先就位。

## Open Questions — Resolved (2026-06-07)

- **Q1 Web3/EVM optional module**：复用现有 runtime-host `*_service_provider` 与 optional module 注册机制；不新建平行 facade。P2 从 kernel 删除 `web3`/`evm`/`a2a`/`payment_policy` 实现后，缺席路径返回结构化 unavailable（见 `baseline.md`）。
- **Q2 persist→context 反转**：抽取持久化 DTO/契约到 `macaca-proto`（或 context 依赖 persist 端口 trait），移除 `macaca-persist/Cargo.toml` 对 `macaca-context` 的 direct edge。P2.7 执行。
- **Q3 Fork-Join / executor 迁出**：`service.execution_control` 拥有 pause/resume/checkpoint 与 resume 诊断；task service 发射 `graph_owner` 审计字段；`delegate_task`/`loop_manager` 消费 execution-control 事件而非 kernel `executor/`。P1.3 + P3.2 执行。
