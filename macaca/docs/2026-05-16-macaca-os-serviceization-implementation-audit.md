# Macaca OS 服务化实现审计

日期：2026-05-16

## 审计依据

本次审计以以下三份稳定治理文档作为设计基线：

- `macaca/docs/macaca-os-architecture-governance.md`
- `macaca/docs/macaca-os-microkernel-boundaries.md`
- `macaca/docs/macaca-os-serviceization-allowlist.md`

关键设计要求如下：

- Kernel 只拥有 identity、registry、policy facade、scheduler/resource/session/task primitives、trace/audit，以及 service-call evidence。
- 可替换能力必须通过 system service、plugin、application-framework capability 或 optional module 进入系统。
- Service 必须具备 descriptor、lifecycle、health、snapshot 或 diagnostics、typed command/result、强制 trace 的调用、side effect 前的 policy、脱敏 audit、结构化 unavailable/unsupported/denied/failure 状态，以及 provider 替换机制。
- SDK/SystemFacade 和 focused clients 是上层 facade；SDK 不得构造 provider 或 runtime-host 内部对象。
- Web、CLI、gateway、frontend 只能是 presentation shell。
- Optional module 可以缺席，缺席时不得 crash、hang、silent fallback 或 fake success。
- Generic OS 代码不得基于 application、workflow、provider、model、driver、gateway、chain、payment 或业务域名称分支。

## 审计方法

执行过的命令：

- `openspec list`
- `openspec list --specs`
- `cargo test -p macaca-integration-tests route_c_dependency_boundaries_reject_unallowlisted_forbidden_edges -- --nocapture`
- `cargo metadata --no-deps --format-version 1`
- `cargo tree -e normal -p macaca-kernel --depth 1`
- `cargo tree -e normal -p macaca-web --depth 1`
- `cargo tree -e normal -p macaca-cli --depth 1`
- 针对 deprecated direct paths、allowlist rows、硬编码 provider/app/domain 字符串、shell-owned runtime fields、service boundary code 的定向 `rg` 扫描。

同时使用 GitNexus 查询了 Route C dependency boundary 和 serviceized call flow。GitNexus 定位到了 dependency gate 实现，以及相关的 service runtime、WASM host-import service-call 测试。

## 总结

项目已经朝设计目标推进了不少：service contract 已存在，`ServiceRuntime` 和 `ServiceRouter` 在 canonical `service.call` 路径上执行 trace/policy/audit，多个 service family 已有 focused SDK client，可执行 dependency boundary gate 也能阻止新增的未登记 forbidden crate edge。

但当前实现尚未满足稳定设计。核心差距是：大量违规项仍被有意保留为 compatibility path。这些路径已有文档记录，很多也标记为 deprecated，但治理文档将它们定义为迁移债务，不是可接受的最终架构。

## 发现 P0-1：Dependency Gate 通过依赖 16 条 Allowlist 边界违规

状态：不满足最终设计；只满足迁移债务可追踪要求。

证据：

- `cargo test -p macaca-integration-tests route_c_dependency_boundaries_reject_unallowlisted_forbidden_edges -- --nocapture` 通过。
- 测试报告 113 条直接 workspace edge，其中 16 条 forbidden edge 被 allowlist 放行。
- `macaca/crates/tests/macaca-integration-tests/tests/route_c_dependency_boundaries/allowlist.rs` 包含：
  - `macaca-kernel -> macaca-driver`
  - `macaca-kernel -> macaca-gateway`
  - `macaca-kernel -> macaca-persist`
  - `macaca-kernel -> macaca-skill`
  - `macaca-kernel -> macaca-task`
  - `macaca-kernel -> macaca-tools`
  - `macaca-cli -> macaca-gateway`
  - `macaca-cli -> macaca-tools`
  - `macaca-cli -> macaca-web`
  - `macaca-web -> macaca-driver`
  - `macaca-web -> macaca-llm`
  - `macaca-web -> macaca-memory`
  - `macaca-web -> macaca-persist`
  - `macaca-web -> macaca-skill`
  - `macaca-web -> macaca-task`
  - `macaca-web -> macaca-tools`
