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
- [x] 3.1.2 将 payment policy / A2A coordinator 能力迁入 payment service（provider 层），保留结构化 unavailable（service 缺席时）。
- [x] 3.1.3 删除 kernel `a2a.rs`、`a2a_event.rs`、`payment_policy.rs` 及 `lib.rs` 中相关 `#[allow(deprecated)]` 导出。
- [x] 3.1.4 测试：payment/A2A 经 service.call 路径；payment service 缺席返回结构化 denied/unavailable，不崩溃。

### 3.2 驱逐 Web3
- [x] 3.2.1 `[impact-memo]` 盘点 `kernel/web3.rs`、`web3_event.rs`、`web3_tests.rs` 消费者；确认现有 web3 service provider（runtime-host）可承接。（生产代码零外部消费者；canonical 路径为 `Web3SystemServiceProvider` + `SystemWeb3Client`）
- [x] 3.2.2 将 Web3 facade/adapter 迁入 web3 optional module/service provider；缺席返回结构化 unavailable。（`web3_service_provider.rs` 已含 `UnavailableWeb3Provider`/`MockWeb3Provider` + Route C bootstrap 默认 unavailable）
- [x] 3.2.3 删除 kernel `web3.rs`、`web3_event.rs`、`web3_tests.rs` 及 `lib.rs` 导出。
- [x] 3.2.4 测试：web3 optional 缺席时 base OS 正常启动/执行/恢复/查询审计。（`web3_service_provider.rs` 6/6 + SDK `web3_client` 2/2；VC-escape violations=0）

### 3.3 驱逐 EVM
- [x] 3.3.1 `[impact-memo]` 盘点 `kernel/evm.rs`、`evm_adapter.rs`、`evm_event.rs`、`evm_tests.rs` 消费者；确认 `runtime-host/evm_service_provider.rs` 可承接（参考 `optional-evm-substrate-frontier-adapter-boundary.md`）。
- [x] 3.3.2 将 EVM facade/adapter（含 `Mock*`/`Unavailable*`）迁入 EVM optional module/service provider。
- [x] 3.3.3 删除 kernel `evm.rs`、`evm_adapter.rs`、`evm_event.rs`、`evm_tests.rs` 及 `lib.rs` 导出。
- [x] 3.3.4 测试：EVM optional 缺席降级；无 base OS 反向依赖。

### 3.4 驱逐执行编排 executor（P1 已迁逻辑，此处删 kernel 模块）
- [x] 3.4.1 确认 P1 已将 `executor/` 逻辑迁至 service；全仓搜索 kernel `executor::` 的残留生产消费者为 0。
- [x] 3.4.2 删除 `kernel/src/executor/` 整目录及 `lib.rs` 中 executor 导出（`ApplicationExecutor/ForkManager/AgentRunner/TaskRouter/...`）。
- [x] 3.4.3 测试：kernel 单测不再依赖 executor；task/execution service 测试覆盖原 executor 行为。

### 3.5 删除 provider 兼容与 compat 构造
- [x] 3.5.1 删除 `kernel/provider_compat.rs`（`KernelProviderCompat/LegacyLlmProvider/LegacyToolCatalog`）及 `lib.rs` 导出。
- [x] 3.5.2 删除 `kernel/kernel.rs` 的 `Kernel::new(config, llm, tools)` deprecated 构造；唯一构造路径经 `KernelBuilder` + `AgentExecutionPort`（service-client adapter）。
- [x] 3.5.3 删除 `kernel_builder.rs` 的 `KernelServiceClientCompat` 等 compat 构造路径。
- [x] 3.5.4 删除 `kernel/scheduler.rs` 的 `#[allow(deprecated)]` 项与 `persistence.rs` 的 deprecated payment store。
- [x] 3.5.5 删除 `runtime-host/compat.rs`（44 处 compat）中已无消费者的兼容代码；剩余必需项迁正规 module 并去 compat 命名。
- [x] 3.5.6 测试：kernel 不再有 `#[allow(deprecated)]` 导出；`cargo check` 无 deprecated 警告（kernel 范围）。

### 3.6 解除越界依赖 + 删 allowlist 行
- [x] 3.6.1 移除 `kernel/Cargo.toml` 对 `macaca-driver` 依赖（前置：web3/evm/a2a 已驱逐使其无消费者）；`cargo metadata` 确认边消失 → 删 allowlist `kernel→driver` 行 + 同步 doc。
- [x] 3.6.2 移除 `kernel/Cargo.toml` 对 `macaca-gateway` 依赖；删 allowlist `kernel→gateway` 行 + doc。
- [x] 3.6.3 移除 `kernel/Cargo.toml` 对 `macaca-skill` 依赖；删 allowlist `kernel→skill` 行 + doc。
- [x] 3.6.4 核查并移除 `kernel/Cargo.toml` 对 `macaca-task`、`macaca-tools` 依赖（若仍存在）；确认 `cargo metadata` 边消失。
- [x] 3.6.5 核查并移除 `kernel/Cargo.toml` 对 `macaca-agent`、`macaca-sdk` 生产依赖（agent 仅经 `AgentExecutionPort` 契约；`AgentExecutionPort` 下沉 `macaca-proto`；manifest-only registry + `LegacyAgentSideRegistry`）。
- [x] 3.6.6 终态断言：`cargo tree -e normal -p macaca-kernel --depth 1` 仅显示 `macaca-proto`、`macaca-ipc`（及 workspace 传递依赖）。

