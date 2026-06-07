# Tasks — 统一调用路径 + 协议化微内核 + 债务清零

> 执行约定（每个 task 默认遵守，不再重复）：
> - 改任何符号前先 `gitnexus_impact`（标 `[impact-memo]` 的 task 必须记录 blast radius 到旁注，但 GitNexus HIGH/CRITICAL 告警**不阻塞**，仅备忘）。
> - 新代码必须有详尽英文注释（功能 + 运行原理 + 权衡）。
> - 关键执行节点加 `tracing`（provider-neutral 维度：service_id/command/trace_id/reason_code）。
> - 每个改动后 `cd macaca && cargo check`；每个能力改动后跑对应 `cargo test -p macaca-<crate>`。
> - 禁止 application 专有/硬编码业务名（fixtures/tests 除外）。
> - 删除 legacy 前，替身（service/optional module/client）必须先就位并通过 absent 降级测试。
> - 验证命令集（VC）见末尾 §13。

## 0. 前置与基线锚定

- [x] 0.1 运行并记录基线：`openspec list`、`openspec spec list --long`、`cargo test -p macaca-integration-tests route_c_dependency_boundaries_reject_unallowlisted_forbidden_edges -- --nocapture`（记录当前 100 production edge / 10 allowlist）。
- [x] 0.2 运行并记录 `cargo tree -e normal -p macaca-kernel --depth 1`、`-p macaca-web`、`-p macaca-cli`、`-p macaca-persist` 作为依赖基线快照（写入本 change 的 `baseline.md` 备忘）。
- [x] 0.3 建立"单路径 audit replay 基线"：对 `/api/chat/v2` 一次 YAML 会话与一次 WASM 会话各跑一次，导出 service-call audit replay（by session_id），记录当前出现的执行链数量（预期 >1，作为收敛前对照）。（见 `audit-replay-baseline.md`：YAML 3 链、WASM 3 链，静态盘点 + replay 命令面）
- [x] 0.4 在 `design.md` Open Questions Q1/Q2/Q3 上做最终决策并记录（web3/evm 复用现有 provider；persist→context 反转方式；execution-control 承接 Fork-Join 契约）。

## 1. P0 — 冻结逃逸口（阻止债务增长）

### 1.1 扩展静态逃逸口门覆盖全部旁路
- [x] 1.1.1 在 `macaca/crates/tests/macaca-integration-tests/tests/serviceization_escape_hatches.rs` 增加规则：生产代码（migration module 外）禁止新增对 `KernelProviderCompat`、`LegacyLlmProvider`、`LegacyToolCatalog`、`LegacyAgentExecutionAdapter` 的引用。失败诊断含 file:line:token + 替换路径。
- [x] 1.1.2 增加规则：生产代码禁止新增对 `AppState` deprecated 字段（`runtime`、`registry`、`llm`/`router`、`memory_runtime`、`mcp_runtime`、`driver_runtime`/`driver_registry`）的读取（已部分存在，补全字段清单）。
- [x] 1.1.3 增加规则：生产代码禁止新增 `AppRuntime::start_app` / `start_app_from_file` 调用（application service provider bridge 除外）。
- [x] 1.1.4 增加规则：生产代码禁止新增直读 `*.collect_tools()`（driver runtime）与 `*.definitions()`（mcp runtime）。
- [x] 1.1.5 增加规则：生产 OS 层禁止新增硬编码 `coordinator/planner/worker/backend/frontend/architect`（manifest 解释/ fixtures/tests 除外）。
- [x] 1.1.6 增加规则：生产代码禁止新增对 kernel `web3::*`、`evm::*`、`a2a::*`、`payment_policy::*` 的引用（为 P2 驱逐做准备）。
- [x] 1.1.7 增加规则：生产代码禁止新增 `graph_owner`/`execution.graph_owner`/`authoritative`/`suppress_executor_lifecycle`/`legacy_chat_main_thread_goal_pause` 新增写入点（冻结协调补丁规模）。