- `cargo tree -e normal -p macaca-kernel --depth 1` 证实 production dependency 中 `macaca-kernel` 仍依赖 `macaca-persist`、`macaca-task`、`macaca-tools`。
- `cargo tree -e normal -p macaca-web --depth 1` 证实 Web 仍直接依赖 provider/service crates：`macaca-driver`、`macaca-llm`、`macaca-memory`、`macaca-persist`、`macaca-skill`、`macaca-task`、`macaca-tools`。
- `cargo tree -e normal -p macaca-cli --depth 1` 证实 CLI 仍直接依赖 `macaca-gateway`、`macaca-tools`、`macaca-web`。

为什么违反设计：

- `macaca-os-serviceization-allowlist.md` 明确要求 kernel 不得依赖 concrete provider implementation，presentation shell 不得成为 provider construction hub，CLI 不得依赖 Web internals。
- 当前 gate 能阻止新增未记录债务，但已记录的 allowlist 行仍是实现债务。

重构建议：

1. 将 allowlist 拆成按阶段归属的债务轨道：kernel provider debt、Web provider-construction debt、CLI shell coupling debt。
2. 每一行都补充当前具体 caller path 和目标 service client replacement。
3. 只有当 `cargo metadata` 证明直接依赖边已消失后，才删除对应 allowlist 行。
4. 优先处理 `macaca-kernel -> macaca-task/tools/persist`，其次处理 `macaca-web -> llm/memory/task/tools`，最后处理 `macaca-cli -> macaca-web`。

## 发现 P0-2：Kernel 仍持有 Legacy Provider Compatibility 路径

状态：不满足最终 microkernel boundary。

证据：

- `macaca/crates/kernel/macaca-kernel/src/provider_compat.rs:1` 明确说明这是 legacy kernel provider construction 的临时 compatibility boundary。
- `macaca/crates/kernel/macaca-kernel/src/provider_compat.rs:20` 定义了 deprecated `KernelProviderCompat`。
- `macaca/crates/kernel/macaca-kernel/src/kernel.rs:18` 保存了 `providers: KernelProviderCompat`。
- `macaca/crates/kernel/macaca-kernel/src/kernel.rs:78` 仍调用 `entry.agent.run(self.providers.llm(), self.providers.tools(), &services).await`。
- 构建警告显示 deprecated kernel Web3/EVM/A2A facade 和 provider adapter 仍被导出和使用。

为什么违反设计：

- Kernel 仍可以把直接 LLM/tool provider handle 带入 agent execution。
- 稳定治理文档要求通过 service/facade 调用，并具备 trace、policy、结构化 unavailable 行为和 provider replacement 机制。

重构建议：

1. 引入 kernel-owned `AgentExecutionPort` 或 service-client-only execution adapter，只接收 typed service client handle，不接收 provider trait。
2. 将 legacy `Agent::run(llm, tools, services)` 执行路径放到 runtime-host 或 application/framework 层的显式 service provider 后面。
3. 只在 test 或 migration module 中保留 `KernelProviderCompat`，然后移除 production constructor path。
4. 让 kernel agent execution 产出 service-call evidence，而不是 provider-name log。

## 发现 P0-3：Web 仍是 Execution、Tools、Sessions 和 Provider Runtime State 的语义所有者

状态：不满足最终 thin-shell 状态。

证据：

- 过大的 shell 文件：
  - `macaca/crates/shells/macaca-web/src/framework_runner.rs`：2484 行。
  - `macaca/crates/shells/macaca-web/src/loop_manager.rs`：2629 行。
  - `macaca/crates/shells/macaca-web/src/chat_orchestrator.rs`：1581 行。
  - `macaca/crates/shells/macaca-web/src/lib.rs`：975 行。