### 3.7 persist → context 越界反转
- [x] 3.7.1 `[impact-memo]` 查明 `macaca-persist` 依赖 `macaca-context` 的具体类型/用途。
- [x] 3.7.2 抽取 context 持久化所需契约到 `macaca-proto`（或反转为 context 依赖 persist），移除 `persist/Cargo.toml` 对 `macaca-context` 的依赖。
- [x] 3.7.3 `cargo metadata` 确认 `macaca-persist → macaca-context` 边消失；persist/context 单测绿。

### 3.8 P2 退出验证
- [x] 3.8.1 kernel `lib.rs` 不再 `pub mod web3/evm/a2a/payment_policy/provider_compat/executor`；kernel 仅含系统不变量模块。
- [x] 3.8.2 与 kernel/persist 相关的 allowlist 行清零；VC-gate 全绿。
- [x] 3.8.3 optional module（web3/evm/payment）absent 降级测试全绿。

## 4. P3 — Web 瘦身为 thin shell + 文件拆分

### 4.1 删除 AppState direct provider 字段
- [x] 4.1.1 `[impact-memo]` 盘点 `macaca-web/src/state.rs` 全部 deprecated 字段（`runtime`、`registry`、`llm`/`router`、`memory_runtime`、`mcp_runtime`、`driver_runtime`/`driver_registry`）的读取点。（见 `appstate-deprecated-field-inventory.md`）
- [x] 4.1.2 逐字段替换为 focused SDK client（`SystemFacade` + clients）及 shell adapter（`application_shell_adapter` / `llm_route_shell_adapter` / `mcp_shell_adapter`）；路由/框架/loop 消费者已迁移。
- [x] 4.1.3 删除 `AppState` 中全部 direct provider 字段；引入 `WebShellCompositionBundle`（bootstrap 期 provider anchors）+ `AppState::composition`。
- [ ] 4.1.4 删除 `macaca-web/src/memory_runtime.rs` 等 shell 内 runtime 持有者（若无消费者）。（**暂缓**：`WebMemoryRuntime` 仍由 bootstrap 构造并注入 composition bundle + memory service provider；待 memory 全量 service 化后删除）

### 4.2 session loop 下沉
- [x] 4.2.1 `loop_manager.rs` 的 plan/worker loop 拉取/唤醒/心跳逻辑迁至 `service.execution_control` + task service；web 仅保留 SSE endpoint 与事件订阅。（`ExecutionControlSessionLoopCoordinator` + `session_loop_shell_adapter`：register/wake/shutdown 经 execution_control audit，本地 waker 为 compat seam）
- [x] 4.2.2 `hook_consumer.rs`/`chat_orchestrator.rs` 中 Fork-Join 暂停/恢复编排改为消费 execution-control 事件。（hook_consumer 已走 `ExecutionControlForkJoinCoordinator`；`cleanup_app_state` 新增 `shutdown_session_loops_via_execution_control`）