### 1.2 allowlist 元数据补全
- [x] 1.2.1 为 `tests/route_c_dependency_boundaries/allowlist.rs` 每行补全 owner track / current caller path / replacement service client / target phase / expiry condition / validation command（对照宪法要求字段）。
- [x] 1.2.2 同步更新 `macaca/docs/macaca-os-serviceization-allowlist.md`，使 markdown memento 与 Rust 执行输入一致（gate 诊断要求双面一致）。

### 1.3 P0 退出验证
- [x] 1.3.1 跑 VC-escape + VC-gate 全绿；故意新增一处违规验证 CI 失败（红/绿对照后回退）。

## 2. P1 — 统一 Agent 执行为单一 Service（最关键）

### 2.1 收紧 AgentExecutionPort 契约（kernel 抽象，禁止接 provider）
- [x] 2.1.1 `[impact-memo]` 审查 `macaca-agent` 的 `AgentExecutionPort` 定义，确认其方法仅以 typed command/handle 表达，无 `LlmProvider`/`ToolCatalog` 参数。
- [x] 2.1.2 实现 `ServiceClientAgentExecutionAdapter`（生产唯一实现）：调用 `service.agent_execution`（经 SystemFacade/SDK client），返回结构化 `AgentExecutionResult`/unavailable；详尽英文注释 + 起止/失败/unavailable 日志。
- [x] 2.1.3 为 `ServiceClientAgentExecutionAdapter` 写单测：service 可用→成功；service 缺席→结构化 unavailable（不伪造成功）。
- [x] 2.1.4 实现 `SwappableAgentExecutionPort` + `Kernel::replace_execution_port`；web 在 `service.agent_execution` 启动后调用 `wire_kernel_to_agent_execution_service`（3 个 runtime-host 单测含 hot-swap）。

### 2.2 Agent 执行实现迁出 web shell → runtime-host service provider
- [x] 2.2.1 `[impact-memo]` 盘点 `macaca/crates/shells/macaca-web/src/agent_execution_backend.rs` 中"模型/工具/loop"执行逻辑（`build_runtime_agent_*` + `agent.reply()` 段，约 836–893 行）。（inventory: 语义层已提取至 `agent_execution_orchestration.rs`；shell 仍保留 `agent.reply()`/FrameworkRunner/SSE/executor 生命周期）
- [x] 2.2.2 在 `macaca-runtime-host` 的 Agent Execution Service provider（`agent_execution_service_provider.rs`）内承接该执行逻辑：context 经 `service.agent_context`、控制经 `service.execution_control`、模型/工具经 service.call，统一产出 service-call evidence。（`ComposedAgentExecutionBackend` + `agent_execution_orchestration`）
- [x] 2.2.3 把 `FrameworkRunner` agent 构造（runtime/framework 关注点）下沉到 `macaca-runtime`/`macaca-framework` 层 service 后面，web 不再直接构造 framework Agent。（`ServiceBackedFrameworkRuntimeAgentPort` + `FrameworkAgentConstructionPort`；web 仅 `WebFrameworkAgentConstructionPort` 适配器；`FrameworkRunner` 本体待 4.3.2 迁出）
- [x] 2.2.4 web 侧 `WebAgentExecutionBackend` 收缩为：仅做 SSE channel 注入 + HTTP/DTO 适配 + 调 `service.agent_execution`；删除 shell 内执行语义。（web 改为 `web_agent_execution_adapters` 端口注入；执行 Template Method 在 runtime-host）
- [x] 2.2.5 为迁移后的 Agent Execution Service 写集成测试：YAML 会话与 WASM 会话经**同一** provider 执行；trace/audit 链一致。（`unified_agent_execution_provider_tests.rs` web 7/7 + runtime-host 3/3）

