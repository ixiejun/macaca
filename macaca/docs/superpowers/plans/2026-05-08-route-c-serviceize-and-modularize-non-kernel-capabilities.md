# Route C 非内核能力服务化与模块化实施计划

> **给后续 agentic worker 的要求：** 按本计划实施时必须先创建对应 OpenSpec change，并使用 `superpowers:executing-plans` 或 `superpowers:subagent-driven-development`。每个切片都必须 additive-first、可编译、可回滚。不得一次性重写。

**目标：** 把 `docs/openharmony-microkernel-architecture-for-macaca-agent-os.md` 中的路线 C 从“基础 contract / skeleton”推进到真实服务化、模块化。最终状态是：kernel 只保留系统不变量；Application、Driver、Skill、MCP、Gateway、Memory、LLM、Store、Payment、Web3、EVM 都作为 system service 或可安装 module 存在。

**架构策略：** 先建 service runtime 和 dependency gate，再迁移 provider，再迁移 consumers，最后移除宏内核式依赖。每一步保持现有 YAML application、`/api/chat/v2`、trace、task board、resume、driver、skill/MCP 可用。

**技术栈：** Rust workspace、`macaca-proto` service/package types、`macaca-kernel` primitive facade、`macaca-ipc` service bus、`macaca-runtime-host` module host、`macaca-sdk` system facade、OpenSpec、Route C regression matrix。

---

## 1. 当前诊断

### 1.1 已完成但仍是地基的部分

- `macaca-proto` 已有 kernel/service/package/plugin/commerce/web3/evm/a2a 等类型。
- `macaca-kernel` 已有 `KernelFacade`、`SystemService`、service registry、plugin registry、A2A/Web3/EVM skeleton。
- `macaca-ipc` 已有 local-first service bus 和 trace-required middleware。
- `macaca-app` 已有 package manifest、ABI、GenUI、compatibility checker。
- `macaca-runtime-host` 已有 entitlement/plugin/MCP runtime 基础。
- `macaca-web` 已有部分 shell/facade 化尝试。

### 1.2 仍然不符合 Route C 的部分

- `macaca-kernel` 仍直接依赖 LLM、Memory、Task、Tools、Persist，边界不够微内核。
- `macaca-web` 仍是事实上的系统协调层，直接组装 Application、Task、Agent、Driver、Skill、MCP、Memory、LLM。
- `macaca-sdk` 仍直接依赖 LLM、Tools、Task、Kernel，而不是只暴露稳定系统 facade。
- service contract 多数还没有成为真实调用路径。
- package/module/entitlement 多数还没有接入服务注册、service bus、runtime lifecycle。
- Web3/EVM 还是 proto/mock/skeleton，没有作为 optional module 安装、注册、调用。

## 2. 执行原则

- 每个阶段必须先 OpenSpec。
- 每个阶段必须先 GitNexus impact，再改 symbol。
- 每个阶段必须引用 Route C regression matrix。
- 所有新增代码必须有英文注释解释功能和运行原理。
- Rust 单文件超过 500 行必须拆分。
- 不新增 app/provider/driver/gateway/model/chain hardcode。
- 旧接口不立刻删除，先 deprecated，并给出替代 service path。
- service call 必须 trace + policy + resource/entitlement hooks。

## 3. 总体阶段

| 阶段 | 名称 | 核心结果 |
| --- | --- | --- |
| S0 | 服务化边界审计与依赖门禁 | 明确 forbidden deps，建立 CI/test gate |
| S1 | ServiceRuntime v1 | 统一 service lifecycle、registry、bus、decorator |
| S2 | Kernel 去 provider 依赖 | kernel 只依赖 proto/ipc/primitive contract |
| S3 | SDK/SystemFacade 收敛 | 上层只通过 facade 调 service |
| S4 | Task/Planner/Review 服务化 | PlanLoop/WorkerLoop/Review 迁出 Web，成为 Task Service |
| S5 | LLM/Memory/Context 服务化 | agent 构建不再直接拿 provider/backend |
| S6 | Driver/Skill/MCP 服务化与模块化 | built-in + plugin provider 同一模型 |
| S7 | Application Framework 服务化 | YAML/WASM/GenUI 应用通过 Application Service 生命周期运行 |
| S8 | Gateway 服务化 | 外部入口是 gateway provider/plugin，不写死在 Web |
| S9 | Store/Entitlement 服务化 | package install/start/call 受 store service 管理 |
| S10 | Payment/A2A 服务化 | quote/intent/receipt 走 payment service |
| S11 | Web3/EVM optional module 真实化 | 可安装、可缺失、可禁用、可 trace |
| S12 | Web/CLI thin shell 完成 | Web/CLI 只做 adapter、renderer、approval surface |
| S13 | 生态认证与硬化 | 第三方应用/插件/模块不改源码即可接入 |

