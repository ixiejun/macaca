# Macaca OS Plugin 服务完善实施计划

## 1. 目标

基于 `2026-05-11-plugin-service-enrichment-brainstorm.md` 选择 **方案 E：分层 Plugin Fabric，先控制面，后执行面**。

本计划覆盖第一阶段到第四阶段：

1. Plugin Control Plane v1
2. Plugin Capability Registry v1
3. Plugin Hook Bus v1
4. Plugin SDK / ABI / Hosts v1

完成后，Plugin 服务应从当前 descriptor-first 骨架升级为 Macaca OS 的完整核心能力，能够支撑：

- 插件安装、发现、校验、启停、升级、卸载。
- 插件配置、密钥声明、权限、资源、健康、trace/audit。
- 插件声明并注册 Driver、Skill、MCP、Gateway、Memory、Context、LLM Provider、Observability、Tool、Hook、HTTP Route、CLI Command 等能力。
- 插件通过稳定 SDK / ABI 接入 Macaca，而不是依赖内部 crate。
- 插件通过 descriptor、built-in adapter、WASM skeleton、process skeleton、remote proxy skeleton 等 host 边界运行或接入。
- Web / CLI / SDK 通过服务边界管理插件，保持 thin shell。

本计划只定义实施路线。后续必须先产出 OpenSpec proposal / design / tasks / spec，得到批准后再实现。

## 2. 约束

- 必须遵守 `macaca/docs/agent-os-microkernel-boundaries.md`。
- 必须遵守 `macaca/docs/route-c-architecture-governance.md`。
- 必须遵守 `macaca/docs/route-c-serviceization-allowlist.md`。
- 必须遵守当前 Route C workspace topology：
  - `foundation` 放协议、IPC、持久化基础。
  - `kernel` 只保留 OS invariant。
  - `runtime` 放 host、runtime、framework。
  - `services` 放非内核能力服务。
  - `facade` 放 SDK。
  - `shells` 放 Web / CLI thin shell。
- 禁止把 provider、application、workflow、driver、gateway、model、chain、业务名称硬编码到 Plugin 服务逻辑。
- 禁止让 plugin 绕过 ServiceRuntime、Service Registry、permission/resource admission、trace/audit。
- 禁止 kernel 执行 plugin code。
- 禁止一次性删除已有兼容逻辑；旧接口先标记 deprecated，保留搜索锚点，便于迁移和排障。
- 所有新增 Rust 代码必须有详尽英文注释，解释功能、运行原理、边界和不变量。
- 每个文件代码行数不得超过 500 行，超出必须拆分模块。

## 3. 总体架构

推荐形成四层 Plugin Fabric：

```text
Web / CLI / SDK
  ↓
Plugin Service Facade
  ↓
Plugin Control Plane
  - repository
  - install source
  - manifest loader
  - compatibility
  - config / secret declarations
  - enable / disable
  - health snapshot
  ↓
Plugin Capability Plane
  - capability registry
  - contract-first discovery
  - conflict policy
  - service/capability descriptors
  ↓
Plugin Hook Plane
  - typed hook bus
  - priority
  - timeout
  - fail-open / fail-closed
  - trace/audit
  ↓
Plugin Execution Plane
  - descriptor host
  - built-in adapter host
  - WASM host skeleton
  - process host skeleton
  - remote proxy host skeleton
  ↓
ServiceRuntime / Kernel Registry / IPC / Trace / Store / Entitlement
```

Kernel 只保留：

- plugin id 唯一性。
- lifecycle state invariant。
- service/capability ownership。
- uninstall cleanup。
- permission/resource admission 的入口或可审计结果。

Runtime-host 负责：

- host factory。
- lifecycle supervisor。
- sandbox/proxy 边界。
- health probe。
- hook execution coordinator。
- resource lease 与 timeout。

Services 负责：

- Driver / Skill / MCP / Gateway / Memory / Context / LLM / Observability 的业务能力。
- 将 built-in 能力适配成 plugin-provided capability。
- 通过 ServiceRuntime 和 SDK client 暴露调用。