### 2.3 执行编排（Fork-Join / worker-loop）迁出 kernel → service
- [x] 2.3.1 `[impact-memo]` 盘点 `macaca-kernel/src/executor/`（`ApplicationExecutor/ForkManager/AgentRunner/TaskRouter/WorkerSupervisor/CallbackDispatcher/ExecutionQueue/EventBus/event_factory/router/bus/queue/worker/app_executor`）所有对外消费者。（见 `executor-consumer-inventory.md`，47 行，live 耦合集中在 macaca-web）
- [x] 2.3.2 在 task/execution service（runtime-host 或 `macaca-task`）建立 Fork-Join 暂停/恢复契约，由 `service.execution_control` 承接（对应 design Q3）。（`ExecutionControlForkJoinCoordinator` + web `orchestration_tools`/`hook_consumer`/`fork_join_shell_adapter` 接入；kernel `ForkManager` 状态待 P3 驱逐）
- [x] 2.3.3 把 `delegate_task` 工具（`macaca-tools/src/orchestration.rs` + web `orchestration_tools.rs`）的执行落点从 kernel executor 改为 `service.agent_execution` / `service.task`。（`ServiceDelegatedTaskDispatcher` + `begin/complete_service_backed_delegation`；fork 生命周期仍经 executor `ForkManager`，待 2.3.2 execution_control 承接）
- [x] 2.3.4 web `loop_manager.rs` 的 session loop 拉取/唤醒逻辑改为消费 `service.execution_control` + task service 事件（不直驱 kernel executor）。（`ExecutionControlGoalLifecycleCoordinator` + `goal_lifecycle_shell_adapter`；PlanLoop `GoalCompleted` + `create_goal` 注册/恢复经 execution_control；worker loop 已走 `service.agent_execution`）
- [x] 2.3.5 集成测试：协调者 `delegate_task`（Fork-Join）与目标-任务（create_goal→worker）两条委派路径均经统一 service 路径，暂停/恢复语义不回归。（`unified_delegation_path_tests.rs` 契约测试 5/5）

### 2.4 YAML 路径并轨
- [x] 2.4.1 `[impact-memo]` 盘点 `macaca-app/src/workflow.rs` + web `agent_runner.rs` 的 workflow 执行落点。（见 `workflow-execution-inventory.md`）
- [x] 2.4.2 workflow 步骤执行改为统一调用 Application Service → `service.agent_execution`，不再走 kernel executor 直驱。（`agent_runner` → `application.agent.delegate` → `application_agent_delegate_bridge`）
- [x] 2.4.3 集成测试：YAML workflow 与 WASM app 经同一 Application ABI → service.call 路径（audit replay 单链）。（`unified_workflow_application_abi_tests.rs` 6/6 + 更新 `unified_agent_execution_provider_tests`）

### 2.5 工具可见性统一经 service（消灭 P-TOOLKIT）
- [x] 2.5.1 `[impact-memo]` 盘点 `macaca-web/src/framework_toolkit.rs` 对 `driver_runtime.collect_tools()`、`mcp_runtime.definitions()` 的直读点。（inventory: toolkit 已走 `driver_client`/`skill_client`/`mcp_client`；剩余 `mcp_runtime` 仅 bootstrap 注入 + dead `skill_mcp`）
- [x] 2.5.2 改为经 `SystemDriverClient`/`SystemSkillClient`/`SystemMcpClient` 的 snapshot 命令获取工具目录；driver/MCP 不可用时返回结构化 unavailable + session-visible 诊断，**删除** deprecated runtime fallback。（`framework_runner` probe + `chat_orchestrator` cleanup 已改 service client）
- [x] 2.5.3 单测：driver catalog/MCP definitions 不可用时不触发旧 runtime fallback。（`production_toolkit_assembly_does_not_register_direct_mcp_clients` + `probe_mcp_capability_inputs_via_client_maps_service_status_views`）

