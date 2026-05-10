# S7 Application Framework 服务化 Brainstorm

## 背景

S7 来自 `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`，目标是让 YAML、WASM、GenUI、headless application 都通过 Application Service 生命周期运行，而不是由 `macaca-web` 直接加载、解释和启动。

必须严格遵守：

- `macaca/docs/agent-os-microkernel-boundaries.md`
- `macaca/docs/route-c-serviceization-allowlist.md`
- `macaca/docs/route-c-architecture-governance.md`
- `macaca/docs/route-c-regression-matrix.md`

当前诊断：

- `macaca-app` 已有 `AppRuntime`、`AppRuntimeBuilder`、`ApplicationRuntimeFactory`、`AppLoader`、`AppRegistry`、`ApplicationAbiAdapter`、`ApplicationAbiInstance`、`ApplicationHost`、`ApplicationLifecycle`、`GenUiRuntime`。
- `AppRuntime` 仍直接接收 `Kernel`，并通过 `MacacaSdk::for_kernel(kernel).register_config(...)` 注册 YAML agent。
- `AppLoader` 当前直接拒绝 `L2Wasm`，但 Application ABI 已有 metadata-only WASM adapter 和 runtime-unavailable result。
- `macaca-web` startup 仍直接 discover apps、start all apps、收集 `app_dirs` / `skills_dirs` / `started_apps`，并用这些信息继续组装 skill、driver、executor、framework runtime。
- `/api/chat/v2` 仍从 Web registry 读取 app manifest、entry agent、agent names，并在新 session 时直接 cleanup/ensure executor。
- `genui_routes` 当前是 thin shell 适配器，但 GenUI surface 仍是 fallback，不通过 Application Service 查询 application-owned surface。
- `macaca-sdk` 已有 `SystemServiceClient` 和 focused clients 的模式，但还没有 `SystemApplicationClient`。
- `macaca-runtime-host` 已有 `ServiceRuntime`、provider wrapper、trace/policy decorator，S7 可以复用 S1/S5/S6 的服务化模型。

S7 不包含：

- Gateway Service 外部入口，属于 S8。
- Store/Entitlement 完整服务化，属于 S9；S7 只保留 package guard / entitlement hook，不实现商店语义。
- Payment/A2A、Web3/EVM optional module 真实化。
- Web/CLI 完全 thin shell，属于 S12；S7 只迁移 Application Framework 生命周期相关入口。
- 删除旧 `AppRuntime` / `AppLoader` / Web registry 兼容字段；旧路径应先标记 deprecated 并作为迁移锚点保留。

## 设计模式候选

### Facade

建立 `ApplicationService` 与 SDK `SystemApplicationClient`：

- discover/list application
- load manifest/package metadata
- start application
- stop/remove application
- status/snapshot
- session start/resume/stop command
- host command / GenUI surface command

优点：

- Web/CLI/Gateway 不再直接解释 application runtime 细节。
- `macaca-app` 保持 Application Framework 语义归属，runtime-host 只负责 service provider lifecycle。
- 和 S3/S5/S6 focused client 模型一致。

风险：

- Facade 过大可能变成新的宏服务。
- Mitigation：首版只覆盖 Application Framework 生命周期和 app-facing host command，不吸收 Task/Driver/Skill/MCP/Gateway 业务能力。

### Adapter / Bridge

把现有 `AppRuntime`、`AppRegistry`、`AppLoader`、`ApplicationHost` 适配为 runtime-host `ApplicationSystemServiceProvider`。

优点：

- 复用已有 YAML manifest、agent config、ABI、lifecycle 行为。
- 保持 additive-first，不重写 `/api/chat/v2` 的 framework runner。
- 未来 WASM runtime、remote application host、package-installed app 可以替换 adapter。

风险：

- 如果 service provider 直接持有 `Kernel` 并注册 agent，会保留一部分宏内核耦合。
- Mitigation：首版允许 provider adapter 接收现有 handles 作为兼容输入，但 contract 层不暴露 `Kernel`；旧 direct path 标记 deprecated，后续阶段把 agent registration 再收敛到 Agent/Task/Application service seam。

### Abstract Factory

用 `ApplicationRuntimeFactory` / `ApplicationServiceProviderFactory` 创建 YAML、WASM metadata-only、headless、GenUI-capable provider。

优点：

- 内置 YAML 只是默认 provider，不是硬编码控制流。
- package/Store/Plugin 后续可以按 manifest 生成 provider。
- 运行时选择可以基于 package runtime kind / ABI declaration，而不是 app name。

风险：

- 一次性实现完整 package factory 会跨到 S9/S13。
- Mitigation：S7 只实现 factory seam 和 built-in YAML/WASM metadata adapters；package install/start 仍交给后续 Store phase。

### Command

所有 Application Service 操作使用 typed command，然后转为 `ServiceCommand`：

- `ApplicationDiscoverCommand`
- `ApplicationStartCommand`
- `ApplicationStopCommand`
- `ApplicationRemoveCommand`
- `ApplicationStatusCommand`
- `ApplicationSnapshotCommand`
- `ApplicationSessionStartCommand`
- `ApplicationSessionResumeCommand`
- `ApplicationHostDispatchCommand`
- `ApplicationGenUiSurfaceCommand`