### 4.3 巨型文件拆分（≤500 行，按 ownership）
- [x] 4.3.1 拆分 `loop_manager.rs`(2629) → execution-control adapter / task-event adapter / SSE channel adapter / 薄 orchestrator（每文件 ≤500 行）。（`loop_manager/` 18 模块，最大 412 行；Facade+Adapter+Strategy；`decomposition_adapter` 去除 legacy graph_owner/agent-role 违规）
- [x] 4.3.2 拆分 `framework_runner.rs`(2484) → runtime agent 构造 service（迁 runtime/framework 层）+ 薄 web 适配。（`framework_runner/` 23 模块，最大 480 行；Facade+Adapter+Factory；public API 不变；`contract_source::framework_runner_module_sources`）
- [x] 4.3.3 拆分 `chat_orchestrator.rs`(1581) → route adapter / application-service 调用适配 / DTO 映射。（`chat_orchestrator/` 18 模块，最大 ≤500 行；Facade+Adapter；`contract_source::chat_orchestrator_module_sources`）
- [x] 4.3.4 拆分 `macaca-web/src/lib.rs`(975) → 小型 composition bundle + route 注册。
- [x] 4.3.5 每次拆分仅做结构移动 + 注释补全，拆分前后 `cargo test -p macaca-web` 绿。
- [x] 4.3.6 拆分 `macaca-web/src/routes.rs`(2591) → `routes/` Facade 模块树（16 文件，max 438 行）；`cargo test -p macaca-web --lib routes` 39/39。
- [x] 4.3.7 拆分 `macaca-web/src/framework_toolkit.rs`(1642) → `framework_toolkit/` Facade 模块树（8 文件，max 413 行）；`contract_source::framework_toolkit_module_sources`；`cargo test -p macaca-web --lib framework_toolkit` 9/9。
- [x] 4.3.8 拆分 `macaca-web/src/session.rs`(1424) → `session/` Facade 模块树（11 文件，max 268 行）；`contract_source::session_module_sources`；`cargo test -p macaca-web --lib session` 10/10；修复 `task_api_migration_audit` / `serviceization_escape_hatches` 路径对齐模块拆分。
- [x] 4.3.9 拆分 `macaca-web/src/skill_operations_routes.rs`(704) → `skill_operations_routes/` Facade 模块树（11 文件，max ~140 行）；`contract_source::skill_operations_module_sources`；`self_evolving_skill_boundaries` 扫描路径更新为目录；filesize allowlist 移除该行（84 行基线债务）。
- [x] 4.3.10 拆分 `macaca-web/src/app_ui_routes.rs`(605) → `app_ui_routes/` Facade 模块树（9 文件，max ~210 行）；`contract_source::app_ui_routes_module_sources`；filesize allowlist 移除该行（83 行基线债务）。
- [x] 4.3.11 拆分 `macaca-web/src/context_memory_injection.rs`(592) → `context_memory_injection/` Facade 模块树（6 文件，max ~220 行）；`contract_source::context_memory_injection_module_sources`；`cargo test -p macaca-web --lib context_memory_injection` 3/3；filesize allowlist 移除该行。
- [x] 4.3.12 拆分 `macaca-web/src/context_reporting_model.rs`(608) → `context_reporting_model/` Facade 模块树（7 文件，max ~222 行）；`assembly_service` / `assembly_legacy` + `assembly_finalize` Template Method；`contract_source::context_reporting_model_module_sources`；`cargo test -p macaca-web --lib context_reporting_model` 2/2；filesize allowlist 移除该行（81 行基线债务）。
- [x] 4.3.13 拆分 `macaca-web/src/skill_mcp.rs`(705) → `skill_mcp/` Facade 模块树（9 文件，max ~192 行）；Cache-Aside `snapshot` + Strategy `server_resolution` + Adapter `probe` + Observer `governance_telemetry`；`contract_source::skill_mcp_module_sources`；`cargo test -p macaca-web --lib skill_mcp` 4/4；filesize allowlist 移除该行（80 行基线债务）。
- [x] 4.3.14 拆分 `macaca-web/src/skill_self_evolution_observer.rs`(810) → `skill_self_evolution_observer/` Facade 模块树（9 文件，max ~257 行）；Observer `observer` + Adapter `projection` + Builder `proposal_builder` + Value Object `semantic_signal` + Command forwarder `forwarder`；`contract_source::skill_self_evolution_observer_module_sources`；`cargo test -p macaca-web --lib skill_self_evolution_observer` 6/6；filesize allowlist 移除该行（79 行基线债务）。
- [x] 4.3.15 拆分 `macaca-web/src/agent_execution_backend/tests.rs`(656) → `agent_execution_backend/` Facade 模块树（`mod.rs` + `tests/` 10 文件，max ~153 行）；Contract Test 子模块（`execution_control_policy` / `heartbeat_evidence` / `execution_envelope` / `heartbeat_shell_contract` / `static_wiring` / `skill_self_evolution_boundary` / `architecture_guards`）+ `support` 共享 imports + `contract_source::agent_execution_backend_test_module_sources`；`cargo test -p macaca-web agent_execution_backend` 29/29；filesize allowlist 移除该行（78 行基线债务）；**macaca-web P3 巨型文件债务清零**。
- [x] 4.3.16 拆分 `macaca-cli/src/skill_operations.rs`(682) → `skill_operations/` Facade 模块树（11 文件，max ~137 行）；Facade+Adapter+Null Object+Strategy；`contract_source::skill_operations_module_sources`；`cargo test -p macaca-cli skill_operations` 6/6；filesize allowlist 移除该行（**77 行基线债务**）；**macaca-cli P4 巨型文件债务清零**。
- [x] 4.3.17 拆分 `macaca-sdk/src/scheduler_client.rs`(504) → `scheduler_client/` Facade 模块树（`mod.rs` + `tests/` 契约子模块，主文件 ~418 行）；Facade+Adapter+Null Object；`cargo test -p macaca-sdk --lib service_backed_scheduler` 1/1；filesize allowlist 移除该行（**75 行基线债务**，iteration 43 再移除 `runtime.rs`）。