### 2.6 删除多路径协调补丁（前置：audit replay 已显示单链）
- [x] 2.6.1 删除 `runtime-host/application_execution_hosted.rs` 的 `authoritative/non_authoritative/legacy_unmarked` 区分逻辑（约 546–695 行），终态判定回归"所有 host command 同等 authoritative"。
- [x] 2.6.2 删除 `wasm_runtime_provider/host_import_bridge.rs` 中 `graph_owner`/`execution.graph_owner` 的"区分真实/兼容"用途（保留纯审计标签如确有审计价值，否则删除）。
- [x] 2.6.3 删除 `agent_execution_backend.rs` 的 `should_emit_executor_lifecycle`/`suppress_executor_lifecycle`（单一发事件方后无需去重）。
- [x] 2.6.4 删除 `agent_execution_backend.rs` 的 `legacy_execution_control_policy` / `legacy_chat_main_thread_goal_pause`，改由 manifest projection 提供 execution-control policy。
- [x] 2.6.5 删除 `application_execution_hosted.rs` 中针对 `queued|pending` 与 legacy 状态的兼容归一化分支中"为兼容路径而存在"的部分（保留协议必须的状态语义）。
- [x] 2.6.6 全仓搜索确认 `legacy_unmarked`/`non_authoritative`/`suppress_executor_lifecycle`/`legacy_chat_main_thread_goal_pause` 在生产代码中为 0 命中。

### 2.7 P1 退出验证
- [x] 2.7.1 audit replay：YAML 与 WASM 会话各自只出现**一条** service.call 执行链（对照 0.3 基线，由 >1 收敛为 1）。
- [x] 2.7.2 route-c 回归矩阵（`macaca/docs/route-c-regression-matrix.md`）全绿；`/api/chat/v2` 创建/恢复不回归；fullstack-autodev 集成测试绿。
- [x] 2.7.3 VC-escape：1.1.7 冻结的协调补丁标记在生产代码清零。

## 3. P2 — 内核纯净化（驱逐非内核能力 + 解除越界依赖）

### 3.1 驱逐 Payment / A2A
- [x] 3.1.1 `[impact-memo]` 盘点 `kernel/a2a.rs`、`a2a_event.rs`、`payment_policy.rs` 的消费者；确认 runtime-host `payment_service_provider.rs`/`payment_adapter.rs`/`payment_admission.rs` 可承接全部能力。（`payment_policy` 实现已迁至 `macaca-proto`；kernel 仅 compat re-export）
- [ ] 3.1.2 将 payment policy / A2A coordinator 能力迁入 payment service（provider 层），保留结构化 unavailable（service 缺席时）。
- [ ] 3.1.3 删除 kernel `a2a.rs`、`a2a_event.rs`、`payment_policy.rs` 及 `lib.rs` 中相关 `#[allow(deprecated)]` 导出。
- [ ] 3.1.4 测试：payment/A2A 经 service.call 路径；payment service 缺席返回结构化 denied/unavailable，不崩溃。

### 3.2 驱逐 Web3
- [ ] 3.2.1 `[impact-memo]` 盘点 `kernel/web3.rs`、`web3_event.rs`、`web3_tests.rs` 消费者；确认现有 web3 service provider（runtime-host）可承接。
- [ ] 3.2.2 将 Web3 facade/adapter 迁入 web3 optional module/service provider；缺席返回结构化 unavailable。
- [ ] 3.2.3 删除 kernel `web3.rs`、`web3_event.rs`、`web3_tests.rs` 及 `lib.rs` 导出。
- [ ] 3.2.4 测试：web3 optional 缺席时 base OS 正常启动/执行/恢复/查询审计。

