# S6 Driver / Skill / MCP 服务化与模块化 Brainstorm

## 背景

S6 来自 `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`，目标是把 Driver、Skill、MCP 从 Web / Kernel / framework 里的直接组装路径迁移为可安装、可替换、可审计的 capability provider 和 system service。内置 driver、内置 skill、全局 MCP、skill-backed MCP 都只能是 built-in provider，不应成为特殊控制流。

必须遵守：

- `macaca/docs/agent-os-microkernel-boundaries.md`
- `macaca/docs/route-c-serviceization-allowlist.md`
- `macaca/docs/route-c-architecture-governance.md`
- `macaca/docs/route-c-regression-matrix.md`

当前诊断：

- `macaca-driver` 已有 `SoftwareDriver`、`DriverRegistry`、`DriverRuntime`、`DriverLoadCommand`、`DriverCommand`、`DriverTraceAdapter`、descriptor-only `driver_service_descriptor()`。
- `macaca-skill` 已有 `SkillRuntimeFacade`、`SkillSnapshotRequest`、`ExecutableSkillToolSet`、`SkillRegistrySnapshot`、descriptor-only `skill_service_descriptor()`。
- `macaca-runtime-host` 已有 `ServiceRuntime`、`ServiceProviderFactory`、`McpRuntimeFacade`、`McpServerFactory`、MCP lifecycle / policy / probe / status / tool registration。
- `macaca-web` 仍直接持有 `DriverRuntime`、`McpRuntimeFacade`，直接加载 executable skills，直接把 driver tools / MCP tools / skill-backed MCP tools 注册进 toolkit。
- `macaca-cli` 仍直接依赖 `macaca-tools`，并保留 provider/tool 兼容入口。
- `macaca-kernel` allowlist 中仍有 `macaca-driver`、`macaca-skill`、`macaca-tools` 的迁移债务。
- `macaca-sdk` 已有 generic `SystemServiceClient`，但还没有 `SystemDriverClient`、`SystemSkillClient`、`SystemMcpClient` 这类 focused clients。

S6 不包含：

- Application lifecycle serviceization，属于 S7。
- Gateway serviceization，属于 S8。
- Store / entitlement 深度服务化，属于 S9；S6 只在 encrypted skill / skill package 路径预留 entitlement hook。
- Web/CLI 完全 thin shell，属于 S12；S6 只迁移 Driver / Skill / MCP 相关调用。

## 设计模式候选

### Facade

建立 `DriverService`、`SkillService`、`McpService` 三个 focused service boundary：

- Driver Service：load/reload/inventory/tool invocation/status/cancel/cleanup。
- Skill Service：discover/snapshot/load executable skills/expose tools/invoke/status。
- MCP Service：register definitions/probe/start or attach tools/list status/stop/cleanup。

优点：

- Web/CLI/framework 只看 service client，不再拿 registry/runtime/provider。
- 每个能力边界单一，避免一个巨型 Tool Service 吃掉所有差异。
- 与 S1 `ServiceRuntime`、S3 `SystemFacade`、S5 focused clients 一致。

风险：

- Driver、Skill、MCP 都能暴露 tools，边界容易重复。
- 需要明确：service owns lifecycle and inventory，tool invocation remains a command through the owning service; generic tool display can use common DTO but不应把三者强行合并。

### Adapter / Bridge

把现有 `DriverRuntime`、`SkillRuntimeFacade`、`ExecutableSkillToolSet`、`McpRuntimeFacade` 作为内置实现，通过 runtime-host `SystemService` provider wrapper 暴露给 `ServiceRuntime`。

优点：

- 复用现有 driver/skill/MCP 行为，不重写协议和 tool execution。
- Web 可以逐步从 direct runtime 切到 service client，旧 runtime 字段保留 deprecated 搜索锚点。
- 未来 plugin/remote/WASM provider 只需要实现 service provider 或 service transport。

风险：

- 如果 runtime-host 直接写死具体 driver/skill/MCP 名称，会形成新的宏内核。
- Mitigation：provider wrapper 只接受 trait/facade/registry handles，descriptor/capability 驱动，不按 driver name / skill name / MCP server name 分支。

### Abstract Factory

通过 `ServiceProviderFactory` 创建 Driver / Skill / MCP provider instance。内置 provider、plugin provider、package-installed provider、remote provider 使用同一个 factory contract。

优点：

- 可安装 capability provider 可以从 manifest/package/plugin metadata 构建。
- 与 S1 `StaticServiceProviderFactory` 和 future Store / Plugin Runtime 兼容。
- 让 built-in provider 只是默认 factory 输出，不是控制流特例。

风险：

- 首版如果做完整 package factory 会跨到 S7/S9/S13。
- Mitigation：S6 只定义 factory seam 和 built-in factory；package/store integration 留给后续阶段。

### Command

所有 service call 使用 typed command，然后序列化为 `ServiceCommand` payload：

