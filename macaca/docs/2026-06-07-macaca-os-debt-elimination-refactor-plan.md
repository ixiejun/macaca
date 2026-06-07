# Macaca OS 债务清除与单路径收敛 重构方案

日期：2026-06-07
依据审计：`2026-06-07-macaca-os-unified-call-path-audit.md`
依据目标：`2026-06-07-macaca-os-protocol-microkernel-target-design.md`
性质：重构计划（不含代码改动）。每个阶段落地前须按强制工作流（brainstorm → write-plan → OpenSpec proposal/design/tasks/spec → 实现 → 测试 → 归档）执行。

## 0. 原则

1. **删除优先于新增**：目标是收敛与拆除，不是再造抽象。canonical 路径（ServiceRuntime/ServiceRouter/ServiceCallExecutor/SystemService）已正确，**禁止**重写。
2. **小步可逆**：每步保持 `cargo check` 绿、`route_c_dependency_boundaries` gate 绿、目标 service 测试绿。
3. **规范先行**：行为/接口/依赖/所有权变更先更新 OpenSpec 与边界文档，再改代码。
4. **以 gate 证明进展**：唯一可信的"债务减少"证据是 allowlist 行数下降 + escape-hatch 测试通过 + audit replay 显示单路径。
5. **不破坏公共契约**：破坏性变更经版本边界表达。
6. **冲突时宪法优先**：与三部治理文档冲突，以治理文档为准。

## 1. 风险与影响评估（编辑前必做）

每阶段动符号前运行 GitNexus impact：`gitnexus_impact({target, direction:"upstream"})`，HIGH/CRITICAL 必须先告知再动。高风险热点（预判）：
- `macaca-kernel::executor::*`（被 web/runtime-host 广泛使用）—— CRITICAL。
- `AgentExecutionPort` / `kernel.execute_agent` —— HIGH。
- `ServiceRuntime` / `ServiceRouter` —— HIGH（不改实现，仅扩大其覆盖面）。
- `macaca-web AppState` 字段 —— HIGH（多路由依赖）。

## 2. 阶段总览

| 阶段 | 名称 | 核心目标 | 退出判据 |
|------|------|----------|----------|
| P0 | 冻结逃逸口 | 阻止债务增长 | escape-hatch 测试覆盖全部旁路；allowlist 每行带 owner/caller/expiry |
| P1 | 统一 Agent 执行为单一 service | 消灭 P-KERNEL / P-WEBEXEC / P-DELEGATE 三入口 | 所有 agent 执行经 `service.agent_execution`；删除协调补丁 |
| P2 | 内核纯净化 | 移出非内核能力与越界依赖 | kernel 仅依赖 proto/ipc；web3/evm/a2a/payment/executor/provider_compat 移出 |
| P3 | Web 瘦身为 thin shell | 删除 direct provider / 巨型文件 | web 仅依赖 sdk；无 ≤500 行违规 |
| P4 | CLI 解耦 + 外置 domain pack | shell/插件边界归位 | cli 不依赖 web；finance/crypto pack 出 runtime-host |
| P5 | Gate 清零 + OpenSpec 对齐 | 终态固化 | allowlist=0；baseline specs 反映终态 |

> 顺序原则：先冻结（P0），再消灭执行双轨（P1，因为它是 kernel/web 债务的总根），随后内核纯净化（P2）与 web 瘦身（P3）可并行推进，最后收尾（P4/P5）。

---

## 3. P0：冻结逃逸口（Freeze Escape Hatches）

目的：在删除前先"封死增量"，把现有旁路变成可执行清单。复用现有 `tests/serviceization_escape_hatches.rs` 与 `route_c_dependency_boundaries`。

任务：
1. 扩展 escape-hatch 静态测试，覆盖全部 §4.2 旁路：
   - 生产代码（migration module 外）不得新增对 `KernelProviderCompat`/`LegacyLlmProvider`/`LegacyToolCatalog` 的引用。
   - 生产代码不得新增 `AppState` deprecated 字段读取（runtime/llm provider/router/memory runtime/mcp runtime/driver registry）。
   - 生产代码不得新增 `AppRuntime::start_app` / `start_app_from_file` 调用。
   - 生产代码不得新增直读 `driver_runtime.collect_tools()` / `mcp_runtime.definitions()`。
   - 生产代码不得新增硬编码 `coordinator/planner/worker/backend/frontend/architect`（fixtures/tests 除外）。
2. 为 allowlist 每行补充：当前 caller path、目标 service client、负责阶段、expiry。
3. 把 deprecation warnings 按 serviceization track 分组，形成可执行迁移清单。

退出判据：上述测试全绿；新增旁路一律 CI 失败。
验证：`cargo test -p macaca-integration-tests serviceization_escape_hatches`、`... route_c_dependency_boundaries_reject_unallowlisted_forbidden_edges`。

---