### 3.3 驱逐 EVM
- [ ] 3.3.1 `[impact-memo]` 盘点 `kernel/evm.rs`、`evm_adapter.rs`、`evm_event.rs`、`evm_tests.rs` 消费者；确认 `runtime-host/evm_service_provider.rs` 可承接（参考 `optional-evm-substrate-frontier-adapter-boundary.md`）。
- [ ] 3.3.2 将 EVM facade/adapter（含 `Mock*`/`Unavailable*`）迁入 EVM optional module/service provider。
- [ ] 3.3.3 删除 kernel `evm.rs`、`evm_adapter.rs`、`evm_event.rs`、`evm_tests.rs` 及 `lib.rs` 导出。
- [ ] 3.3.4 测试：EVM optional 缺席降级；无 base OS 反向依赖。

### 3.4 驱逐执行编排 executor（P1 已迁逻辑，此处删 kernel 模块）
- [ ] 3.4.1 确认 P1 已将 `executor/` 逻辑迁至 service；全仓搜索 kernel `executor::` 的残留生产消费者为 0。
- [ ] 3.4.2 删除 `kernel/src/executor/` 整目录及 `lib.rs` 中 executor 导出（`ApplicationExecutor/ForkManager/AgentRunner/TaskRouter/...`）。
- [ ] 3.4.3 测试：kernel 单测不再依赖 executor；task/execution service 测试覆盖原 executor 行为。

### 3.5 删除 provider 兼容与 compat 构造
- [ ] 3.5.1 删除 `kernel/provider_compat.rs`（`KernelProviderCompat/LegacyLlmProvider/LegacyToolCatalog`）及 `lib.rs` 导出。
- [ ] 3.5.2 删除 `kernel/kernel.rs` 的 `Kernel::new(config, llm, tools)` deprecated 构造；唯一构造路径经 `KernelBuilder` + `AgentExecutionPort`（service-client adapter）。
- [ ] 3.5.3 删除 `kernel_builder.rs` 的 `KernelServiceClientCompat` 等 compat 构造路径。
- [ ] 3.5.4 删除 `kernel/scheduler.rs` 的 `#[allow(deprecated)]` 项与 `persistence.rs` 的 deprecated payment store。
- [ ] 3.5.5 删除 `runtime-host/compat.rs`（44 处 compat）中已无消费者的兼容代码；剩余必需项迁正规 module 并去 compat 命名。
- [ ] 3.5.6 测试：kernel 不再有 `#[allow(deprecated)]` 导出；`cargo check` 无 deprecated 警告（kernel 范围）。

### 3.6 解除越界依赖 + 删 allowlist 行
- [ ] 3.6.1 移除 `kernel/Cargo.toml` 对 `macaca-driver` 依赖（前置：web3/evm/a2a 已驱逐使其无消费者）；`cargo metadata` 确认边消失 → 删 allowlist `kernel→driver` 行 + 同步 doc。
- [ ] 3.6.2 移除 `kernel/Cargo.toml` 对 `macaca-gateway` 依赖；删 allowlist `kernel→gateway` 行 + doc。
- [ ] 3.6.3 移除 `kernel/Cargo.toml` 对 `macaca-skill` 依赖；删 allowlist `kernel→skill` 行 + doc。
- [ ] 3.6.4 核查并移除 `kernel/Cargo.toml` 对 `macaca-task`、`macaca-tools` 依赖（若仍存在）；确认 `cargo metadata` 边消失。
- [ ] 3.6.5 核查并移除 `kernel/Cargo.toml` 对 `macaca-agent`、`macaca-sdk` 依赖（agent 仅经 `AgentExecutionPort` 契约；sdk 不应被 kernel 依赖）；如需 `AgentExecutionPort`，下沉契约到 `macaca-proto`。
- [ ] 3.6.6 终态断言：`cargo tree -e normal -p macaca-kernel --depth 1` 仅显示 `macaca-proto`、`macaca-ipc`。