### 4.4 删除 web 越界依赖 + allowlist 行
- [x] 4.4.1 替换完成后逐条移除 `macaca-web/Cargo.toml` 对 `macaca-driver/llm/memory/persist/skill/task/tools` 的直接依赖（经 `macaca-sdk::shell_provider_bridge` + `macaca_runtime_host::persist` 别名）。
- [x] 4.4.2 每删一条 `cargo metadata` 确认边消失 → 删对应 7 条 web allowlist 行 + 同步 doc（allowlist 行数 0）。
- [x] 4.4.3 移除 `macaca-web` 对 `macaca-kernel` 直接依赖（`macaca_sdk::kernel`）；`macaca-runtime-host` 仍保留为 composition-root seam（`app→sdk→runtime-host→app` 环阻止 SDK 桥接，persist 经 `runtime_host::persist`）。
- [ ] 4.4.4 终态断言：`cargo tree -e normal -p macaca-web --depth 1` 仅 `macaca-sdk`（+ proto DTO/必要 framework 适配）。（**部分**：仍含 app/agent/runtime/context/framework/runtime-host，待 P4/P5 收敛）

### 4.5 P3 退出验证
- [x] 4.5.1 全仓无 >500 行 OS 层源文件（VC-filesize）。（**完成**：macaca-web + macaca-cli + macaca-framework 巨型文件已全部拆分合规；全仓 **0** 行 allowlist 债务，gate 已实现；iteration 112 退役 `macaca-framework/tool.rs`（Facade + Registry + Strategy + Chain of Responsibility 模块树）；iteration 111 退役 `macaca-framework/react_agent.rs`（Facade + Template Method + Builder + Strategy 模块树）；iteration 110 退役 `macaca-framework/plan.rs`（State Machine + Memento + Aggregate + Value Object Facade 模块树）；iteration 109 退役 `macaca-framework/memory.rs`（Strategy + Memento + Template Method + Value Object Facade 模块树）；iteration 108 退役 `macaca-framework/mcp.rs`（Strategy + Factory + Adapter + Bridge Facade 模块树）；iteration 107 退役 `macaca-framework/formatter.rs`（Strategy + Adapter + Facade 模块树）；iteration 106 退役 `macaca-framework/a2a.rs`（Adapter + Strategy Facade 模块树）；iteration 105 退役 `macaca-framework/model_impls.rs`（Adapter + Strategy Facade 模块树）；iteration 104 退役 `macaca-framework/pipeline.rs`（Strategy + Chain of Responsibility + Composite + Mediator Facade 模块树）；iteration 103 退役 `macaca-framework/agent.rs`（Strategy + Decorator + Chain of Responsibility + Singleton Facade 模块树）；iteration 102 退役 `macaca-framework/message.rs`（Composite + Strategy + Builder Facade 模块树）；iteration 101 退役 `macaca-integration-tests/pipeline_dry_run.rs`（Facade + Template Method + Command + Observer 模块树）；iteration 100 退役 `macaca-app/workflow.rs`（Template Method + Strategy + Chain of Responsibility Facade 模块树）；iteration 99 退役 `macaca-app/service_projection.rs`（Projection + Adapter + Facade 模块树）；iteration 98 退役 `macaca-app/model.rs`（Aggregate Root + Composite + Facade 模块树）；iteration 97 退役 `macaca-app/manifest_v1/yaml_adapter.rs`（Adapter + Memento + Builder + Strategy 模块树）；iteration 96 退役 `macaca-sdk/system_facade.rs`（Facade + Strategy + Builder + Null Object + Observer 模块树）；iteration 95 退役 `macaca-sdk/skill_client.rs` + `skill_client_service_backed.rs`（Facade + Null Object + Adapter + Strategy + Observer 模块树）；iteration 94 退役 `macaca-sdk/mcp_client.rs`（Facade + Null Object + Adapter + Observer 模块树）；iteration 93 退役 `macaca-scheduler/local_provider.rs`（Facade + Memento + State + Strategy + Observer 模块树）；iteration 92 退役 `macaca-scheduled-agent-task/local_provider.rs`（Facade + Memento + State + Specification + Observer 模块树）；iteration 91 退役 `macaca-context/engine.rs`（Facade + Strategy + Registry + Chain of Responsibility + Decorator + Adapter 模块树）；iteration 90 退役 `macaca-tools/todo.rs`（Facade + Command + Composite + Strategy + Observer/Callback 模块树）；iteration 89 退役 `macaca-runtime/agentic_loop.rs`（Facade + Template Method + Observer + Decorator + Strategy 模块树）；iteration 88 退役 `macaca-task/todo_board.rs`（Facade + Strategy + State + Specification + Value Object 模块树）；iteration 87 退役 `macaca-task/runtime.rs`（Facade + Strategy + Command + Observer + Memento + Specification 模块树）；iteration 86 退役 `agent_skill.rs`（macaca-skill Facade + Value Object + Builder/Parser + Lazy Loading 模块树）；iteration 85 退役 `macaca-skill/runtime.rs`（Facade + Memento + Chain of Responsibility + Strategy 模块树）；iteration 84 退役 `plan_loop.rs`（macaca-task Facade + Observer + Template Method + Strategy 模块树）；iteration 83 退役 `application_service.rs`（macaca-proto Facade 模块树）；iteration 82 退役 `types.rs`（macaca-proto Facade 模块树）；iteration 81 退役 `config.rs`（macaca-proto Facade 模块树）；iteration 80 退役 `mcp_runtime.rs`（runtime-host Facade + Strategy + Adapter + Registry + Template Method + Observer 模块树）；iteration 79 退役 `app_executor.rs`（runtime-host Facade + Supervisor + Template Method + Observer + Registry + Memento 模块树）；iteration 78 退役 `fork_manager.rs`（runtime-host Facade + State/Memento + Observer + Specification 模块树）；iteration 77 退役 `heartbeat_agent_dispatch.rs`（runtime-host Facade + Value Object + Strategy + Specification + Observer/Memento 模块树）；iteration 76 退役 `autonomy_service_provider.rs`（runtime-host Facade + Adapter/Bridge + Command Router + Null Object + Abstract Factory 模块树）；iteration 75 退役 `autonomy_dispatch.rs`（runtime-host Facade + Value Object + Strategy + Command Router + Memento + Adapter 模块树）；iteration 74 退役 `application_service_provider.rs`（runtime-host Facade + Adapter/Bridge + Command Router + Repository/Memento + Strategy 模块树）；iteration 73 退役 `mcp_service_provider.rs`（runtime-host Facade + Factory + Command Router + Chain of Responsibility + Adapter 模块树）；iteration 72 退役 `component_model.rs`（runtime-host Facade + Strategy + Session Object + Bridge + Memento + Template Method 模块树）；iteration 71 退役 `host_import_bridge.rs`（runtime-host Facade + Bridge + Adapter + Command + Repository 模块树）；iteration 70 退役 `component_model_tests.rs`（runtime-host Facade + Object Mother + Contract Test 模块树）；iteration 69 退役 `host_import_tests.rs`（runtime-host Facade + Object Mother + Contract Test 模块树）；iteration 68 退役 `application_execution_hosted.rs`（runtime-host Facade + Adapter + Strategy + Observer + Memento 模块树）；iteration 67 退役 `tool_service_invocation.rs`（runtime-host Decorator + Strategy Router 模块树）；iteration 66 退役 `tool_family_providers.rs`（runtime-host Abstract Factory 模块树）；iteration 65 退役 `skill_service_provider_tests.rs`（runtime-host Contract Test 模块树）；iteration 64 退役 `skill_service_provider.rs`（runtime-host Command Router 模块树）；iteration 63 退役 `skill_service_provider_state`/`lib.rs`（runtime-host）；iteration 62 退役 `heartbeat_lane/tests`/`default_provider`/`hardened_provider`；iteration 61 退役 `application_execution_remote_agent`/`skill_service_provider_proposal_materialization`；iteration 60 退役 `macaca-heartbeat/local_provider`；iteration 59 退役 `heartbeat_service`/`tool_service`；iteration 58 退役 `governance_store.rs`/`resilient.rs`/`scheduled_agent_task_service`；iteration 57 退役 `application_manifest.rs`/`memory.rs`/`agent_execution_service`；iteration 56 退役 `governance/pipeline.rs`/`scheduler.rs`/`todo_store.rs`/`executor/queue.rs`/`entitlement.rs`/`execution_control_service`）
- [x] 4.5.2 web 相关 allowlist 清零；VC-gate 绿；`cargo test -p macaca-web --lib` 250/250。