SDK 负责：

- 提供开发者可用的稳定 API。
- 不暴露内部 crate。
- 提供 contract test kit 与 fixture builder。

## 4. 设计模式

必须采用：

- **Facade**：`PluginServiceFacade` / `PluginControlService` / `PluginSdkFacade` 对外隐藏内部复杂度。
- **Abstract Factory**：`PluginHostFactory` 创建 descriptor、built-in、WASM、process、remote proxy host。
- **Adapter**：把 Driver、Skill、MCP、Gateway、Memory、Context、LLM Provider、Observability 适配为 plugin capability。
- **Strategy**：安装源、签名验证、compat policy、activation policy、hook failure policy、resource policy、entitlement policy 可替换。
- **Chain of Responsibility**：install admission 走 manifest parse -> schema validate -> signature -> compatibility -> permission -> resource -> entitlement -> security scan。
- **State**：plugin lifecycle 和 host lifecycle 都必须是显式状态机。
- **Observer / Event Bus**：plugin lifecycle、hook、capability call、health、policy denial 均发 trace/audit。
- **Proxy**：WASM、process、remote plugin 均通过 proxy，不把实现对象暴露给 core。
- **Command**：install、enable、disable、start、stop、call、hook invoke 都必须是 typed command。
- **Specification**：manifest、capability、hook、permission、resource、compat、signature 使用规则对象校验。

谨慎采用：

- **Composite**：允许一个 plugin 提供多个 capability，但必须有 ownership、权限和卸载清理。
- **Builder**：用于 manifest/test fixture/SDK registration。
- **Null Object**：缺失可选插件返回 structured unavailable。
- **Memento**：plugin registry snapshot、health snapshot、activation snapshot。

## 5. OpenSpec 拆分

建议一次性产出 4 个 OpenSpec 提案，统一设计边界，分阶段实施。

### 5.1 `add-plugin-control-plane-v1`

负责第一阶段：Plugin Control Plane。

包含：

- 插件仓库和安装源抽象。
- manifest loader 和 compatibility policy。
- plugin enable/disable/startup activation policy。
- config schema、secret requirement、env requirement。
- health snapshot 和 diagnostics。
- Plugin Service facade 的控制面命令。
- CLI/Web/SDK 管理 API 的服务端契约。

### 5.2 `add-plugin-capability-registry-v1`

负责第二阶段：Plugin Capability Registry。

包含：

- capability descriptor schema。
- contract-first discovery，不启动 runtime 即可发现 capability owner。
- tool/provider/channel/gateway/driver/skill/mcp/memory/context/llm-provider/observability/http-route/cli-command capability。
- conflict policy。
- built-in service adapter canonicalization。
- capability call routing 的服务边界。

### 5.3 `add-plugin-hook-bus-v1`

负责第三阶段：Plugin Hook Bus。

包含：

- typed hook names。
- observer / mutating / blocking / approval hook categories。
- priority、timeout、failure policy。
- hook result schema。
- trace/audit。
- 核心 hook 点：agent lifecycle、application lifecycle、task lifecycle、tool call、prompt/context build、memory ingest、LLM call、gateway message、approval、session start/end。

### 5.4 `add-plugin-sdk-and-hosts-v1`

负责第四阶段：Plugin SDK / ABI / Hosts。

包含：

- Plugin SDK facade。
- manifest builder、registration builder、contract test kit。
- descriptor host 和 built-in host canonical API。
- WASM host skeleton。
- process host skeleton。
- remote proxy host skeleton。
- host lifecycle supervisor skeleton。
- sandbox/resource/timeout/health 边界。

## 6. 实施顺序

必须按以下顺序实施，避免执行面早于控制面：

1. `add-plugin-control-plane-v1`
2. `add-plugin-capability-registry-v1`
3. `add-plugin-hook-bus-v1`
4. `add-plugin-sdk-and-hosts-v1`
5. 跨阶段集成与回归：Web / CLI thin shell、SDK client、Route C dependency gate、GitNexus detect。