---

## 4. S0：服务化边界审计与依赖门禁

### 目标

先阻止宏内核继续扩张。建立可执行的依赖边界测试，允许短期 migration allowlist，但每个例外必须有迁移阶段。

### 设计模式

- Specification：用规则描述哪些 crate 允许依赖哪些层。
- Visitor：遍历 Cargo metadata dependency graph。

### 涉及文件

- 新增：`macaca/crates/macaca-integration-tests/tests/route_c_dependency_boundaries.rs`
- 新增：`macaca/docs/route-c-serviceization-allowlist.md`
- 修改：`macaca/docs/route-c-architecture-governance.md`

### 实施步骤

1. 定义 crate 分层：
   - `proto`
   - `kernel`
   - `service-contract`
   - `service-provider`
   - `runtime-host`
   - `application-framework`
   - `presentation-shell`
   - `optional-module`

2. 写 dependency boundary test：
   - `macaca-kernel` 不得依赖 provider crate。
   - `macaca-web` 不得新增 provider construction。
   - optional module 不得成为 base OS 必需依赖。

3. 建 migration allowlist：
   - 记录当前仍存在的 direct deps。
   - 每条写明迁移阶段和替代 service。

### 验证

```bash
cargo test -p macaca-integration-tests route_c_dependency_boundaries
cargo metadata --no-deps --format-version 1
```

### 里程碑

- forbidden dependency gate 可运行。
- 当前违规项被记录为短期 allowlist，而不是默认为合理架构。

---

## 5. S1：ServiceRuntime v1

### 目标

把阶段 2/3 的 service contract 和 service bus 连接成真实 runtime。所有服务都能注册、启动、调用、停止、trace、policy-check。

### 设计模式

- Facade：`ServiceRuntime`
- Decorator：trace/policy/resource/entitlement wrapper
- Bridge：runtime 与 local/remote/plugin transport 解耦
- State：service lifecycle
- Abstract Factory：provider factory

### 涉及文件

- 新增：`macaca/crates/macaca-runtime-host/src/service_runtime.rs`
- 新增：`macaca/crates/macaca-runtime-host/src/service_provider.rs`
- 新增：`macaca/crates/macaca-runtime-host/src/service_decorator.rs`
- 修改：`macaca/crates/macaca-runtime-host/src/lib.rs`
- 修改：`macaca/crates/macaca-kernel/src/service_registry.rs`
- 新增测试：`macaca/crates/macaca-runtime-host/tests/service_runtime.rs`

### 实施步骤

1. 新增 `ServiceRuntime`：
   - register provider
   - start provider
   - call provider through service bus
   - stop provider
   - snapshot health

2. 新增 decorator chain：
   - trace required
   - policy required
   - resource lock optional
   - entitlement optional
   - metering optional

3. 新增 built-in provider factory trait：
   - 不绑定具体 LLM/Driver/Skill 名称。
   - provider 只通过 descriptor/capability 暴露。

4. 接入现有 `ServiceBus`：
   - local typed-first。
   - remote/plugin transport 只保留扩展点。

### 验证

```bash
cargo test -p macaca-runtime-host service_runtime
cargo test -p macaca-ipc service_bus
cargo test -p macaca-kernel system_service
```

### 里程碑

- mock service 通过 runtime 完整 start/call/stop。
- 无 trace/policy 的 call 被拒绝。
- runtime snapshot 能显示 lifecycle/health。

---

## 6. S2：Kernel 去 provider 依赖

### 目标

把 `macaca-kernel` 从 provider 容器收敛为真正微内核。Kernel 不再直接依赖 LLM、Memory、Tools、Task provider 实现。

### 设计模式

- Dependency Inversion：kernel 只依赖抽象 contract。
- Facade：kernel 只暴露 primitive facade。
- Adapter：旧 kernel API 通过 adapter 转调 service runtime。

### 涉及文件