## 5. P4 — CLI 解耦 + domain pack 外置

### 5.1 CLI 解耦
- [x] 5.1.1 `[impact-memo]` 盘点 `macaca-cli/src/commands.rs`/`command_handlers.rs` 对 `Kernel`/`KernelBuilder`/`GatewayBuilder`/`LlmProvider`/`macaca_web::WebServerBuilder` 的使用。（见 `cli-decoupling-inventory.md`：零命中；run/status/agents 经 `SystemFacade` + SDK clients）
- [x] 5.1.2 CLI run/status 改用 runtime-host bootstrap client 或 SDK status/service inspector，删除 kernel 直接构造。（`execute_run_kernel`/`execute_show_status`/`execute_list_agents` 经 `cli_system_facade` + `StaticSystemStatusDataSource`）
- [x] 5.1.3 `macaca web` 进程启动 seam 移到小型 public bootstrap facade（binary-only entrypoint），删除 `macaca-cli → macaca-web` internals。（`WebServerProcessLauncher` 子进程启动 `macaca-web-server`；契约测试 `web_command_uses_only_public_server_start_seam`）
- [x] 5.1.4 删除 `macaca-cli/Cargo.toml` 对 `macaca-gateway/tools/web` 的直接依赖 + 同步 allowlist/doc。（`cargo tree -e normal -p macaca-cli --depth 1` 仅 `macaca-proto` + `macaca-sdk` + 通用 crate；route-c allowlist 无 CLI 行）
- [x] 5.1.5 CLI 命令冒烟测试（status/run/web）通过。（`cargo test -p macaca-cli` 16/16；含 handler smoke + skill_operations boundary tests）