- `macaca/crates/shells/macaca-web/src/state.rs:292` 仍保留 deprecated application runtime 和 registry anchor。
- `macaca/crates/shells/macaca-web/src/state.rs:346` 仍保留 deprecated direct LLM provider/router field。
- `macaca/crates/shells/macaca-web/src/state.rs:360` 仍保留 deprecated memory runtime。
- `macaca/crates/shells/macaca-web/src/state.rs:367` 仍保留 deprecated MCP runtime。
- `macaca/crates/shells/macaca-web/src/state.rs:370` 和 `:373` 仍保留 deprecated driver registry/runtime。
- `macaca/crates/shells/macaca-web/src/framework_toolkit.rs:107` 仍通过 `state.driver_runtime.collect_tools().await` 收集 driver tools。
- `macaca/crates/shells/macaca-web/src/framework_toolkit.rs:274` 仍通过 `state.mcp_runtime.definitions().await` 读取 MCP definitions。

为什么违反设计：

- Web 仍不只是 shell：它还拥有 toolkit composition、session loop plumbing、MCP/driver runtime exposure 和 compatibility semantics。
- 设计只允许 Web 解析输入、渲染输出、订阅事件，并调用 `SystemFacade` 或 focused clients。

重构建议：

1. 将 `framework_toolkit` 中 driver/MCP/tool visibility 改为通过 `SystemDriverClient`、`SystemSkillClient`、`SystemMcpClient` 的 snapshot command 获取。
2. 将剩余 session loop ownership 下沉到 execution-control/task services，Web 只保留 SSE channel endpoint 和 HTTP DTO mapping。
3. 用小型 shell composition bundle 加 focused SDK clients 替换 Web `AppState` 中的 direct provider fields。
4. 增加 boundary test：如果新的 Web production code 在显式 migration module 之外引用 deprecated direct fields，则失败。

## 发现 P1-1：CLI 仍构造 Compatibility Kernel 且依赖 Web

状态：部分满足；仍是迁移债务。

证据：

- `macaca/crates/shells/macaca-cli/src/commands.rs:5` 导入 `macaca_agent::LlmProvider`。
- `macaca/crates/shells/macaca-cli/src/commands.rs:8` 导入 `macaca_gateway::GatewayBuilder`。
- `macaca/crates/shells/macaca-cli/src/commands.rs:9` 导入 `macaca_kernel::{Kernel, KernelBuilder, KernelServiceClientCompat}`。
- `macaca/crates/shells/macaca-cli/src/commands.rs:24` 定义 `CliUnavailableLlmProvider`。
- `macaca/crates/shells/macaca-cli/src/commands.rs:270` 通过 `KernelBuilder::from_service_clients(...)` 构建 kernel。
- `macaca/crates/shells/macaca-cli/src/command_handlers.rs:104` 调用 `macaca_web::WebServerBuilder::new().port(...).serve().await`。

为什么违反设计：

- CLI 仍为 status/run flow 直接组合 kernel/gateway/tool compatibility。
- CLI 仍依赖 Web crate 做 server startup。虽然这已记录为 S12 compatibility debt，但最终 shell boundary 要求 CLI 是 terminal adapter，而不是 Web-internal consumer。

重构建议：

1. 用 runtime-host bootstrap client 或 SDK status/service inspector client 替换 CLI run/status 的 kernel construction。
2. 保持 `macaca web` 只负责进程启动，但将共享 server-start contract 移到一个很小的 public bootstrap facade 或 binary-only entrypoint facade，避免 CLI 依赖 `macaca-web` internals。
3. 当直接依赖边消失后，删除 `macaca-cli -> macaca-gateway/tools/web` allowlist rows。

## 发现 P1-2：Application Lifecycle 已服务化，但 Direct `AppRuntime` Start 路径仍存在

状态：部分满足。

证据：

- 构建警告显示 deprecated `AppRuntime::start_app` 和 `start_app_from_file` 仍被 integration tests 和 compatibility paths 使用。
- `macaca/crates/application/macaca-app/src/runtime.rs:98` 和 `:115` 将 direct start methods 标记为 deprecated，替代路径是 Application Service start commands。
- `macaca/crates/shells/macaca-web/src/state.rs:292` 仍携带 direct `runtime: Arc<AppRuntime>` 作为 deprecated compatibility anchor。

为什么违反设计：

- Application lifecycle 必须由 application framework service boundary 拥有，并通过 typed commands、trace、policy 和结构化 unavailable 行为执行。
- Direct runtime start path 让新调用方可以轻易绕过 service admission。