## 7. 第一阶段：Plugin Control Plane v1

### 7.1 Scope

建立 Plugin 的安装、发现、配置、启停、健康和管理控制面。

### 7.2 主要文件

新增或修改候选：

- `macaca/crates/foundation/macaca-proto/src/plugin.rs`
- `macaca/crates/runtime/macaca-runtime-host/src/plugin.rs`
- `macaca/crates/runtime/macaca-runtime-host/src/plugin_control/`
- `macaca/crates/runtime/macaca-runtime-host/src/service_runtime.rs`
- `macaca/crates/facade/macaca-sdk/src/plugin_client.rs`
- `macaca/crates/shells/macaca-cli/src/commands.rs`
- `macaca/crates/shells/macaca-cli/src/command_handlers.rs`
- `macaca/crates/shells/macaca-web/src/routes.rs`
- `macaca/crates/shells/macaca-web/src/state.rs`
- `macaca/crates/tests/macaca-integration-tests/tests/`
- `macaca/docs/developer/plugin-development-guide.md`

### 7.3 切片

#### Slice P1.1：影响分析与现状审计

动作：

- 读取当前 `PluginRuntimeFacade`、`PluginRegistry`、`PluginManifest`。
- 读取 Store/Entitlement、ServiceRuntime、CLI/Web 当前管理路径。
- GitNexus impact 需要在编辑任何现有 symbol 前执行。
- 记录 blast radius。

输出：

- OpenSpec design 中的现状差距表。

#### Slice P1.2：Plugin Repository 与 Install Source 抽象

新增：

- `PluginRepository`
- `PluginInstallSource`
- `PluginInstallSourceKind`
- `PluginInstallRequest`
- `PluginInstallResult`
- `PluginPackageLocation`

支持源：

- bundled。
- user local directory。
- project local directory。
- dev-link。
- archive。
- store-cache placeholder。
- git placeholder。

规则：

- 第一版可以不实现真实网络 clone。
- 路径必须防 traversal。
- project plugin 默认关闭，必须显式启用。
- install source 不能硬编码业务名称。

#### Slice P1.3：Manifest Loader 与 Compatibility Chain

新增：

- `PluginManifestLoader`
- `PluginManifestFormat`
- `PluginCompatibilityPolicy`
- `PluginAdmissionChain`
- `PluginAdmissionReport`

行为：

- 支持 JSON/TOML/YAML 中至少一种项目现有最自然格式；如果要引入 YAML 依赖必须明确理由，否则优先 TOML/JSON。
- 校验 manifest version、plugin api range、Macaca OS version、runtime kind、signature metadata、permissions/resources。
- 失败返回 structured diagnostics。

#### Slice P1.4：Config / Secret / Env Requirement

新增 proto DTO：

- `PluginConfigSchema`
- `PluginSecretRequirement`
- `PluginEnvRequirement`
- `PluginConfigStatus`

行为：

- 插件可声明需要的 config、secret、env。
- 不在 trace/log 中泄漏 secret。
- CLI/Web 可显示 missing requirements。

#### Slice P1.5：Activation / Enable / Disable / Health

新增：

- `PluginActivationPolicy`
- `PluginActivationState`
- `PluginHealthSnapshot`
- `PluginDiagnosticsReport`

行为：

- enable/disable 是显式状态。
- disabled plugin 不注册 active capability。
- missing optional dependency 返回 degraded/unavailable，不阻塞 OS。
- health snapshot deterministic。

#### Slice P1.6：Plugin Control Service Facade

新增：

- `PluginControlService`
- typed commands：
  - `plugin.list`
  - `plugin.inspect`
  - `plugin.install`
  - `plugin.enable`
  - `plugin.disable`
  - `plugin.start`
  - `plugin.stop`
  - `plugin.uninstall`
  - `plugin.health`
  - `plugin.diagnostics`

规则：