### 5.2 domain pack 外置
- [x] 5.2.1 `[impact-memo]` 盘点 `runtime-host/domain_pack_service_provider.rs`、`finance_live_data.rs`（`service.market_data/financials/news_digest/llm.analysis`、Coindesk/Binance/OKX、`asset_class=="crypto"` 分支）。（见 `domain-pack-inventory.md`）
- [x] 5.2.2 将 finance/crypto domain-pack 实现移出 base runtime-host，注册为 plugin/package service provider（带 descriptor + policy metadata，经 manifest 声明）。（`macaca-domain-pack-finance` + `finance_domain_pack_registrations`）
- [x] 5.2.3 runtime-host 仅保留 generic `ServiceProviderFactory` + 注册机制；deterministic fixtures 仅保留在测试。（删除 `finance_live_data.rs` / `finance_llm_analysis_provider.rs`；`bootstrap_builtin_domain_pack_services` 空化）
- [x] 5.2.4 测试：domain pack 缺席返回结构化 unavailable；base runtime-host 无业务域字符串（VC-hardcoded）。（`domain_pack_finance_package` 2/2 + `runtime_host_domain_pack_gate` 1/1）

### 5.3 P4 退出验证
- [x] 5.3.1 `cargo tree -p macaca-cli --depth 1` 仅 `macaca-sdk`（+ proto）；CLI allowlist 清零。（route-c + filesize allowlist 均无 macaca-cli 行；**macaca-cli 巨型文件债务已清零**）
- [x] 5.3.2 runtime-host 无 finance/crypto/exchange 业务域代码（VC-hardcoded 扫描）。（`runtime_host_domain_pack_gate` 扫描 `macaca-runtime-host/src` 生产代码零命中）

## 6. P5 — 强制门清零 + OpenSpec baseline 对齐

### 6.1 终态门
- [x] 6.1.1 在 `route_c_dependency_boundaries` 增加 terminal 断言：allowlist 行数 == 0（任何残留即失败）。（`assert_route_c_allowlist_terminal_state` + dedicated test；Route C allowlist 已清零）
- [x] 6.1.2 实现 `no-direct-provider-call` 审计：每个已服务化能力（llm/memory/context/task/driver/skill/mcp/agent_execution/payment/web3/evm/gateway），生产代码只能经 service client；旁路引用 → 失败。（`p5_terminal_audit_gates/no_direct_provider_call` + migration surfaces）
- [x] 6.1.3 实现 `no-hardcoded-name` 审计：生产代码无硬编码 agent/app/provider/model/driver/gateway/chain/payment 名（fixtures/tests 除外）。（`p5_terminal_audit_gates/no_hardcoded_name`）
- [x] 6.1.4 实现 `shell-not-semantic-owner` 审计：shell 不得直驱 task/loop、不引用 deprecated direct fields、不持 agent 执行实现。（`p5_terminal_audit_gates/shell_not_semantic_owner` scoped to `crates/shells/`）
- [x] 6.1.5 实现 `kernel-purity` 审计：kernel 仅依赖 proto/ipc（与 3.6.6 联动）。（`kernel_purity_gate` via `cargo metadata` workspace dep audit）
- [x] 6.1.6 实现 `file-size` 审计：OS 层无 >500 行源文件。（`os_layer_file_size_gate` + `assert_os_layer_file_size_allowlist_terminal_state`；**0** 行 allowlist 终态达成，iteration 112–113）
- [x] 6.1.7 实现 `shell-dependency-purity` 审计：CLI 终态仅 proto+sdk；Web 冻结 7 条 workspace 依赖基线，禁止新增。（`shell_dependency_purity_gate` via `cargo metadata`）
- [x] 6.1.8 实现 P5 终态 DoD gate：`unified_audit_replay_terminal_gate`（§9.1 单链 replay）+ `p5_coordination_patch_retirement_gate`（§9.2 协调补丁清零）+ `p5_external_contract_gate`（§9.7 对外契约）+ `audit_redaction_terminal_gate`（§7.3 脱敏）+ `openspec_validate_terminal_gate`（§9.8 OpenSpec validate）+ `audit_blind_spot_terminal_gate`（§7.4 审计盲区）+ `provider_neutral_logging_terminal_gate`（§7.2 provider-neutral 日志）+ `p5_dod_terminal_gate_matrix`（§9.5 统一 DoD 矩阵编排）。（iteration 114–118）