- 修改：`macaca/crates/macaca-kernel/Cargo.toml`
- 修改：`macaca/crates/macaca-kernel/src/kernel.rs`
- 修改：`macaca/crates/macaca-kernel/src/kernel_builder.rs`
- 修改：`macaca/crates/macaca-kernel/src/services.rs`
- 修改：`macaca/crates/macaca-kernel/src/executor/*`
- 新增：`macaca/crates/macaca-kernel/src/provider_compat.rs`

### 实施步骤

1. 标记 direct provider builder deprecated：
   - kernel 中直接接受 `LlmProvider`、`ToolCatalog`、Memory backend 的入口标记 deprecated。

2. 新增 compat adapter：
   - 保留旧 API 行为，但内部转为 `ServiceRuntime` 或 `SystemFacade`。

3. 移除 kernel Cargo direct provider deps：
   - 先迁出 `macaca-llm`
   - 再迁出 `macaca-tools`
   - 再迁出 `macaca-memory`
   - 最后处理 `macaca-task` 和 `macaca-persist` 的 primitive/service 边界。

4. 添加 dependency gate：
   - `macaca-kernel` 不能重新引入 provider deps。

### 验证

```bash
cargo check -p macaca-kernel
cargo test -p macaca-kernel
cargo test -p macaca-integration-tests route_c_dependency_boundaries
```

### 里程碑

- kernel provider direct deps 数量归零，或只剩 allowlist 明确的短期项。
- 现有 executor 流程不退化。

---

## 7. S3：SDK/SystemFacade 收敛

### 目标

让上层 application、web、cli、gateway 都通过 SDK/SystemFacade 调用系统能力，而不是直接拿 crate 实现。

### 设计模式

- Facade：`SystemFacade`
- Command：所有入口转换为 system command
- Adapter：HTTP/CLI/Gateway/UI 只是 adapter

### 涉及文件

- 修改：`macaca/crates/macaca-sdk/src/system_facade.rs`
- 新增：`macaca/crates/macaca-sdk/src/service_client.rs`
- 新增：`macaca/crates/macaca-sdk/src/task_client.rs`
- 新增：`macaca/crates/macaca-sdk/src/trace_client.rs`
- 新增：`macaca/crates/macaca-sdk/src/package_client.rs`
- 修改：`macaca/crates/macaca-web/src/shell.rs`
- 修改：`macaca/crates/macaca-cli`

### 实施步骤

1. 扩展 `SystemFacade`：
   - service query/call
   - task query/create/review
   - trace subscribe/replay
   - package install/start/status

2. Web route 迁移：
   - 先迁 task board/session events。
   - 再迁 trace/SSE。
   - 再迁 app/session lifecycle。

3. CLI 迁移：
   - app list
   - session inspect
   - trace tail
   - service inspect

### 验证

```bash
cargo test -p macaca-sdk
cargo test -p macaca-web
cargo check -p macaca-cli
```

---

## 8. S4：Task/Planner/Review 服务化

### 目标

PlanLoop、WorkerLoop、review、goal completion、coordinator resume 不再由 `macaca-web::loop_manager` 直接拥有，而是 Task Service 的生命周期和事件。

### 设计模式

- Mediator：Task Service 协调 goal、planner、worker、review、resume。
- State：goal/task/review lifecycle。
- Observer：task events -> trace/event log。
- Strategy：planner/reviewer/worker assignment policy 可替换。

### 涉及文件

- 修改：`macaca/crates/macaca-task/src/*`
- 新增：`macaca/crates/macaca-task/src/service.rs`
- 新增：`macaca/crates/macaca-task/src/runtime.rs`
- 新增：`macaca/crates/macaca-task/src/events.rs`
- 修改：`macaca/crates/macaca-web/src/loop_manager.rs`
- 修改：`macaca/crates/macaca-web/src/framework_toolkit.rs`

### 实施步骤

1. 在 `macaca-task` 新增 `TaskServiceProvider`。
2. 把 TaskSpace/TaskBoard/PlanLoop/WorkerLoop 的 start/stop/snapshot 放入 service runtime。
3. Web 只发送 `CreateGoal` / `QueryTaskBoard` / `SubscribeTaskEvents` command。
4. 旧 `loop_manager` 缩减为 adapter，最终 deprecated。

### 验证

```bash
cargo test -p macaca-task
cargo test -p macaca-web task
cargo test -p macaca-integration-tests route_c_baseline
```

### 回归场景

- RC-GOAL-001
- RC-TRACE-001
- RC-TASK-001

---

## 9. S5：LLM / Memory / Context 服务化

### 目标

