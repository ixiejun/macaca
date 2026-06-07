# Change: 统一调用路径 + 协议化微内核 + 非内核能力服务化（债务清零）

## Why

2026-06-07 全面审计（`macaca/docs/2026-06-07-macaca-os-unified-call-path-audit.md`）证实：Macaca OS 已建成正确的协议化骨架（`ServiceRouter → ServiceRuntime → ServiceBus → SystemServiceBusHandler → ServiceCallExecutor → SystemService`），但与之并存的 legacy 执行轨道仍在生产运行，靠 `graph_owner / authoritative / legacy_unmarked / suppress_executor_lifecycle / legacy_*` 等标记"和稀泥"协调终态。三部治理宪法把这些 legacy 轨道定义为**债务而非可接受终态**：

- 同一"运行 agent / 调用工具"能力存在多条入口（P-KERNEL 内核直连 provider、P-WEBEXEC web shell 直跑 framework agent、P-DELEGATE 内核 executor、P-TOOLKIT web 直读 runtime）。
- 内核反向依赖 application/facade/service 层，并内置 web3/evm/a2a/payment/executor/provider_compat 等**非内核能力**。
- `macaca-persist`（foundation）反向依赖 `macaca-context`（service）。
- 依赖 gate 仍 allowlist 放行 10 条 forbidden edge；web 是 execution/tools/session/runtime 的语义所有者；存在 4 个 2000+ 行巨型 shell 文件。

本提案的目标是**完完全全**（非半成品）消除上述全部问题，使系统 100% 达到 `2026-06-07-macaca-os-protocol-microkernel-target-design.md` 的目标态：唯一协议调用路径 + 纯净微内核 + 非内核能力全部服务化/模块化 + 历史债务清零。

## What Changes

按 `2026-06-07-macaca-os-debt-elimination-refactor-plan.md` 的 P0–P5 分阶段落地，最终：

- **单一调用路径**：消灭 P-KERNEL / P-WEBEXEC / P-DELEGATE / P-TOOLKIT 四条旁路，所有应用类型（YAML/WASM/GenUI/headless）的 agent 执行与服务能力调用全部收敛到唯一 `service.call` 协议路径。**BREAKING**（内部执行路径重构，对外 HTTP/SSE 契约不变）。
- **删除多路径协调补丁**：移除 `graph_owner / authoritative / non_authoritative / legacy_unmarked / suppress_executor_lifecycle / legacy_chat_main_thread_goal_pause` 等区分逻辑（单路径下天然 authoritative）。
- **内核纯净化**：把 `web3 / evm / a2a / payment_policy / executor(worker-loop) / provider_compat` 移出 `macaca-kernel`，迁至 optional module 或 service provider；内核仅保留系统不变量。**BREAKING**（内核公共导出收缩，经版本边界表达）。
- **解除越界依赖**：`macaca-kernel` 最终仅依赖 `macaca-proto`/`macaca-ipc`；`macaca-persist` 不再依赖 `macaca-context`；`macaca-web`/`macaca-cli` 仅依赖 `macaca-sdk`。
- **Web 瘦身为 thin shell**：删除 `AppState` direct provider 字段、`framework_toolkit` 直读 runtime、session loop ownership 下沉；拆分全部 >500 行文件。
- **逃逸口由"冻结"升级为"删除"**：现有 escape-hatch 静态门从"阻止新增"升级为"存量清零"。
- **依赖 gate allowlist 清零**：10 → 0；新增 no-direct-provider-call / no-hardcoded-name / shell-not-semantic-owner 强制门。
- **OpenSpec baseline 对齐**：将本变更落地后的终态固化进 `specs/`。

## Impact

- Affected specs（capabilities）：
  - 新增 `unified-execution-path`
  - 新增 `microkernel-boundary-purity`
  - 修改 `serviceization-dependency-gate`（新增终态 allowlist=0 与强制门）
  - 修改 `serviceization-escape-hatches`（新增"删除存量逃逸口"终态）
  - 修改 `web-cli-thin-shell-completion`（新增 thin-shell 终态判据）
- Affected code（关键文件/系统）：
  - `macaca/crates/kernel/macaca-kernel/`（`lib.rs`、`kernel.rs`、`provider_compat.rs`、`kernel_builder.rs`、`web3.rs`、`evm*.rs`、`a2a*.rs`、`payment_policy.rs`、`scheduler.rs`、`persistence.rs`、`executor/`）
  - `macaca/crates/runtime/macaca-runtime-host/`（`service_runtime.rs`、`service_router.rs`、`application_execution_hosted.rs`、`wasm_runtime_provider/host_import_bridge.rs`、`agent_execution_service_provider.rs`、`compat.rs`、`domain_pack_service_provider.rs`、`finance_live_data.rs`、payment/web3/evm provider）
  - `macaca/crates/shells/macaca-web/`（`agent_execution_backend.rs`、`framework_runner.rs`、`framework_toolkit.rs`、`loop_manager.rs`、`chat_orchestrator.rs`、`state.rs`、`lib.rs`、`agent_runner.rs`）
  - `macaca/crates/shells/macaca-cli/`（`commands.rs`、`command_handlers.rs`）
  - `macaca/crates/application/macaca-app/`（`workflow.rs`、`runtime.rs`）
  - `macaca/crates/foundation/macaca-persist/`、`macaca/crates/facade/macaca-sdk/`
  - `macaca/crates/tests/macaca-integration-tests/tests/route_c_dependency_boundaries/`、`tests/serviceization_escape_hatches.rs`
  - 治理文档：`macaca/docs/macaca-os-serviceization-allowlist.md`（同步 allowlist）
- 不变更：对外 HTTP API（`/api/chat/v2` 等）、SSE 事件契约、前端、应用 manifest 格式、任务 session 隔离语义。
- 备注（非阻塞）：本变更涉及 `macaca-kernel::executor`、`AgentExecutionPort`、`ServiceRuntime`、`AppState` 等高扇出符号，按规则会触发 GitNexus HIGH/CRITICAL 影响告警。**本次仅记录备忘，不作为阻塞**（见 `design.md` §影响备忘录）。