### 3.7 persist → context 越界反转
- [ ] 3.7.1 `[impact-memo]` 查明 `macaca-persist` 依赖 `macaca-context` 的具体类型/用途。
- [ ] 3.7.2 抽取 context 持久化所需契约到 `macaca-proto`（或反转为 context 依赖 persist），移除 `persist/Cargo.toml` 对 `macaca-context` 的依赖。
- [ ] 3.7.3 `cargo metadata` 确认 `macaca-persist → macaca-context` 边消失；persist/context 单测绿。

### 3.8 P2 退出验证
- [ ] 3.8.1 kernel `lib.rs` 不再 `pub mod web3/evm/a2a/payment_policy/provider_compat/executor`；kernel 仅含系统不变量模块。
- [ ] 3.8.2 与 kernel/persist 相关的 allowlist 行清零；VC-gate 全绿。
- [ ] 3.8.3 optional module（web3/evm/payment）absent 降级测试全绿。

## 4. P3 — Web 瘦身为 thin shell + 文件拆分

### 4.1 删除 AppState direct provider 字段
- [ ] 4.1.1 `[impact-memo]` 盘点 `macaca-web/src/state.rs` 全部 deprecated 字段（`runtime`、`registry`、`llm`/`router`、`memory_runtime`、`mcp_runtime`、`driver_runtime`/`driver_registry`）的读取点。
- [ ] 4.1.2 逐字段替换为 focused SDK client（`SystemFacade` + clients）；每替换一个字段跑相关路由/单测。
- [ ] 4.1.3 删除 `AppState` 中全部 direct provider 字段；引入小型 shell composition bundle（仅持 SDK clients + SSE/HTTP 适配状态）。
- [ ] 4.1.4 删除 `macaca-web/src/memory_runtime.rs` 等 shell 内 runtime 持有者（若无消费者）。

### 4.2 session loop 下沉
- [ ] 4.2.1 `loop_manager.rs` 的 plan/worker loop 拉取/唤醒/心跳逻辑迁至 `service.execution_control` + task service；web 仅保留 SSE endpoint 与事件订阅。
- [ ] 4.2.2 `hook_consumer.rs`/`chat_orchestrator.rs` 中 Fork-Join 暂停/恢复编排改为消费 execution-control 事件。

### 4.3 巨型文件拆分（≤500 行，按 ownership）
- [ ] 4.3.1 拆分 `loop_manager.rs`(2629) → execution-control adapter / task-event adapter / SSE channel adapter / 薄 orchestrator（每文件 ≤500 行）。
- [ ] 4.3.2 拆分 `framework_runner.rs`(2484) → runtime agent 构造 service（迁 runtime/framework 层）+ 薄 web 适配。
- [ ] 4.3.3 拆分 `chat_orchestrator.rs`(1581) → route adapter / application-service 调用适配 / DTO 映射。
- [ ] 4.3.4 拆分 `macaca-web/src/lib.rs`(975) → 小型 composition bundle + route 注册。
- [ ] 4.3.5 每次拆分仅做结构移动 + 注释补全，拆分前后 `cargo test -p macaca-web` 绿。

### 4.4 删除 web 越界依赖 + allowlist 行
- [ ] 4.4.1 替换完成后逐条移除 `macaca-web/Cargo.toml` 对 `macaca-driver/llm/memory/persist/skill/task/tools` 的直接依赖。
- [ ] 4.4.2 每删一条 `cargo metadata` 确认边消失 → 删对应 7 条 web allowlist 行 + 同步 doc。
- [ ] 4.4.3 核查并移除 `macaca-web` 对 `macaca-kernel`、`macaca-runtime-host` 的直接依赖（改经 `macaca-sdk`/facade；如必要保留 runtime-host 仅限 composition root 启动 seam，移至 4.x bootstrap facade）。
- [ ] 4.4.4 终态断言：`cargo tree -e normal -p macaca-web --depth 1` 仅 `macaca-sdk`（+ proto DTO/必要 framework 适配）。

### 4.5 P3 退出验证
- [ ] 4.5.1 全仓无 >500 行 OS 层源文件（VC-filesize）。
- [ ] 4.5.2 web 相关 allowlist 清零；VC-gate 绿；`/api/chat/v2`、SSE、GenUI 渲染回归绿。