Agent 构建、planner、reviewer、worker 不再直接持有 LLM provider 和 Memory backend，而是通过 LLM Service / Memory Service / Context Service 调用。

### 设计模式

- Strategy：model/provider routing。
- Adapter：现有 provider/backend 适配为 service provider。
- Decorator：token usage、trace、policy、context budget。
- Memento：memory/context snapshot。

### 涉及文件

- 新增：`macaca/crates/macaca-llm/src/service_adapter.rs`
- 新增：`macaca/crates/macaca-memory/src/service_adapter.rs`
- 新增：`macaca/crates/macaca-context/src/service_adapter.rs`
- 修改：`macaca/crates/macaca-web/src/framework_runner.rs`
- 修改：`macaca/crates/macaca-agent`
- 修改：`macaca/crates/macaca-framework`

### 实施步骤

1. LLM provider adapter：
   - chat/completion/model selection 走 service command。

2. Memory service adapter：
   - remember/recall/forget/digest 走 service command。

3. Context service adapter：
   - active recall、workspace digest、session context injection 走 service。

4. Agent builder 迁移：
   - 不再直接传 provider/backend。
   - 改传 service client。

### 验证

```bash
cargo test -p macaca-llm
cargo test -p macaca-memory
cargo test -p macaca-context
cargo test -p macaca-web framework_runner
```

---

## 10. S6：Driver / Skill / MCP 服务化与模块化

### 目标

Driver、Skill、MCP 都成为可安装 capability provider。内置 Claude Code/OpenCode/Playwright/Skill-backed MCP 只是 built-in provider，不是特例路径。

### 设计模式

- Adapter：内置 driver/skill/MCP 转 service provider。
- Abstract Factory：按 manifest/runtime kind 创建 provider。
- Resource Manager / Mediator：浏览器、driver process、workspace lock。
- State：driver/MCP session lifecycle。
- Null Object：缺失 driver/MCP 返回 unavailable。

### 涉及文件

- 修改：`macaca/crates/macaca-driver/src/*`
- 修改：`macaca/crates/macaca-skill/src/*`
- 修改：`macaca/crates/macaca-runtime-host/src/mcp*`
- 修改：`macaca/crates/macaca-web/src/framework_toolkit.rs`
- 修改：`macaca/crates/macaca-web/src/skill_mcp.rs`

### 实施步骤

1. Driver Service：
   - execute/resume/status/cancel/cleanup 走 service command。
   - resource lock 由 resource manager 管。

2. Skill Service：
   - skill discovery/install/load/execute 走 service command。
   - encrypted skill 走 entitlement + decrypt hook。

3. MCP Service：
   - MCP server lifecycle 独立为 module/provider。
   - 多实例 browser/MCP 通过 resource scope 隔离。

4. Web/agent toolkit 迁移：
   - allowed tools 解析为 capability request。
   - tool invocation 走 service client。

### 验证

```bash
cargo test -p macaca-driver
cargo test -p macaca-skill
cargo test -p macaca-runtime-host mcp
cargo test -p macaca-integration-tests package_certification
```

### 回归场景

- RC-DRIVER-001
- RC-SKILL-001
- RC-TRACE-001

---

## 11. S7：Application Framework 服务化

### 目标

Application 不再由 Web 直接加载和解释。YAML app、WASM app、GenUI app、headless app 都通过 Application Service 统一生命周期运行。

### 设计模式

- Adapter：YAML/WASM/Hybrid app 适配同一 ABI。
- State：application lifecycle。
- Facade：ApplicationHost。
- Specification：manifest/permission/compatibility validation。

### 涉及文件

- 修改：`macaca/crates/macaca-app/src/runtime.rs`
- 修改：`macaca/crates/macaca-app/src/abi.rs`
- 新增：`macaca/crates/macaca-app/src/service.rs`
- 修改：`macaca/crates/macaca-web/src/chat_orchestrator.rs`
- 修改：`macaca/crates/macaca-web/src/routes.rs`

### 实施步骤

1. 新增 `ApplicationServiceProvider`。
2. YAML app loader 改为 Application Service provider。
3. WASM package 继续 metadata-only，但注册为 unavailable execution runtime。
4. GenUI render/output 通过 ApplicationHost 走 service call。
5. Web 只通过 Application Service start/resume/session command。

### 验证

```bash
cargo test -p macaca-app application_abi
cargo test -p macaca-app package_manifest
cargo test -p macaca-web chat
```

---

## 12. S8：Gateway 服务化