优点：

- 每个入口携带 trace、application/session scope、policy hints，便于审计。
- Web/CLI/Gateway 都可以复用相同命令。
- 未来 remote service transport 不暴露 Rust concrete type。

风险：

- `ApplicationSessionStartCommand` 容易把 chat prompt、task command、agent reply 语义拉进 Application Service。
- Mitigation：Application Service 只拥有 session lifecycle envelope、entry agent resolution、executor readiness；prompt execution 仍属于 framework/task/LLM 服务链路。

### State

Application lifecycle 使用既有 `ApplicationLifecycle` / `ApplicationLifecycleState`：

- Declared
- Initialized
- Started
- Paused
- Resumed
- ShuttingDown
- Stopped
- Failed

同时保留 `AppStatus` 兼容 view。

优点：

- YAML/WASM/headless/GenUI 使用统一生命周期状态。
- status/snapshot 可以给 Web/CLI 显示，不读内部 registry。
- Failed/unavailable 结构化，不 panic、不 hang。

风险：

- 现有 `AppStatus` 和 ABI lifecycle state 可能双轨。
- Mitigation：S7 contract 明确 ABI lifecycle 是 service truth，`AppStatus` 是 deprecated compatibility projection。

### Specification

用规格对象集中验证：

- manifest validity
- ABI declaration compatibility
- permission/capability declaration
- package runtime kind
- trace presence
- application/session/agent scope
- WASM execution availability

优点：

- 避免 Web、runtime、loader 各自复制 if/else。
- 可测试且可扩展。
- 与 Route C dependency gate 的 Specification 思路一致。

风险：

- 过度抽象会拖慢首版。
- Mitigation：先抽 `ApplicationAdmissionSpec` / `ApplicationRuntimeKindSpec` 小规格，覆盖启动、host command、WASM unavailable；复杂企业 policy 留 hook。

### Observer

关键节点必须有 structured logs/events：

- application discover
- manifest loaded
- ABI descriptor loaded
- lifecycle transition
- start/stop/remove
- session start/resume/stop
- host command dispatch
- GenUI surface query/render/event
- unavailable/failure

优点：

- 满足 Route C “无 trace 不执行”。
- 支撑 RC-APP-001、RC-CHAT-001、RC-GOAL-001、RC-TRACE-001。
- 后续审计可以复盘 app lifecycle。

风险：

- logs 可能泄漏 prompt、manifest body、host command payload。
- Mitigation：日志只记录 ids、counts、status、runtime kind、trace id；不记录 prompt body、full manifest、secret、raw payload。

### Null Object

WASM execution、missing app service、missing GenUI surface 返回结构化 unavailable：

- WASM metadata-only：descriptor 可加载，execution returns runtime_unavailable。
- missing application service：SDK client returns unavailable。
- no application-provided GenUI surface：returns empty/unavailable surface view。

优点：

- base OS 不因为 optional runtime 缺失失败。
- Web startup 不因某个 application 不可执行而整体挂掉。
- 明确区分 unavailable 与 success。

风险：

- Null Object 如果被误认为已启动，会掩盖真实配置错误。
- Mitigation：result 必须带 service id、runtime kind、reason、trace id、status。

### Memento

Application snapshot / checkpoint 保存可恢复状态：

- application id/name/version/runtime kind
- lifecycle state
- entry agent
- registered agent ids/names
- session ids
- package id/runtime kind
- sanitized ABI descriptor

优点：

- session resume 和 Web reload 能从 service snapshot 恢复。
- 后续 app upgrade / package restart / WASM checkpoint 有扩展点。
- 不需要 Web 扫描整个 registry 推断状态。

风险：

- snapshot 可能变成大对象。
- Mitigation：默认 snapshot 只含 metadata 和 counts；full manifest/agent config 通过单独 debug command 且需权限。

## 可选方案

### 方案 A：只增加 Application Service descriptor，不迁移 Web startup/chat

做法：

- 在 `macaca-app` 加 service descriptor / DTO。
- Web 继续直接 `AppRegistry::discover_apps` 和 `AppRuntime::start_app_from_file`。

优点：

- 变更小。
- 不影响当前启动流程。

缺点：

- 不满足 S7 “Application 不再由 Web 直接加载和解释”。
- `macaca-web -> macaca-app` 仍是事实协调中枢，allowlist 无法收敛。
- 后续 S12 thin shell 仍会被 Web startup 胶水阻塞。

结论：拒绝。只能作为第一小步，不能作为 S7 完成定义。

### 方案 B：建立 Application Service contract/provider/client，并迁移 Web startup 的 discover/start/status 到 service

做法：

- `macaca-app` 定义 Application Service typed commands/results 和 sanitized snapshot。
- `macaca-runtime-host` 新增 `ApplicationSystemServiceProvider`，内部适配 `AppRegistry`、`AppRuntime`、`ApplicationHost`。
- `macaca-sdk` 新增 `SystemApplicationClient`。
- `macaca-web` startup 注册并启动 Application Service，通过 client discover/start apps，旧 direct registry/runtime 字段保留 deprecated。