## 5. P4 — CLI 解耦 + domain pack 外置

### 5.1 CLI 解耦
- [ ] 5.1.1 `[impact-memo]` 盘点 `macaca-cli/src/commands.rs`/`command_handlers.rs` 对 `Kernel`/`KernelBuilder`/`GatewayBuilder`/`LlmProvider`/`macaca_web::WebServerBuilder` 的使用。
- [ ] 5.1.2 CLI run/status 改用 runtime-host bootstrap client 或 SDK status/service inspector，删除 kernel 直接构造。
- [ ] 5.1.3 `macaca web` 进程启动 seam 移到小型 public bootstrap facade（binary-only entrypoint），删除 `macaca-cli → macaca-web` internals。
- [ ] 5.1.4 删除 `macaca-cli/Cargo.toml` 对 `macaca-gateway/tools/web` 的直接依赖 + 同步 allowlist/doc。
- [ ] 5.1.5 CLI 命令冒烟测试（status/run/web）通过。

### 5.2 domain pack 外置
- [ ] 5.2.1 `[impact-memo]` 盘点 `runtime-host/domain_pack_service_provider.rs`、`finance_live_data.rs`（`service.market_data/financials/news_digest/llm.analysis`、Coindesk/Binance/OKX、`asset_class=="crypto"` 分支）。
- [ ] 5.2.2 将 finance/crypto domain-pack 实现移出 base runtime-host，注册为 plugin/package service provider（带 descriptor + policy metadata，经 manifest 声明）。
- [ ] 5.2.3 runtime-host 仅保留 generic `ServiceProviderFactory` + 注册机制；deterministic fixtures 仅保留在测试。
- [ ] 5.2.4 测试：domain pack 缺席返回结构化 unavailable；base runtime-host 无业务域字符串（VC-hardcoded）。

### 5.3 P4 退出验证
- [ ] 5.3.1 `cargo tree -p macaca-cli --depth 1` 仅 `macaca-sdk`（+ proto）；CLI allowlist 清零。
- [ ] 5.3.2 runtime-host 无 finance/crypto/exchange 业务域代码（VC-hardcoded 扫描）。

## 6. P5 — 强制门清零 + OpenSpec baseline 对齐

### 6.1 终态门
- [ ] 6.1.1 在 `route_c_dependency_boundaries` 增加 terminal 断言：allowlist 行数 == 0（任何残留即失败）。
- [ ] 6.1.2 实现 `no-direct-provider-call` 审计：每个已服务化能力（llm/memory/context/task/driver/skill/mcp/agent_execution/payment/web3/evm/gateway），生产代码只能经 service client；旁路引用 → 失败。
- [ ] 6.1.3 实现 `no-hardcoded-name` 审计：生产代码无硬编码 agent/app/provider/model/driver/gateway/chain/payment 名（fixtures/tests 除外）。
- [ ] 6.1.4 实现 `shell-not-semantic-owner` 审计：shell 不得直驱 task/loop、不引用 deprecated direct fields、不持 agent 执行实现。
- [ ] 6.1.5 实现 `kernel-purity` 审计：kernel 仅依赖 proto/ipc（与 3.6.6 联动）。
- [ ] 6.1.6 实现 `file-size` 审计：OS 层无 >500 行源文件。

### 6.2 逃逸口由"冻结"升级为"删除"
- [ ] 6.2.1 每个逃逸口对应 service client 全量替换后，删除其 migration module 豁免，使任何引用（含旧引用）CI 失败。
- [ ] 6.2.2 `serviceization_escape_hatches.rs` 切换为"存量清零"断言：扫描结果命中数为 0（除显式 fixtures/tests）。