- 所有命令必须带 trace context。
- 所有状态变更必须 emit trace/audit。
- Web/CLI 只能通过 service/SDK client 调用。

#### Slice P1.7：CLI/Web Thin Shell 管理入口

CLI：

- `macaca plugin list`
- `macaca plugin inspect <id>`
- `macaca plugin enable <id>`
- `macaca plugin disable <id>`
- `macaca plugin health <id>`

Web：

- 提供 plugin list / inspect / health API。
- 不直接读 plugin filesystem。

### 7.4 验收

- `openspec validate add-plugin-control-plane-v1 --strict`
- `cargo test -p macaca-proto plugin`
- `cargo test -p macaca-runtime-host plugin_control`
- `cargo test -p macaca-sdk plugin_client`
- `cargo test -p macaca-cli plugin`
- `cargo test -p macaca-web plugin`
- `cargo check --workspace`

## 8. 第二阶段：Plugin Capability Registry v1

### 8.1 Scope

让插件成为系统 capability 的正式提供者，而不仅是 lifecycle descriptor。

### 8.2 主要文件

新增或修改候选：

- `macaca/crates/foundation/macaca-proto/src/plugin.rs`
- `macaca/crates/foundation/macaca-proto/src/capability_tool.rs`
- `macaca/crates/kernel/macaca-kernel/src/plugin_registry.rs`
- `macaca/crates/runtime/macaca-runtime-host/src/plugin_capability/`
- `macaca/crates/services/macaca-driver/src/service_adapter.rs`
- `macaca/crates/services/macaca-skill/src/service_adapter.rs`
- `macaca/crates/runtime/macaca-runtime-host/src/mcp_service_provider.rs`
- `macaca/crates/services/macaca-gateway/src/service_adapter.rs`
- `macaca/crates/services/macaca-memory/src/service_adapter.rs`
- `macaca/crates/services/macaca-context/src/service_contract.rs`
- `macaca/crates/services/macaca-llm/src/service_adapter.rs`
- `macaca/crates/facade/macaca-sdk/src/plugin_client.rs`

### 8.3 切片

#### Slice P2.1：Capability Descriptor Schema

新增：

- `PluginCapabilityDescriptor`
- `PluginCapabilityKind`
- `PluginCapabilityContract`
- `PluginCapabilityVisibility`
- `PluginCapabilityInputSchema`
- `PluginCapabilityOutputSchema`
- `PluginCapabilityPermissionHint`
- `PluginCapabilityResourceHint`

能力类型：

- tool。
- hook。
- driver。
- gateway。
- skill。
- mcp。
- memory。
- context。
- llm_provider。
- observability。
- http_route。
- cli_command。
- custom。

#### Slice P2.2：Contract-First Discovery

行为：

- 系统可从 manifest/repository snapshot 得到 capability owner。
- 不启动 plugin runtime。
- 支持按 kind、service id、capability id、visibility 查询。

#### Slice P2.3：Conflict Policy

新增：

- `PluginCapabilityConflictPolicy`
- `PluginCapabilitySlotPolicy`
- `PluginCapabilityConflictReport`

规则：

- tool 同名冲突必须可解释。
- exclusive slot，如默认 memory/context/provider，只允许一个 active owner 或有显式优先级。
- gateway route / http route / cli command 冲突必须 fail-closed。

#### Slice P2.4：Built-in Adapter Canonicalization

动作：

- 将当前 built-in descriptor 构造统一到 canonical facade。
- 旧直接 descriptor construction 标记 deprecated，不删除。
- Driver / Skill / MCP / Gateway / Memory / Context / LLM provider 内置能力都能暴露为 plugin-provided capability。

#### Slice P2.5：Capability Registration 与 Cleanup

行为：

- enable/start 注册 active capability。
- disable/stop/uninstall 清理 capability ownership。
- registry snapshot 不残留 stale descriptor。

#### Slice P2.6：Capability Call Routing Skeleton

行为：