### 目标

Discord、Telegram、飞书、钉钉、WhatsApp、Email 等入口都应是 gateway service/plugin。Web/CLI 不再作为唯一入口语义。

### 设计模式

- Adapter：不同外部平台适配统一 Gateway Service。
- Command：外部消息转换为 system command。
- Strategy：routing/identity/session mapping。
- Observer：gateway event trace。

### 涉及文件

- 修改：`macaca/crates/macaca-gateway/src/*`
- 新增：`macaca/crates/macaca-gateway/src/service_adapter.rs`
- 修改：`macaca/crates/macaca-runtime-host/src/plugin_builtin.rs`
- 修改：`macaca/crates/macaca-web/src/routes.rs`

### 实施步骤

1. Gateway provider descriptor。
2. Gateway event -> SystemFacade command。
3. Gateway session mapping 不写死平台。
4. Gateway plugin install/register/start/stop。

### 验证

```bash
cargo test -p macaca-gateway
cargo test -p macaca-runtime-host plugin_runtime
```

---

## 13. S9：Store / Entitlement 服务化

### 目标

Store/Entitlement 从 runtime helper 升级为系统服务，统一管理 package source、签名、license、subscription、metering、encrypted package。

### 设计模式

- Facade：Store Service / Entitlement Service。
- Chain of Responsibility：signature -> compatibility -> entitlement -> metering。
- Strategy：license/metering/provider policy。
- Proxy：付费能力可远程执行。

### 涉及文件

- 未来新增：`macaca/crates/macaca-store`
- 修改：`macaca/crates/macaca-runtime-host/src/entitlement.rs`
- 修改：`macaca/crates/macaca-persist/src/entitlement_store.rs`
- 修改：`macaca/crates/macaca-app/src/commercial_package.rs`
- 修改：`macaca/crates/macaca-skill/src/encrypted_package.rs`

### 实施步骤

1. Store Service descriptor。
2. Entitlement Service provider。
3. Package install/start/call guard 统一调用 entitlement service。
4. Metering event 进入 trace/audit/payment store。
5. Web/CLI package manager 只调用 Store Service。

### 验证

```bash
cargo test -p macaca-runtime-host entitlement
cargo test -p macaca-persist entitlement
cargo test -p macaca-app commercial_package
cargo test -p macaca-skill encrypted_package
```

---

## 14. S10：Payment / A2A 服务化

### 目标

A2A quote、payment intent、budget、approval、receipt 不再只是 kernel helper，而是 Payment Service。Kernel 只保留 policy primitive。

### 设计模式

- Mediator：A2A coordinator。
- Strategy：payment adapter。
- Command：quote/intent/settlement。
- State：payment lifecycle。
- Memento：receipt/proof。

### 涉及文件

- 修改：`macaca/crates/macaca-kernel/src/a2a.rs`
- 修改：`macaca/crates/macaca-kernel/src/payment_policy.rs`
- 新增：`macaca/crates/macaca-runtime-host/src/payment_service.rs`
- 修改：`macaca/crates/macaca-persist/src/payment_store.rs`

### 实施步骤

1. 把 payment adapter 从 kernel implementation 迁出。
2. Payment Service 提供 quote/create_intent/approve/settle/receipt。
3. Budget/approval policy 保留为 kernel facade。
4. 所有 payment 状态转移进入 trace/audit。

### 验证

```bash
cargo test -p macaca-kernel a2a_payment
cargo test -p macaca-persist payment_store
cargo test -p macaca-runtime-host payment_service
```

---

## 15. S11：Web3 / EVM optional module 真实化

### 目标

Web3/EVM 不再只是 proto/mock，而是可选 module。未安装时 base OS 正常；安装后注册 wallet/signing/transaction/chain_query/evm service。

### 设计模式

- Null Object：unavailable provider。
- Adapter：Substrate/Frontier/RPC provider。
- Strategy：signing/gas/network policy。
- Proxy：本地节点或远程 RPC。
- Observer：transaction/contract event trace。

### 涉及文件

- 未来新增：`macaca/crates/macaca-web3`
- 未来新增：`macaca/crates/macaca-evm`
- 修改：`macaca/crates/macaca-runtime-host/src/service_runtime.rs`
- 修改：`macaca/crates/macaca-app`
- 修改：`macaca/crates/macaca-sdk`

### 实施步骤