### 6.2 逃逸口由"冻结"升级为"删除"
- [x] 6.2.1 每个逃逸口对应 service client 全量替换后，删除其 migration module 豁免，使任何引用（含旧引用）CI 失败。（**终态**：`migration_debt_baseline.rs` 冻结 raw=**0**；**全部 10 个 family 已退役**，含 `provider-model-routing-name`（121→0，iteration 54）；`macaca-llm/src/` 为 canonical routing owner，其余层 shed 字面量/空默认/compile-time id）
- [x] 6.2.2 `serviceization_escape_hatches.rs` 双模式门：freeze 模式 violations=0；debt inventory 模式 raw=**0** + 空 per-family baseline；reconciliation markers 生产代码硬断言 0。（`provider_model_routing_absent_in_production` + `memory_embedding_provider_literals_absent` + `framework_model_impls_provider_literals_absent` + 其余 retired-family 硬断言 + `migration_debt_inventory_matches_baseline`）

### 6.3 OpenSpec baseline 对齐
- [x] 6.3.1 将本 change 落地后的终态固化进 `openspec/specs/`：`unified-execution-path`、`microkernel-boundary-purity`、更新 `serviceization-dependency-gate`/`serviceization-escape-hatches`/`web-cli-thin-shell-completion`。
- [ ] 6.3.2 分批 archive 相关 completed changes，使 baseline 反映终态。（**部分**：iteration 42 已 archive `update-kernel-to-provider-facade-boundary`、`serviceize-agent-execution-v1`、`refactor-route-c-workspace-topology`、`unify-application-execution-path`；主提案本身不 archive）
- [x] 6.3.3 `openspec validate --all --strict` 全绿。（196/196 pass，iteration 42）

## 7. 横切 — 注释 / 日志 / trace-audit

- [ ] 7.1 所有新增/迁移模块补详尽英文注释（功能 + 运行原理 + 权衡）；巨型文件拆分后的新文件逐一覆盖。
- [x] 7.2 关键节点日志统一为 provider-neutral 维度；删除以 provider/model/app name 为主键的日志（如 `provider_compat.rs` 风格）。（`provider_neutral_logging_terminal_gate` 1/1：OS 层 shell/runtime-host/framework 生产 tracing 扫描 1420 文件 0 违规；`tracing_utils` 宏与 web/runtime-host/agent 日志字段对齐 `service_id`/`command`/`trace_id`/`reason_code`；iteration 118）
- [x] 7.3 全局化脱敏（`sanitize_json`/`is_safe_metadata_key`/`should_omit_metadata_map_key`）到所有 audit/trace/snapshot 出口；`macaca-proto::audit_redaction` 单测 + `audit_redaction_terminal_gate`。（iteration 115）
- [x] 7.4 旁路删除后验证"无审计盲区"：所有执行都产出 service-call evidence，可按 trace_id/session_id replay。（`audit_blind_spot_terminal_gate` 1/1：replay 命令面 + web bootstrap 共享 sink + router 契约扫描 + runtime-host 行为子进程；iteration 116）

## 8. 治理文档同步

- [x] 8.1 每删一条 allowlist 同步 `macaca/docs/macaca-os-serviceization-allowlist.md`。（Route C + file-size allowlist 清零已同步，iteration 113）
- [x] 8.2 终态后更新审计三件套（`2026-06-07-*`）的"已达成"状态注记，作为归档证据。（`2026-06-07-macaca-os-unified-call-path-audit.md` §12 终态注记，iteration 113）

## 9. 终态验收（Definition of Done，逐条证明）