## 4. P1：统一 Agent 执行为单一 Service（最关键）

目的：消灭"运行一个 agent / 调一个工具"的三入口（P-KERNEL / P-WEBEXEC / P-DELEGATE），全部并入 `service.agent_execution` → §2 单路径，并删除协调补丁。

### 4.1 把执行实现迁出 shell/kernel
- 将 `WebAgentExecutionBackend` 的执行逻辑（`agent_execution_backend.rs` 模型/工具/loop 部分）下沉为 runtime-host 的 Agent Execution Service provider 实现；web 仅保留 SSE channel + DTO 适配。
- 将 kernel `executor::ApplicationExecutor/ForkManager/AgentRunner/TaskRouter` 的执行编排迁到 task/execution service 后面；kernel 仅保留 `AgentExecutionPort` typed 抽象。
- 收紧 `AgentExecutionPort`：只接 service client / typed handle，**禁止**接 provider trait。删除 `provider_compat.rs` 的 production 构造路径。

### 4.2 删除协调补丁（双轨消失的直接收益）
- 删除 `application_execution_hosted.rs:546-695` 的 `authoritative/non_authoritative/legacy_unmarked` 区分逻辑——单路径下所有 task 天然 authoritative。
- 删除 `host_import_bridge.rs` 中 `graph_owner` 双重标记（`graph_owner` + `execution.graph_owner`）的"区分真实/兼容"用途（保留必要的纯审计标签即可）。
- 删除 `agent_execution_backend.rs:151-163` `should_emit_executor_lifecycle/suppress_executor_lifecycle`——单一发事件方后无需去重。
- 删除 `agent_execution_backend.rs:386-513` `legacy_execution_control_policy` / `legacy_chat_main_thread_goal_pause`——改由 manifest projection 提供 execution-control policy。

### 4.3 YAML 路径并轨
- `macaca-app/src/workflow.rs` 与 web `agent_runner.rs` 的 workflow 执行改为统一调用 Application Service → service.agent_execution，不再走 kernel executor 直驱。

退出判据：audit replay 显示任意应用（YAML/WASM）的 agent 执行只出现一条 service.call 链；`graph_owner/authoritative/suppress_executor_lifecycle/legacy_*` 标记在生产代码中清零。
验证：现有 route-c 回归矩阵（`docs/route-c-regression-matrix.md`）+ `/api/chat/v2` 端到端 + WASM host import 测试 + fullstack-autodev 集成测试。

---

## 5. P2：内核纯净化

目的：内核回归 §4（目标设计）的"仅不变量"，删除越界依赖与非内核能力。

### 5.1 移出非内核能力
| 动作 | 文件 | 去向 |
|------|------|------|
| 删/移 Web3 | `kernel/web3.rs`、`web3_event.rs`、`web3_tests.rs` | `macaca-web3` optional module / runtime-host provider |
| 删/移 EVM | `kernel/evm.rs`、`evm_adapter.rs`、`evm_event.rs`、`evm_tests.rs` | EVM optional module（见 `optional-evm-substrate-frontier-adapter-boundary.md`） |
| 删/移 A2A/Payment | `kernel/a2a.rs`、`a2a_event.rs`、`payment_policy.rs` | payment service（runtime-host `payment_service_provider.rs` 已存在） |
| 移出执行编排 | `kernel/executor/*` | task/execution service（P1 已迁逻辑，此处删 kernel 模块） |
| 删除 provider 兼容 | `kernel/provider_compat.rs`、`kernel_builder.rs` 的 `KernelProviderCompat/KernelServiceClientCompat` compat 构造、`Kernel::new(llm,tools)` | 删除 |
| 清理 deprecated | `kernel/scheduler.rs`(`#[allow(deprecated)]`)、`persistence.rs`(deprecated payment store) | 删除 deprecated 项 |

### 5.2 解除越界依赖（逐条删 allowlist）
按优先级删除直接依赖边后，再删对应 allowlist 行：
1. `macaca-kernel → macaca-driver / gateway / skill`（移出 web3/evm/a2a 后这些依赖应消失）。
2. 顺带核查并消除 `macaca-kernel → macaca-task / tools / agent / sdk`（agent 改为只依赖 `AgentExecutionPort` 契约；sdk 不应被 kernel 依赖）。
3. `macaca-persist → macaca-context`：将 persist 对 context 的依赖反转为契约依赖（context 依赖 persist，或共享 proto 契约）。

每删一条：`cargo metadata --no-deps` 确认依赖边消失 → 删 allowlist 行 → 同步更新 `macaca-os-serviceization-allowlist.md`。

退出判据：`Cargo.toml` 中 kernel 仅 `macaca-proto`/`macaca-ipc`；kernel 相关 allowlist 行清零；kernel 内无 web3/evm/a2a/payment/executor/provider_compat。
验证：`cargo tree -e normal -p macaca-kernel --depth 1`、boundary gate、kernel 单元测试、payment/web3/evm optional-absent 降级测试。