- `DriverLoadCommandV1`、`DriverInventoryCommandV1`、`DriverToolInvokeCommandV1`、`DriverStatusCommandV1`、`DriverCleanupCommandV1`。
- `SkillSnapshotCommandV1`、`SkillExecutableLoadCommandV1`、`SkillToolInvokeCommandV1`、`SkillStatusCommandV1`。
- `McpRegisterCommandV1`、`McpProbeCommandV1`、`McpAttachToolsCommandV1`、`McpStatusCommandV1`、`McpCleanupCommandV1`。

优点：

- Trace、policy、resource scope、permission、session/app/agent scope 可审计。
- SDK/Web/CLI/Gateway 可以复用同一命令。
- 后续 remote service transport 不需要暴露 Rust concrete type。

风险：

- Tool invocation payload 是 JSON，容易变成无约束通道。
- Mitigation：命令必须携带 `capability_id` / `tool_name` / `resource_scope` / `trace` / `policy hints`，并由 service provider 做 admission validation。

### Resource Manager / Mediator

Driver process、browser/MCP session、workspace lock、stdio child process 通过 resource scope 协调。首版可用 service-level lease/lock adapter，后续接入正式 Resource Service。

优点：

- 多实例 browser/MCP 和 driver process 不再互相踩状态。
- 统一表达 session/app/agent/call 级生命周期。
- 解决 skill-backed MCP 与 global MCP 的冲突。

风险：

- 正式 Resource Service 尚未独立完成，S6 不能阻塞。
- Mitigation：先在 command/result 中建模 `resource_scope`、`lease_id`、`cleanup_policy`，runtime-host provider 内使用现有 `McpSessionLease` / driver session primitives。

### State

Driver session、MCP server lifecycle、skill snapshot cache 都是 State / Memento：

- Driver：unloaded / loaded / running / failed / cleaning / stopped。
- Skill：source_discovered / snapshot_built / executable_loaded / unavailable / policy_denied。
- MCP：registered / dependency_missing / ready / attached / failed / stopped。

优点：

- Web/CLI 能显示状态而不读内部 registry。
- Failure/unavailable 结构化，不 panic、不 hang。
- 和 ServiceRuntime lifecycle snapshot 组合。

风险：

- 状态模型过细会拖慢实现。
- Mitigation：首版只稳定服务状态、provider status、per-definition status、tool exposure status；细粒度 process telemetry 后续扩展。

### Null Object

缺少 driver、skill、MCP 或被 policy 禁用时返回 unavailable provider/client：

- Driver unavailable：inventory empty、tool invoke returns structured unavailable。
- Skill unavailable：snapshot empty with diagnostics、execute returns structured unavailable。
- MCP unavailable：probe dependency_missing / disabled，不阻塞 toolkit build。

优点：

- Base OS 不依赖具体 driver/MCP/skill package。
- Web/CLI 可以安全显示 service unavailable。
- 符合 optional module 缺失行为治理。

风险：

- Null Object 如果假装成功，会掩盖配置错误。
- Mitigation：所有 unavailable result 必须带 reason、service id、trace id、scope 和 status。

### Observer

所有关键节点记录 structured logs/events：

- driver service register/load/reload/invoke/status/cancel/cleanup。
- skill service discover/snapshot/load executable/invoke/status。
- MCP service register/probe/attach/start/stop/cleanup。
- policy denied、dependency missing、resource conflict、tool conflict。

优点：

- 支撑 RC-DRIVER-001、RC-SKILL-001、RC-TRACE-001。
- 前端 trace viewer 可以显示 driver/MCP/skill 真实来源。
- 审计系统可以复盘 tool capability call。

风险：

- 事件 payload 可能泄漏 tool input/output、env、headers、token。
- Mitigation：默认事件只记录 counts、ids、redacted metadata、hash/status；完整 tool I/O 继续走已有安全 trace channel。

### Specification

用规格对象验证：

- capability permission。
- app/session/agent scope。
- tool allow/deny policy。
- required binaries / env / dependency readiness。
- resource scope conflict。
- encrypted skill entitlement readiness。

优点：

- 避免 Web、framework、runtime-host 各自写 if/else。
- 可测试、可扩展、可审计。
- 与 Route C dependency gate 的 Specification 思路一致。

风险：

- S6 一次性落完整 policy engine 会跨阶段。
- Mitigation：首版把 validation 放在 command constructor/provider admission，复杂企业 policy 以 hook/strategy seam 预留。

## 可选方案

### 方案 A：只加 service descriptor，不迁移调用路径

做法：

- 扩展 driver/skill/MCP descriptor 和 DTO。
- 不接入 runtime-host provider，不迁 Web toolkit。

优点：

- 变更小，风险低。

缺点：

- 不满足 S6“服务化与模块化”的目标。
- Web 仍是 provider construction hub，allowlist 无法收敛。

结论：拒绝。S0/S1/S5 已证明 skeleton-only 不足以推进 Route C。