- 提供 provider-neutral call envelope。
- 第一版只路由 descriptor/built-in adapter 或 unavailable，不急于真实执行外部代码。
- call 必须通过 permission/resource admission 和 trace。

### 8.4 验收

- `openspec validate add-plugin-capability-registry-v1 --strict`
- `cargo test -p macaca-proto plugin_capability`
- `cargo test -p macaca-kernel plugin_registry`
- `cargo test -p macaca-runtime-host plugin_capability`
- `cargo test -p macaca-integration-tests route_c_dependency_boundaries`
- `cargo check --workspace`

## 9. 第三阶段：Plugin Hook Bus v1

### 9.1 Scope

让 plugin 能安全参与 Macaca OS 关键生命周期和执行节点。

### 9.2 主要文件

新增或修改候选：

- `macaca/crates/foundation/macaca-proto/src/plugin.rs`
- `macaca/crates/runtime/macaca-runtime-host/src/plugin_hooks/`
- `macaca/crates/runtime/macaca-runtime-host/src/plugin.rs`
- `macaca/crates/runtime/macaca-framework/src/`
- `macaca/crates/services/macaca-task/src/service_adapter.rs`
- `macaca/crates/services/macaca-context/src/`
- `macaca/crates/services/macaca-memory/src/`
- `macaca/crates/services/macaca-gateway/src/`
- `macaca/crates/shells/macaca-web/src/`

### 9.3 切片

#### Slice P3.1：Hook Contract

新增：

- `PluginHookName`
- `PluginHookKind`
- `PluginHookDescriptor`
- `PluginHookInvocation`
- `PluginHookResult`
- `PluginHookFailurePolicy`
- `PluginHookTimeoutPolicy`

Hook categories：

- observer。
- mutating。
- blocking。
- approval。

#### Slice P3.2：Hook Runner

新增：

- `PluginHookBus`
- `PluginHookRunner`
- `PluginHookRegistry`
- `PluginHookInvocationContext`
- `PluginHookAuditEvent`

行为：

- 按 priority 执行。
- timeout 后按 failure policy 处理。
- mutating hook 输出必须走 schema validation。
- blocking hook 可返回 allow/block/require_approval。
- observer hook 失败默认 fail-open。

#### Slice P3.3：核心 Hook 点

第一批必须覆盖：

- `before_agent_start`
- `after_agent_end`
- `before_prompt_build`
- `after_context_assemble`
- `before_tool_call`
- `after_tool_call`
- `before_llm_call`
- `after_llm_call`
- `before_memory_ingest`
- `after_memory_ingest`
- `before_gateway_dispatch`
- `after_gateway_send`
- `before_approval_request`
- `after_approval_response`
- `session_start`
- `session_end`
- `task_started`
- `task_completed`
- `application_start`
- `application_stop`

#### Slice P3.4：Trace / Audit / Replay

行为：

- 每次 hook invocation 都记录 plugin id、hook name、duration、decision、error code、trace id。
- 不记录 secret、raw prompt 全量、API key、provider credential。
- trace viewer 后续可显示 hook event。

#### Slice P3.5：Hook Safety

规则：

- hook 不能无限阻塞。
- hook 不能绕过 permission。
- hook 不能直接调用内部 runtime 对象。
- hook result 必须被服务层解释，不允许 plugin 任意修改核心状态。

### 9.4 验收

- `openspec validate add-plugin-hook-bus-v1 --strict`
- `cargo test -p macaca-proto plugin_hook`
- `cargo test -p macaca-runtime-host plugin_hook`
- `cargo test -p macaca-framework plugin_hook`
- `cargo test -p macaca-integration-tests plugin_hook`
- `cargo check --workspace`

## 10. 第四阶段：Plugin SDK / ABI / Hosts v1

### 10.1 Scope

提供开发者可用 SDK / ABI，并建立执行 host skeleton。完成后 Macaca Plugin 服务具备完整生态入口。

### 10.2 主要文件

新增或修改候选：