---

## 6. P3：Web 瘦身为 Thin Shell

目的：web 只解析输入、调 SDK、渲染/订阅事件。

任务：
1. `framework_toolkit` 的 driver/MCP/tool 可见性改为经 `SystemDriverClient`/`SystemSkillClient`/`SystemMcpClient` 的 snapshot 命令（删除 `driver_runtime.collect_tools()`/`mcp_runtime.definitions()` 直读）。
2. 用 focused SDK clients 替换 `AppState` 中 direct provider 字段（runtime/llm/router/memory/mcp/driver）。
3. 剩余 session loop ownership 下沉到 `service.execution_control` + task service；web 仅留 SSE endpoint + HTTP DTO mapping。
4. 巨型文件拆分（按 ownership，非格式化）：`loop_manager.rs`(2629)、`framework_runner.rs`(2484)、`chat_orchestrator.rs`(1581)、`lib.rs`(975) → 均 ≤500 行。
5. 删除 `macaca-web → driver/llm/memory/persist/skill/task/tools` 直接依赖 → 删对应 7 条 allowlist 行。

退出判据：`macaca-web` 仅依赖 `macaca-sdk`(+proto)；web 相关 allowlist 清零；无文件 >500 行。
验证：boundary gate、`/api/chat/v2` 回归、SSE/GenUI 渲染验证、web 单测。

---

## 7. P4：CLI 解耦 + 外置 Domain Pack

任务：
1. CLI run/status 改用 runtime-host bootstrap client 或 SDK status/service inspector，删除 `KernelBuilder::from_service_clients` 等 kernel 构造；`macaca web` 进程启动 seam 移到小型 public bootstrap facade（避免 `macaca-cli → macaca-web` internals）。
2. 删除 `macaca-cli → gateway/tools/web` 直接依赖与 allowlist 行。
3. 将 `runtime-host/domain_pack_service_provider.rs`、`finance_live_data.rs`（Coindesk/Binance/OKX 等业务域）移出 base runtime-host，注册为 plugin/package service provider（带 descriptor + policy metadata）；runtime-host 仅保留 generic `ServiceProviderFactory` 与注册机制。

退出判据：cli 仅依赖 sdk；runtime-host 无业务域 provider；缺失 domain pack 时返回结构化 unavailable。
验证：boundary gate、cli 命令冒烟、domain-pack-absent 降级测试。

---

## 8. P5：Gate 清零 + OpenSpec Baseline 对齐

任务：
1. 确认 `route_c_dependency_boundaries` allowlist = 0；新增 §10（目标设计）的强化 gate（no-direct-provider-call 审计、no-hardcoded-name 审计、shell-not-semantic-owner 审计）。
2. 将已完成 changes 分批 archive 到 `openspec/specs/`，使 baseline 覆盖：service runtime、dependency gate、SDK/SystemFacade、application service、execution control、Web/CLI thin shell、单一 agent execution 路径。
3. 每批后 `openspec validate --strict` + boundary tests。

退出判据：allowlist=0；`openspec list --specs` 反映终态架构；全部 gate 绿。

---

## 9. 横切任务：注释与日志补齐

随每阶段同步执行：
- 拆分后的新模块补详尽英文注释（功能 + 运行原理 + 权衡）。
- 关键节点日志统一为 provider-neutral 维度（service_id/command/trace_id/reason_code），删除以 provider/model/app name 为主键的日志。
- 全局化脱敏（`sanitize_json`/`is_safe_metadata_key`）到所有 audit/trace/snapshot 出口。
- 旁路删除后，确认所有执行都产出 service-call evidence（消除审计盲区）。

## 10. 每阶段统一验证命令

```bash
cd macaca
cargo check
cargo test -p macaca-integration-tests route_c_dependency_boundaries_reject_unallowlisted_forbidden_edges -- --nocapture
cargo test -p macaca-integration-tests serviceization_escape_hatches -- --nocapture
cargo metadata --no-deps --format-version 1   # 确认依赖边变化
cargo test -p macaca-<受影响 crate>            # 目标 service 测试
# 端到端：/api/chat/v2 会话创建/恢复、WASM host import、fullstack-autodev
```

## 11. 回退策略

- 每阶段独立分支 + 独立 OpenSpec change，可单独 revert。
- 删除 kernel 能力前，先确保 optional module/service 版本通过 absent 降级测试，再删 kernel 实现（避免"删了没替身"）。
- 协调补丁删除（P1）以 audit replay 为门：replay 仍显示双链则不删，先收敛路径。

## 12. 完成定义（DoD）

与审计报告 §11 / 目标设计 §12 对齐：单路径达成、协调补丁清零、内核纯净、web thin、allowlist=0、巨型文件达标、OpenSpec baseline 对齐、无审计盲区、无硬编码 application 业务名。