1. Unavailable Web3/EVM provider 默认注册。
2. Web3 module package 安装后替换 provider。
3. Wallet/signing/transaction commands 全部 policy-check。
4. EVM deploy/call/read/subscribe 通过 Web3 module service。
5. Region/compliance disabled 返回 policy denied。

### 验证

```bash
cargo test -p macaca-proto web3
cargo test -p macaca-proto evm
cargo test -p macaca-web3
cargo test -p macaca-evm
cargo check --workspace
```

---

## 16. S12：Web / CLI Thin Shell 完成

### 目标

`macaca-web` 和 `macaca-cli` 只保留表现层/入口适配职责，不再拥有核心 session/task/trace/package/payment/service 语义。

### 设计模式

- Adapter：HTTP/CLI/SSE/UI。
- Facade：SystemFacade。
- Observer：trace subscription。
- Visitor：GenUI renderer。

### 涉及文件

- 修改：`macaca/crates/macaca-web/src/*`
- 修改：`macaca/crates/macaca-cli/src/*`
- 修改：`frontend/app`
- 修改：`frontend/components`

### 实施步骤

1. Web state 去 provider ownership：
   - AppState 不再直接持有 driver_runtime/skill_catalog/mcp_runtime/memory_backend/llm_router。
   - 改持有 `SystemFacade` 或 `ServiceRuntimeHandle`。

2. Route adapter 化：
   - chat/session/task/trace/package/service routes 都转 command。

3. SSE/trace 薄化：
   - Web 只订阅 Trace Service。
   - 去除重复 forwarder 和重复 event source。

4. Frontend：
   - chat shell 保留。
   - GenUI shell mount 可渲染 application UI。
   - trace 历史恢复和实时增量使用同一 dedupe key。

### 验证

```bash
cargo test -p macaca-web
cargo check -p macaca-cli
cd frontend && npm run lint && npx tsc --noEmit
```

### 回归场景

- RC-CHAT-001
- RC-CHAT-002
- RC-TRACE-001
- RC-TRACE-002
- RC-TASK-001

---

## 17. S13：生态认证与硬化

### 目标

让第三方无需修改 Macaca 源码即可开发、打包、安装、运行、trace、调试、发布 application/plugin/skill/MCP/driver/module。

### 设计模式

- Specification：certification rules。
- Template Method：每类 package 使用统一 certification flow。
- Builder：开发者模板。
- Visitor：manifest/ABI/service graph checker。

### 涉及文件

- 修改：`macaca/crates/macaca-app/src/compatibility_checker.rs`
- 新增：`macaca/crates/macaca-integration-tests/tests/service_module_certification.rs`
- 修改：`macaca/docs/developer/*`
- 新增：`macaca/crates/macaca-sdk/examples/*`

### 实施步骤

1. Certification 扩展到 service/module 层。
2. SDK examples 覆盖：
   - YAML app
   - WASM stub app
   - GenUI app
   - gateway plugin
   - driver plugin
   - skill package
   - MCP module
   - paid package
   - Web3 optional app
   - EVM optional DApp

3. Compatibility checker 检查：
   - manifest
   - ABI version
   - required/optional services
   - permissions
   - entitlement
   - service trace schema
   - lifecycle hooks

### 验证

```bash
cargo test -p macaca-app compatibility_checker
cargo test -p macaca-integration-tests package_certification
cargo test -p macaca-integration-tests service_module_certification
```

---

## 18. 全局验收

每完成一个阶段，都必须跑：

```bash
cargo check --workspace
cargo test -p macaca-integration-tests route_c_baseline
cargo test -p macaca-integration-tests route_c_dependency_boundaries
```

每完成一个 service/module 迁移，都必须验证：

- direct path 已 deprecated。
- consumer 已迁到 service path。
- service call 有 trace。
- service call 有 policy。
- optional module 缺失返回 structured unavailable。
- Web UI 实时 trace 不重复、不丢失。
- session 历史恢复完整。

## 19. 首个建议 OpenSpec

下一步应先写并实现：

```text
add-route-c-service-runtime-and-boundary-gates
```

原因：

- 没有 service runtime，后续每个服务迁移都会各自发明 runtime。
- 没有 dependency gate，kernel/web 会继续被新能力污染。
- 这是后续所有真实服务化的前置条件。

该 OpenSpec 必须覆盖：

- `ServiceRuntime v1`
- provider factory
- decorator chain
- dependency boundary test
- migration allowlist
- mock service runtime tests

暂时不要先迁 LLM/Task/Driver。先把 runtime 和门禁落稳。