重构建议：

1. 先将 tests 和 shell helpers 改到 Application Service start/snapshot commands。
2. 增加 static audit test，禁止 production caller 在 service provider 和 compatibility tests 之外调用 `start_app` 与 `start_app_from_file`。
3. 所有 route 和 startup path 都使用 `application_client` 后，移除 `AppState::runtime`。

## 发现 P1-3：Domain-Specific Finance/Crypto Services 位于 Runtime Host

状态：设计风险；尚未完全通用化。

证据：

- `macaca/crates/runtime/macaca-runtime-host/src/domain_pack_service_provider.rs:1` 实现了 built-in finance domain-pack services。
- 它声明固定 service id：`service.market_data`、`service.financials`、`service.news_digest`、`service.llm.analysis`。
- `macaca/crates/runtime/macaca-runtime-host/src/domain_pack_service_provider.rs:112` 基于 `asset_class == "crypto"` 分支。
- `macaca/crates/runtime/macaca-runtime-host/src/finance_live_data.rs:14` 硬编码 Coindesk RSS URL。
- `macaca/crates/runtime/macaca-runtime-host/src/finance_live_data.rs:123` 和 `:160` 转换 Binance/OKX exchange payload。

为什么违反或压迫设计：

- 治理文档允许 service/provider layer 中存在 provider adapter，但也警告 generic OS/runtime-host 代码不应拥有具体业务域行为。
- 该实现被包装为 domain pack，但仍编译进 runtime-host，而 runtime-host 是 base runtime composition crate。

重构建议：

1. 将 finance/crypto domain-pack implementations 移到 plugin/package provider boundary 后面。
2. runtime-host 只保留 generic `ServiceProviderFactory` 和 registration mechanics。
3. 要求 domain-pack packages 通过 manifest 声明 service descriptors 和 provider metadata。
4. 将 deterministic local fixtures 保留在测试中，不作为 base runtime-host 行为。

## 发现 P1-4：Provider 和 Model Name Routing 仍存在于 Service Code

状态：因为位于 service code 中，部分可接受；但不是最终 provider-neutral policy。

证据：

- `macaca/crates/services/macaca-llm/src/router.rs:48` 记录了 built-in model-prefix routing。
- `macaca/crates/services/macaca-llm/src/router.rs:126` 基于 provider name `"openai"`、`"anthropic"`、`"dashscope"` 分支。
- `macaca/crates/services/macaca-llm/src/router.rs:275` 映射 `gpt-*`、`o1*`、`o3*` 和 `claude-*`。
- `macaca/crates/services/macaca-llm/src/resolver.rs` 包含 provider/model prefix rules。

为什么这是设计差距：

- LLM service 是正确所有权层，所以这不是 kernel/shell violation。
- 但稳定设计要求 provider choice、routing、model policy、budget 和 degradation 是可替换 strategy/config，而不是固定代码分支。

重构建议：

1. 将 built-in provider/model mappings 移入 config 或 provider descriptors。
2. 将 prefix resolution 作为 LLM Service 加载的默认 `ResolverChain` plugin/strategy。
3. 增加 audit test：kernel/Web/CLI 代码不得基于 provider/model name 分支；另加 service-level test 证明 resolver behavior 由 descriptor 驱动。

## 发现 P1-5：Service 具备核心 Contract，但并非所有 Capability Path 都统一强制执行

状态：部分满足。

证据：

- 正向证据：`macaca/crates/foundation/macaca-proto/src/service.rs` 定义了 `ServiceDescriptor`、`ServiceCommand`、`ServiceCallResult` 和结构化 `ServiceError`。
- 正向证据：`macaca/crates/kernel/macaca-kernel/src/system_service.rs` 定义了带 descriptor/start/call/stop/cleanup/health 的 `SystemService`。
- 正向证据：`macaca/crates/runtime/macaca-runtime-host/src/service_router.rs:75` 通过 contract、policy、runtime dispatch、retry、timeout 和 audit 进行路由。
- 差距：该 router 之外仍存在 direct compatibility calls，包括 kernel `Agent::run(...)`、Web driver/MCP runtime access、direct Application Runtime start paths，以及 deprecated memory/task APIs。