- `macaca/crates/facade/macaca-sdk/src/plugin.rs`
- `macaca/crates/facade/macaca-sdk/src/plugin_client.rs`
- `macaca/crates/facade/macaca-sdk/src/plugin_fixture.rs`
- `macaca/crates/runtime/macaca-runtime-host/src/plugin_hosts/`
- `macaca/crates/runtime/macaca-runtime-host/src/plugin.rs`
- `macaca/crates/foundation/macaca-ipc/src/`
- `macaca/crates/foundation/macaca-proto/src/plugin.rs`
- `macaca/docs/developer/plugin-development-guide.md`
- `macaca/crates/tests/macaca-integration-tests/tests/`

### 10.3 切片

#### Slice P4.1：Plugin SDK Facade

新增：

- `PluginSdk`
- `PluginContext`
- `PluginRegistrationBuilder`
- `PluginManifestBuilder`
- `PluginCapabilityBuilder`
- `PluginHookBuilder`
- `PluginContractTestKit`

规则：

- SDK 暴露窄接口。
- 不暴露 internal runtime-host/kernel structs。
- SDK DTO 走 `macaca-proto`。

#### Slice P4.2：Contract Test Kit

新增：

- manifest contract tests。
- capability contract tests。
- hook contract tests。
- config/secret validation tests。
- unavailable-safe behavior tests。

用途：

- 内置插件和未来第三方插件都可以用同一套测试。

#### Slice P4.3：WASM Host Skeleton

新增：

- `WasmPluginHost`
- `WasmPluginHostFactory`
- `WasmPluginHostConfig`
- `WasmPluginHostUnavailable`

行为：

- 第一版可以不引入真实 WASM 执行依赖。
- 需要定义 ABI 边界、host functions、resource/permission/trace 原则。
- 如果暂不执行，返回 structured unavailable，并保留 skeleton。

#### Slice P4.4：Process Host Skeleton

新增：

- `ProcessPluginHost`
- `ProcessPluginSupervisor`
- `ProcessPluginTransport`
- `ProcessPluginHealthProbe`

行为：

- 第一版可以不启动真实外部进程。
- 必须定义 spawn policy、timeout、shutdown、stdout/stderr handling、resource lease、trace。
- 默认 fail-closed。

#### Slice P4.5：Remote Proxy Host Skeleton

新增：

- `RemoteProxyPluginHost`
- `RemotePluginEndpoint`
- `RemotePluginTransportPolicy`
- `RemotePluginHealthProbe`

行为：

- 第一版可以只支持 mock/local test transport。
- 必须定义 auth、TLS、timeout、retry、circuit breaker placeholder。

#### Slice P4.6：SDK 示例与开发文档

更新：

- `macaca/docs/developer/plugin-development-guide.md`

内容：

- 最小 descriptor plugin。
- built-in adapter plugin。
- hook plugin。
- capability plugin。
- remote proxy plugin skeleton。
- manifest 示例。
- contract test 命令。

### 10.4 验收

- `openspec validate add-plugin-sdk-and-hosts-v1 --strict`
- `cargo test -p macaca-sdk plugin`
- `cargo test -p macaca-runtime-host plugin_hosts`
- `cargo test -p macaca-proto plugin`
- `cargo test -p macaca-integration-tests plugin_contract`
- `cargo check --workspace`

## 11. 跨阶段集成

四个阶段全部完成后必须做：

### 11.1 上层消费迁移

迁移目标：

- Web 不直接读 plugin 文件系统。
- CLI 不直接操作 plugin runtime internals。
- SDK client 成为上层管理入口。
- Driver/Skill/MCP/Gateway/Memory/Context/LLM built-in capability 都通过 plugin descriptor / capability registry 可见。

旧接口：

- 标记 `#[deprecated]`。
- 保留原语义和代码。
- 禁止新调用。

### 11.2 Route C 边界检查

必须确认：

- kernel 不依赖 provider/service 实现。
- runtime-host 不变成业务 hub。
- shells 不直接依赖内部 plugin host。
- services 不绕过 ServiceRuntime。