### 方案 B：建立三套完整 service contract + runtime provider + SDK client，再逐步迁 Web toolkit

做法：

- 在 driver/skill/runtime-host 定义 provider-neutral contract 与 provider wrapper。
- 在 SDK 新增 focused clients。
- Web startup 注册 Driver/Skill/MCP service。
- Web toolkit 先把 inventory/status/probe/attach path 迁到 clients，再迁 tool invocation。
- 旧 direct runtime fields 保留 deprecated。

优点：

- 与 S5 模式一致，可审查、可回滚。
- 实现粒度适中，能真正减少 direct runtime coupling。
- 保留现有 driver/skill/MCP 行为。

缺点：

- 修改面跨 driver、skill、runtime-host、sdk、web、integration tests。
- 需要处理 tool trait object 无法直接跨 service payload 的问题。

结论：推荐。首版可以把 toolkit attach 作为 host-local adapter：service 返回 provider-neutral tool catalog / invocation proxy metadata，Web 用 service-backed tool adapter 调 `SystemDriverClient` / `SystemSkillClient` / `SystemMcpClient`。

### 方案 C：引入一个通用 Tool Service 覆盖 Driver/Skill/MCP

做法：

- 所有 driver/skill/MCP tool 都归一为 `ToolService`。
- Web toolkit 只注册 `ServiceBackedToolAdapter`。

优点：

- 上层非常简单。
- Tool invocation 统一。

缺点：

- Driver lifecycle、Skill discovery、MCP transport/session 生命周期差异被隐藏，最后会在 Tool Service 内部变成巨型 if/else。
- 不利于模块化 provider 和 package/store 生命周期。

结论：暂不采用。S6 可以定义 common `CapabilityToolDescriptor` DTO，但服务 ownership 仍归 Driver / Skill / MCP。

### 方案 D：先迁 MCP Service，再迁 Driver/Skill

做法：

- MCP 已在 runtime-host，优先服务化。
- Driver/Skill 后续单独阶段。

优点：

- MCP 当前离 ServiceRuntime 最近。

缺点：

- Skill-backed MCP 依赖 Skill snapshot，Driver tools 与 framework toolkit 同样在 Web 聚合。
- 只迁 MCP 会留下 toolkit 多头聚合，无法解决 Web provider hub。

结论：不作为主方案。MCP 可作为第一个实现切片，但 S6 plan 必须覆盖 Driver/Skill/MCP 全部。

## 推荐方案

采用方案 B，以 **三服务 + common capability tool DTO + host-local service-backed tool adapter** 推进：

1. Driver、Skill、MCP 分别拥有 service contract、descriptor、snapshot/status/result DTO。
2. runtime-host 提供 `DriverSystemServiceProvider`、`SkillSystemServiceProvider`、`McpSystemServiceProvider`。
3. SDK 提供 `SystemDriverClient`、`SystemSkillClient`、`SystemMcpClient` 和 unavailable clients。
4. Web startup 注册内置 provider；Web state 保存 service clients 为主路径，direct runtime fields 标记 deprecated。
5. Framework toolkit 通过 service clients 获取 tool catalog，并注册 service-backed tool adapter；tool invoke 再走 service command。
6. 旧 runtime/facade/registry API 不删除，只 deprecated 或保留为 provider implementation detail。

## 主要风险与缓解

| 风险 | 影响 | 缓解 |
| --- | --- | --- |
| Tool trait object 无法跨 service payload | Web toolkit 迁移卡住 | service 返回 `CapabilityToolDescriptor`，Web 注册 service-backed adapter，执行时发 command |
| Driver/MCP streaming trace 丢失 | RC-DRIVER-001 回归 | command/result 保留 trace id，provider wrapper bridge 到现有 trace/event channel |
| Skill-backed MCP 与 global MCP 生命周期重复 | 重复工具、资源冲突 | MCP service 统一 register/probe/attach，skill service 只提供 snapshot 和 MCP definition source |
| 过早删除 direct runtime | 破坏现有 Web/API | direct fields/API 保留 deprecated，迁移完成后再删 allowlist |
| Runtime-host 变成 provider hardcode hub | 违背微内核 | provider wrapper 接收 facade/registry handles；factory context 不写 app/driver/server 特例 |
| 文件超 500 行 | 违反项目规则 | 按 service_contract、provider、client、tool_adapter、events 拆文件 |
| Policy/entitlement 未完整实现 | capability call 不可审计 | 首版使用 ServiceRuntime trace/policy decorator + command validation，entitlement hook 预留 |
| Cargo 依赖 allowlist 无法立即删除 | 架构债务继续存在 | 每个 allowlist 行绑定切片和过期条件；只在 cargo metadata 证明边消失后删除 |

## 计划输出

本 brainstorm 对应执行计划：

- `docs/superpowers/plans/2026-05-09-s6-driver-skill-mcp-serviceization-plan.md`

建议 OpenSpec change id：

- `add-driver-skill-mcp-services-v1`