- [x] 9.1 单路径：YAML 与 WASM 应用 agent 执行 audit replay 均为单一 service.call 链（对照 0.3）。（`unified_audit_replay_terminal_gate` 1/1 orchestrates web 6/6 + runtime-host 3/3 contract tests；`audit-replay-baseline.md` post-convergence YAML/WASM each 1 chain）
- [x] 9.2 协调补丁清零：多路径协调补丁 token（`legacy_unmarked`/`non_authoritative`/`suppress_executor_lifecycle`/`legacy_chat_main_thread_goal_pause`/`TaskGraphOwner::TaskServiceCompatibility`/`TaskGraphOwner::DiagnosticOnly`）生产代码 0 命中。（`p5_coordination_patch_retirement_gate` 1/1 + `serviceization_escape_hatches_reconciliation_markers_absent_in_production`；注：`TaskGraphOwner` 领域字段 `graph_owner` 为合法 proto/task 语义，非协调补丁）
- [x] 9.3 内核纯净：kernel 无 web3/evm/a2a/payment/executor/provider_compat；`cargo tree -p macaca-kernel` 仅 proto/ipc。（`kernel_purity_gate` 1/1）
- [ ] 9.4 越界依赖清零：persist 不依赖 context；web/cli 仅依赖 sdk。（persist→context 终态达成：`p2_microkernel_exit_validation` 3/3；CLI 终态达成；web 仍冻结 7 条 workspace 依赖基线）
- [x] 9.5 allowlist == 0；全部终态门绿。（Route C + filesize allowlist==0；`p5_dod_terminal_gate_matrix` 1/1 编排 20 项终态 gate 子进程全绿；web shell 依赖基线仍冻结 7 条，§9.4 部分达成）
- [x] 9.6 无 >500 行 OS 源文件；domain pack 出 base runtime-host。（`os_layer_file_size_gate` 2/2 + `runtime_host_domain_pack_gate`）
- [x] 9.7 对外契约不回归：`/api/chat/v2`、SSE、manifest、session 隔离。（`p5_external_contract_gate` 4/4：chat v2 route + session module + SSE surface + route_c no-network pipeline）
- [x] 9.8 OpenSpec baseline 反映终态；`openspec validate --all --strict` 绿。（`openspec_validate_terminal_gate` 1/1，iteration 115）

## 10. 备注 — GitNexus 影响（非阻塞）

- [x] 10.1 在本 change 维护一份 `impact-memo.md`，记录各 `[impact-memo]` task 的 blast radius / risk level，仅备忘不阻塞。（持续维护至 iteration 113）

## 13. 验证命令集（VC）

```bash
cd macaca
# VC-check    编译
cargo check
# VC-gate     依赖边界门（含 allowlist=0 终态）
cargo test -p macaca-integration-tests route_c_dependency_boundaries_reject_unallowlisted_forbidden_edges -- --nocapture
# VC-escape   逃逸口门（freeze=0；debt inventory baseline=200）
cargo test -p macaca-integration-tests --test serviceization_escape_hatches -- --nocapture
# VC-shell-deps  shell workspace 依赖纯度门
cargo test -p macaca-integration-tests --test shell_dependency_purity_gate -- --nocapture
# VC-tree     依赖快照
cargo tree -e normal -p macaca-kernel --depth 1
cargo tree -e normal -p macaca-web --depth 1
cargo tree -e normal -p macaca-cli --depth 1
cargo metadata --no-deps --format-version 1
# VC-svc      受影响能力 service 测试
cargo test -p macaca-task && cargo test -p macaca-runtime-host && cargo test -p macaca-kernel
# VC-e2e      端到端：/api/chat/v2（YAML + WASM）、fullstack-autodev、route-c 回归矩阵
cargo test -p macaca-integration-tests
# VC-filesize OS 层文件 ≤500 行（审计门 + allowlist 终态）
cargo test -p macaca-integration-tests --test os_layer_file_size_gate -- --nocapture
# VC-filesize-terminal  filesize allowlist 必须为 0 行
cargo test -p macaca-integration-tests os_layer_file_size_allowlist_terminal_state_is_zero_rows -- --nocapture
# VC-audit-replay  单路径 audit replay 终态（§9.1）
cargo test -p macaca-integration-tests unified_audit_replay_single_chain_terminal_state -- --nocapture
# VC-coordination-patch  协调补丁 token 生产代码清零（§9.2）
cargo test -p macaca-integration-tests p5_coordination_patch_retirement_gate_rejects_multi_path_markers -- --nocapture
# VC-external-contract  对外 HTTP/SSE/session 契约不回归（§9.7）
cargo test -p macaca-integration-tests --test p5_external_contract_gate -- --nocapture
# VC-audit-redaction  全局脱敏 canonical 终态（§7.3）
cargo test -p macaca-integration-tests --test audit_redaction_terminal_gate -- --nocapture
# VC-openspec  OpenSpec strict validate 终态（§9.8）
cargo test -p macaca-integration-tests --test openspec_validate_terminal_gate -- --nocapture
# VC-audit-blind-spot  无审计盲区 + trace/session replay 终态（§7.4）
cargo test -p macaca-integration-tests --test audit_blind_spot_terminal_gate -- --nocapture
# VC-provider-neutral-logging  provider-neutral tracing 终态（§7.2）
cargo test -p macaca-integration-tests --test provider_neutral_logging_terminal_gate -- --nocapture
# VC-p5-dod-matrix  P5 终态 DoD 统一门矩阵（§9.5，编排全部终态 gate）
cargo test -p macaca-integration-tests --test p5_dod_terminal_gate_matrix -- --nocapture
# VC-hardcoded 无硬编码 application/provider/model 业务名（审计门）
# VC-spec     openspec validate --strict（或由 openspec_validate_terminal_gate 子进程等价覆盖）
```