### 6.3 OpenSpec baseline 对齐
- [ ] 6.3.1 将本 change 落地后的终态固化进 `openspec/specs/`：`unified-execution-path`、`microkernel-boundary-purity`、更新 `serviceization-dependency-gate`/`serviceization-escape-hatches`/`web-cli-thin-shell-completion`。
- [ ] 6.3.2 分批 archive 相关 completed changes，使 baseline 反映终态。
- [ ] 6.3.3 `openspec validate --strict` 全绿。

## 7. 横切 — 注释 / 日志 / trace-audit

- [ ] 7.1 所有新增/迁移模块补详尽英文注释（功能 + 运行原理 + 权衡）；巨型文件拆分后的新文件逐一覆盖。
- [ ] 7.2 关键节点日志统一为 provider-neutral 维度；删除以 provider/model/app name 为主键的日志（如 `provider_compat.rs` 风格）。
- [ ] 7.3 全局化脱敏（`sanitize_json`/`is_safe_metadata_key`）到所有 audit/trace/snapshot 出口；新增脱敏单测。
- [ ] 7.4 旁路删除后验证"无审计盲区"：所有执行都产出 service-call evidence，可按 trace_id/session_id replay。

## 8. 治理文档同步

- [ ] 8.1 每删一条 allowlist 同步 `macaca/docs/macaca-os-serviceization-allowlist.md`。
- [ ] 8.2 终态后更新审计三件套（`2026-06-07-*`）的"已达成"状态注记，作为归档证据。

## 9. 终态验收（Definition of Done，逐条证明）

- [ ] 9.1 单路径：YAML 与 WASM 应用 agent 执行 audit replay 均为单一 service.call 链（对照 0.3）。
- [ ] 9.2 协调补丁清零：`graph_owner/authoritative/legacy_unmarked/suppress_executor_lifecycle/legacy_*` 生产代码 0 命中。
- [ ] 9.3 内核纯净：kernel 无 web3/evm/a2a/payment/executor/provider_compat；`cargo tree -p macaca-kernel` 仅 proto/ipc。
- [ ] 9.4 越界依赖清零：persist 不依赖 context；web/cli 仅依赖 sdk。
- [ ] 9.5 allowlist == 0；全部终态门绿。
- [ ] 9.6 无 >500 行 OS 源文件；domain pack 出 base runtime-host。
- [ ] 9.7 对外契约不回归：`/api/chat/v2`、SSE、manifest、session 隔离。
- [ ] 9.8 OpenSpec baseline 反映终态；`openspec validate --strict` 绿。

## 10. 备注 — GitNexus 影响（非阻塞）

- [ ] 10.1 在本 change 维护一份 `impact-memo.md`，记录各 `[impact-memo]` task 的 blast radius / risk level，仅备忘不阻塞。

## 13. 验证命令集（VC）

```bash
cd macaca
# VC-check    编译
cargo check
# VC-gate     依赖边界门（含 allowlist=0 终态）
cargo test -p macaca-integration-tests route_c_dependency_boundaries_reject_unallowlisted_forbidden_edges -- --nocapture
# VC-escape   逃逸口门（终态：存量清零）
cargo test -p macaca-integration-tests serviceization_escape_hatches -- --nocapture
# VC-tree     依赖快照
cargo tree -e normal -p macaca-kernel --depth 1
cargo tree -e normal -p macaca-web --depth 1
cargo tree -e normal -p macaca-cli --depth 1
cargo metadata --no-deps --format-version 1
# VC-svc      受影响能力 service 测试
cargo test -p macaca-task && cargo test -p macaca-runtime-host && cargo test -p macaca-kernel
# VC-e2e      端到端：/api/chat/v2（YAML + WASM）、fullstack-autodev、route-c 回归矩阵
cargo test -p macaca-integration-tests
# VC-filesize OS 层文件 ≤500 行（审计门）
# VC-hardcoded 无硬编码 application/provider/model 业务名（审计门）
# VC-spec     openspec validate --strict
```