为什么违反设计：

- 设计条件不是“service runtime 已存在”即可。每个 capability call 都必须经过 trace、policy、structured errors、audit 和 replacement mechanics。
- 任何 direct path 都是逃逸口。

重构建议：

1. 为每个已服务化 capability 定义 canonical “no direct provider call” audit。
2. 每个 capability 只允许在一个 migration module 中列出 direct reference，其他引用全部通过测试失败。
3. 将 service client API 变成 LLM、memory、context、task、driver、skill、MCP、app lifecycle、store、entitlement、payment、Web3、EVM、gateway 的唯一 public production entrypoint。

## 发现 P2-1：Kernel/Web/Task 测试和部分 Production Path 仍存在硬编码 Agent Role Name

状态：部分满足；取决于每处出现是 test-only 还是 compatibility。

证据：

- `macaca/crates/kernel/macaca-kernel/src/executor/router.rs:259` 在 kernel tests 或 examples 中使用 `"coordinator"`。
- `macaca/crates/kernel/macaca-kernel/src/executor/event_factory.rs:94` 在 tests 中使用 `"planner"`。
- `macaca/crates/shells/macaca-web/src/chat_orchestrator.rs:875` fallback 到 `"coordinator"`。
- `macaca/crates/shells/macaca-web/src/orchestration_tools.rs:116` 引用 `"coordinator"`。
- `macaca/crates/runtime/macaca-runtime-host/src/agent_execution_service_provider.rs:162` 使用 `"worker"`。
- OpenSpec 中存在 `refactor-core-architecture/specs/eliminate-hardcoded-coordinator/spec.md`，要求从 kernel/task/runtime/proto/Web 中移除 hardcoded agent names。

为什么违反设计：

- 治理文档禁止 OS layers 中出现硬编码 workflow/app/provider/driver/business names。
- 部分 role name 现在被视作 compatibility default，但目标是 manifest/config-driven identity。

重构建议：

1. 将 Web coordinator fallback 替换为 manifest entry-agent resolution，并在 manifest 无效时返回结构化错误。
2. 将默认 role names 移入 test fixtures 或 application manifests。
3. 增加 executable scan，扫描 fixture/test directories 之外 production code 中的 `"coordinator"`、`"planner"`、`"worker"`、`"backend"`、`"frontend"`、`"architect"`。

## 发现 P2-2：文件大小和职责边界仍违反项目自身重构规则

状态：不满足仓库架构卫生规则。

证据：

- `framework_runner.rs`：2484 行。
- `loop_manager.rs`：2629 行。
- `chat_orchestrator.rs`：1581 行。
- `macaca-web` 的 `lib.rs`：975 行。
- `domain_pack_service_provider.rs`：560 行。
- `a2a.rs`：515 行。

为什么重要：

- 项目规则规定 Rust 文件最多 500 行，并将巨型文件视为所有权不清的证据。
- 最大的文件集中在本应最薄的 shell 和 orchestration 区域。

重构建议：

1. 将每个文件大小违规都视为 ownership split，而不是格式化任务。
2. 将 Web execution 拆成 route adapters、SSE adapter、session-channel adapter、service-client adapter 和 compatibility-only modules。
3. 将 domain-pack provider 拆成 generic registration、finance descriptor、market adapter、financials adapter、news adapter 和 tests。
4. 将 kernel A2A/Web3/EVM compatibility facades 拆成 deprecated modules 和 service-client-facing modules。

## 发现 P2-3：OpenSpec Baseline 没有跟上已完成的架构变化

状态：流程和设计可追踪性缺口。

证据：

- `openspec list --specs` 只返回一个 baseline spec：`context-composer`。
- `openspec list` 显示大量 completed changes，包括 service runtime、serviceization dependency gate、Web/CLI thin shell、payment/A2A service、execution control service、WASM host import service portal 等。

为什么违反工作流意图：

- 项目约定 `openspec/specs/` 是 baseline fact source。
- 如果已完成 changes 仍停留在 `openspec/changes/`，baseline 无法表达当前期望架构。

重构建议：