优点：

- 满足 S7 核心目标。
- additive-first，能保留现有 `AppRuntime` 和 `/api/chat/v2`。
- 与 S5/S6 模型一致，风险可控。

缺点：

- Web 仍需要兼容读取 registry/app dirs/skills dirs，短期不能删除 `macaca-web -> macaca-app` 依赖。
- `AppRuntime` 内部仍注册 kernel agent，需要后续更深的 agent/application service seam 才能彻底解耦。

结论：推荐。符合当前 Route C 渐进式重构节奏。

### 方案 C：一次性把 `/api/chat/v2`、session lifecycle、executor registry、framework runner 全部迁入 Application Service

做法：

- Application Service 直接处理 chat session start/resume、coordinator build、executor lifecycle、SSE event。
- Web 只转发 HTTP 到 Application Service。

优点：

- Web thin shell 目标推进最大。
- Application lifecycle 边界最完整。

缺点：

- 跨越 S4/S5/S6/S7/S12，blast radius 很大。
- 容易把 Task/LLM/Driver/Skill/MCP 业务调用又聚合进 Application Service，形成新宏服务。
- `/api/chat/v2`、goal resume、trace 实时推送风险高。

结论：拒绝。应拆到后续 S12 或专门 chat/session orchestration service slice。

### 方案 D：把 `AppRuntime` 迁到 `macaca-runtime-host`，`macaca-app` 只保留 DTO

做法：

- Runtime-host 拥有 application runtime、registry、loader。
- `macaca-app` 只保留 manifest/ABI。

优点：

- 所有 runtime provider 都集中到 host。

缺点：

- 违反 Application Framework 边界：`macaca-app` 应拥有 application runtime 语义。
- runtime-host 会变成巨型宏宿主。
- 迁移成本高，收益不如 provider wrapper。

结论：拒绝。runtime-host 只拥有 service provider wrapper，不拥有 application semantics。

## 推荐方案

采用方案 B：`macaca-app` 拥有 Application Service contract 与 Application Framework 语义，`macaca-runtime-host` 通过 Adapter/Bridge 暴露 host-owned `ApplicationSystemServiceProvider`，`macaca-sdk` 提供 `SystemApplicationClient`，`macaca-web` 迁移 startup/status/chat-preflight 到 client-first 路径。

推荐边界：

- `macaca-app`：manifest、loader、registry、runtime、ABI、host、lifecycle、service DTO、adapter。
- `macaca-runtime-host`：`ApplicationSystemServiceProvider`，负责把 `ServiceCommand` 转 typed command 并委托 `AppRuntime` / `AppRegistry` / `ApplicationHost`。
- `macaca-sdk`：`SystemApplicationClient`，只调用 service，不构造 runtime/registry/kernel。
- `macaca-web`：HTTP/SSE adapter。startup 可以组合 built-in provider，但业务路径优先用 application client。旧 direct runtime/registry 保留 deprecated。
- `macaca-kernel`：不新增 application provider 依赖，不承载 application lifecycle。

## 主要风险与缓解

| 风险 | 影响 | 缓解 |
| --- | --- | --- |
| `AppRuntime` 直接注册 kernel agent | Kernel/provider 耦合短期仍存在 | S7 contract 不暴露 Kernel；provider adapter 内保留兼容，标记 deprecated；后续 agent registration service 化再移除 |
| Web startup 依赖 `app_dirs` / `skills_dirs` | Web 仍保留部分 app interpretation | Application Service start result 返回 sanitized app dirs / skill dirs / agent names view；Web 只消费 view |
| WASM currently rejected by loader | S7 要求 WASM metadata-only | Service path 使用 ABI/package metadata-only admission，legacy `AppLoader` direct path 保持 deprecated rejection |
| Application Service 变成巨型 chat service | 新宏服务 | 只拥有 lifecycle/session envelope/host command，不执行 LLM/task/tool business logic |
| Trace/log 泄漏 prompt/manifest | 安全风险 | logs 只记录 ids/counts/status/runtime kind；禁止 raw prompt/full manifest/secret |
| Breaking `/api/chat/v2` | 用户可见回归 | 先迁 startup/status，chat handler只做 preflight/status client-first，不把 coordinator execution 移入 service |
| 文件超过 500 LOC | 违反 AGENTS.md | service contract、adapter、client、provider 分文件；大实现拆 command/result/status/snapshot |

## OpenSpec 建议

建议 change id：

- `add-application-framework-service-v1`

建议 delta specs：

- `application-service`
- `application-runtime-host-provider`
- `application-sdk-client`
- `application-web-adapter`

提案必须声明：

- S7 是 additive-first。
- YAML apps、`/api/chat/v2`、session trace、goal resume、driver/skill/MCP service-backed toolkit 不退化。
- 旧 direct `AppRuntime` / `AppLoader` / Web registry/runtime 字段保留但标记 deprecated。
- Service calls require trace/policy scope。
- WASM execution remains structured unavailable, not panic/not hidden success。
- No app name/workflow/provider hardcode in service control flow.