### 11.3 Trace / Audit

必须覆盖：

- install。
- validate。
- enable。
- start。
- hook invoke。
- capability register。
- capability call。
- health change。
- disable。
- uninstall。
- failure。

### 11.4 Web / CLI 管理面

最低完成态：

- list。
- inspect。
- health。
- enable。
- disable。
- diagnostics。

install/update/remove 可先支持 local/dev-link，git/store 留扩展点。

## 12. 验证矩阵

### 12.1 OpenSpec

```bash
openspec validate add-plugin-control-plane-v1 --strict
openspec validate add-plugin-capability-registry-v1 --strict
openspec validate add-plugin-hook-bus-v1 --strict
openspec validate add-plugin-sdk-and-hosts-v1 --strict
```

### 12.2 Cargo

```bash
cargo fmt --all --check
cargo check --workspace
cargo test -p macaca-proto plugin
cargo test -p macaca-kernel plugin_registry
cargo test -p macaca-runtime-host plugin
cargo test -p macaca-sdk plugin
cargo test -p macaca-cli plugin
cargo test -p macaca-web plugin
cargo test -p macaca-integration-tests plugin
cargo test -p macaca-integration-tests route_c_dependency_boundaries
```

### 12.3 Scripts / Governance

```bash
bash scripts/check-cli-consumer-migration.sh
bash scripts/check-web-cli-thin-shell.sh
npx gitnexus detect-changes -r agent
```

如目录或大量符号变化后：

```bash
npx gitnexus analyze
```

## 13. 风险与应对

| 风险 | 后果 | 应对 |
| --- | --- | --- |
| 四阶段范围大 | 实现时间长、易冲突 | 四个 OpenSpec 分提案、按阶段合入 |
| WASM/process host 过早做真实执行 | 安全和稳定风险 | 第四阶段先 skeleton + unavailable-safe，真实执行后续单独提案 |
| Plugin Runtime 变宏内核 | 破坏 Route C | runtime-host 只做 host/supervisor/proxy，业务仍在 services |
| Hook 影响主流程稳定 | 7x24 系统卡死 | timeout、failure policy、priority、trace |
| 权限资源只做声明不做治理 | 插件绕过边界 | capability call 必须经过 admission chain |
| Web/CLI 直接耦合 runtime | thin shell 退化 | 只通过 SDK client / Plugin Service |
| 插件 SDK 过大 | 难维护 | 分 capability submodule，最小核心优先 |
| 现有内置能力迁移破坏行为 | 用户流程回归 | built-in adapter additive first，旧路径 deprecated 保留 |

## 14. 完成定义

四个阶段完成后，Plugin 服务必须满足：

- 插件可以被安装源发现、加载、校验、启用、禁用、健康检查、卸载。
- 插件 capability 可以在不启动 runtime 的情况下被发现。
- 插件可声明 tool、hook、driver、gateway、skill、mcp、memory、context、llm-provider、observability、http-route、cli-command。
- 插件 hook 可以安全参与核心生命周期，具备超时、失败策略、trace/audit。
- SDK 提供稳定开发入口，第三方无需依赖内部 crate。
- WASM/process/remote proxy host skeleton 明确存在，且 unavailable-safe。
- Web / CLI 通过 thin shell 管理插件。
- 旧 direct runtime/descriptor 构造路径保留并标记 deprecated。
- `cargo check --workspace` 和相关 plugin/service/Route C 测试通过。
- GitNexus detect-changes 风险符合预期；若出现 HIGH/CRITICAL，必须先报告并重新评估。

## 15. 后续动作

下一步应创建 4 个 OpenSpec 提案：

1. `add-plugin-control-plane-v1`
2. `add-plugin-capability-registry-v1`
3. `add-plugin-hook-bus-v1`
4. `add-plugin-sdk-and-hosts-v1`

建议一次性创建四个提案的 proposal/design/tasks/spec，以确保边界统一；实现时严格按阶段顺序推进。