1. 分批 archive completed changes，并更新 baseline specs。
2. 先处理 governance/core specs：service runtime、dependency gate、SDK/SystemFacade、Web/CLI thin shell、application service、execution control。
3. 每批之后运行 `openspec validate --strict`。
4. 实现存在后，不要再把 completed task checklist 当作权威事实源。

## 正向实现证据

以下区域与设计方向一致，应当保留：

- `ServiceDescriptor`、`ServiceCommand`、`ServiceCallResult` 和 `ServiceError` 是 provider-neutral 且可扩展的。
- `SystemService` 暴露 descriptor/start/call/stop/cleanup/health。
- `ServiceRouter` 是 canonical `service.call` 的清晰 Policy Enforcement Point。
- WASM host import tests 覆盖了 service runtime routing、policy denial、structured unavailable behavior 和 service-call audit replay。
- Dependency boundary gate 是 deterministic 的，并能阻止新增未 allowlist 的 forbidden direct edge。
- 很多 compatibility paths 已显式标记 deprecated，并带有 replacement guidance。

## 推荐重构顺序

### Phase 1：冻结逃逸口

- 增加 static tests，禁止 production code 新增对 deprecated direct fields 和 methods 的引用。
- 保留当前 allowlist rows，但为每行增加 owner/current-caller/expiry 细节。
- 将 deprecation warnings 按 serviceization tracks 分组，让它们变成可执行迁移清单。

### Phase 2：移除 Kernel Provider Compatibility

- 用 service-client execution ports 替换 `KernelProviderCompat` production wiring。
- 将 agent execution provider ownership 移到 runtime-host/application framework service boundaries。
- 逐条移除 kernel 对 service provider crates 的直接依赖。

### Phase 3：瘦身 Web

- 用 focused SDK clients 替换 Web 对 LLM/memory/driver/skill/MCP/task runtime 的直接访问。
- 将 loop/session control 下沉到 `service.execution_control` 和 task service APIs。
- Web 只保留 HTTP/SSE/GenUI rendering 和 event subscription。

### Phase 4：解耦 CLI

- 通过将 server-start seam 移到小型 public bootstrap facade 或 binary-only entrypoint，移除 `macaca-cli -> macaca-web`。
- 通过 SDK/runtime clients 移除 CLI 对 gateway/tools provider 的直接依赖。

### Phase 5：外置 Domain Packs

- 将 finance/crypto domain-pack implementations 移出 base runtime-host。
- 将其注册为带 descriptor 和 policy metadata 的 plugin/package service providers。
- 保持 runtime-host 通用。

### Phase 6：对齐 OpenSpec Baseline

- 将 completed changes archive 到 `openspec/specs/`。
- 让 baseline specs 覆盖 service runtime、dependency gate、SDK/SystemFacade、application service、execution control 和 Web/CLI thin shell。
- 重新运行 strict OpenSpec validation 和 dependency boundary tests。

## 下一步具体工作项

1. 创建一个 OpenSpec change：`freeze serviceization escape hatches`。
2. 增加 static regression tests：
   - migration modules 之外不得新增 `AppState` deprecated-field reads。
   - production code 中不得新增 hardcoded role names。
   - 不得新增 direct `AppRuntime::start_app` callers。
   - migration modules 之外不得直接调用 Web driver/MCP runtime。
3. 先处理 `macaca-kernel -> macaca-task/tools/persist` allowlist rows，因为 kernel purity 是最高优先级 invariant。
4. 然后将 `framework_toolkit` 迁移到 service clients，因为这一步可以一次性移除多条 Web provider-construction rows。
5. 每删除一条 allowlist row 后，运行：
   - `cargo metadata --no-deps --format-version 1`
   - `cargo test -p macaca-integration-tests route_c_dependency_boundaries_reject_unallowlisted_forbidden_edges -- --nocapture`
   - 该 capability 对应的 targeted service tests。

## 验证结果

当前 dependency gate 通过，但它只能证明没有新增未追踪的 forbidden direct dependency edge。它不能证明当前实现已经满足三份稳定治理文档，因为 16 条已知 forbidden edge 仍被 allowlist 放行，并且 production code 中仍存在多条 direct compatibility path。
