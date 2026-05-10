# Route C 服务化依赖门禁 Allowlist

## 1. 目的

本文档记录 S0 依赖边界门禁中暂时允许通过的既有依赖债务。

Allowlist 不是架构批准。它只是当前迁移状态的 Memento：说明哪条依赖违反 Route C 微内核边界、为什么还存在、未来应该迁到哪个 service/facade、在哪个阶段过期。

新增例外必须先更新 OpenSpec，并同步更新 `macaca/crates/tests/macaca-integration-tests/tests/route_c_dependency_boundaries.rs` 中的测试内 allowlist。禁止只改代码或只改文档。

## 2. 执行门禁

可执行门禁位于：

- `macaca/crates/tests/macaca-integration-tests/tests/route_c_dependency_boundaries.rs`

Workspace topology 另有独立门禁：

- `macaca/crates/tests/macaca-integration-tests/tests/route_c_workspace_topology.rs`

Topology 门禁只校验 crate 是否位于正确的 Route C filesystem layer，不代表依赖边已经被允许或迁移完成。

运行方式：

```bash
cargo test -p macaca-integration-tests route_c_dependency_boundaries
```

门禁使用 `cargo metadata --no-deps --format-version 1` 读取 workspace 直接依赖边，并按 Route C layer 评估 forbidden rules。未知 workspace crate 会失败，要求先通过 OpenSpec 明确分层。

## 3. Allowlist 字段

| 字段 | 含义 |
| --- | --- |
| Rule id | 触发的边界规则 |
| From crate | 依赖发起 crate |
| To crate | 被依赖 crate |
| Current reason | 当前保留原因 |
| Replacement service/facade path | 目标替代路径 |
| Target migration phase | 计划迁移阶段 |
| Expiry condition | 何时删除该 allowlist |
| Owner/status | 责任归属和当前状态 |

## 4. 当前迁移债务

| Rule id | From crate | To crate | Current reason | Replacement service/facade path | Target migration phase | Expiry condition | Owner/status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `kernel-no-provider-deps` | `macaca-kernel` | `macaca-driver` | Kernel 当前仍承载 driver execution 兼容路径。 | Driver Service facade | S6 | Driver 调用全部经 ServiceRuntime 或 SystemFacade。 | Route C / active debt |
| `kernel-no-provider-deps` | `macaca-kernel` | `macaca-gateway` | Kernel 当前仍保留 gateway 兼容注册和协调入口。 | Gateway Service facade | S8 | Gateway provider 通过 service/plugin 注册，不再由 kernel 直接依赖。 | Route C / active debt |
| `kernel-no-provider-deps` | `macaca-kernel` | `macaca-persist` | Kernel 当前仍直接使用持久化能力保存系统状态。 | Persistence Service contract | S1/S2 | Kernel 只依赖 persistence contract 或 service facade。 | Route C / active debt |
| `kernel-no-provider-deps` | `macaca-kernel` | `macaca-skill` | Kernel 当前仍保留 skill/MCP 兼容路径。 | Skill/MCP Service facade | S6 | Skill/MCP 通过 service/plugin runtime 接入。 | Route C / active debt |
| `kernel-no-provider-deps` | `macaca-kernel` | `macaca-task` | Kernel 当前仍保留 task primitive 与 task service 混合路径。 | Task Service facade | S4 | Planner/worker/review 全部迁到 Task Service。 | Route C / active debt |
| `kernel-no-provider-deps` | `macaca-kernel` | `macaca-tools` | Kernel 当前仍保留 tools 调用兼容入口。 | Tool/Skill Service facade | S6 | 工具能力通过 Skill/Tool Service 调用。 | Route C / active debt |
| `presentation-no-provider-construction-hub` | `macaca-cli` | `macaca-gateway` | CLI 当前仍包含 gateway 启动/检查兼容入口。 | Gateway Service client | S8 | CLI 只通过 SDK/SystemFacade 调 gateway service。 | Route C / active debt |
| `presentation-no-provider-construction-hub` | `macaca-cli` | `macaca-tools` | CLI 当前仍直接访问 tools 兼容能力。 | Tool/Skill Service client | S6 | CLI 工具调用全部经 service client。 | Route C / active debt |
| `presentation-no-provider-construction-hub` | `macaca-web` | `macaca-driver` | Web 当前仍是部分 driver execution 的协调入口。 | Driver Service client | S6 | Web 只订阅 driver trace/state，不构造 driver provider。 | Route C / active debt |
| `presentation-no-provider-construction-hub` | `macaca-web` | `macaca-llm` | Web 当前仍保留 chat/pipeline LLM 兼容路径。 | LLM Service client | S5 | `/api/chat/v2` 经 SystemFacade/service client 调用。 | Route C / active debt |
| `presentation-no-provider-construction-hub` | `macaca-web` | `macaca-memory` | Web 当前仍直接读取部分 memory/session trace 数据。 | Memory/Context Service client | S5 | Web 只请求 session scoped memory/context view。 | Route C / active debt |
| `presentation-no-provider-construction-hub` | `macaca-web` | `macaca-persist` | Web 当前仍直接访问持久化层读取 session/application 状态。 | Persistence Service client | S1/S12 | Web 通过 facade 拉取分页 session 和 trace。 | Route C / active debt |
| `presentation-no-provider-construction-hub` | `macaca-web` | `macaca-skill` | Web 当前仍直接处理 skill/MCP 展示和调用兼容路径。 | Skill/MCP Service client | S6 | Web 只展示 Skill/MCP service state 和 trace。 | Route C / active debt |
| `presentation-no-provider-construction-hub` | `macaca-web` | `macaca-task` | Web 当前仍直接访问 task board/session task 状态。 | Task Service client | S4 | Task board 通过 Task Service 分页 API 获取。 | Route C / active debt |
| `presentation-no-provider-construction-hub` | `macaca-web` | `macaca-tools` | Web 当前仍直接依赖 tools 兼容能力。 | Tool/Skill Service client | S6 | Web 工具相关展示通过 Skill/Tool Service view model。 | Route C / active debt |
| `cli-no-web-internals` | `macaca-cli` | `macaca-web` | CLI 当前复用 Web server startup 兼容路径。 | public Web server-start seam only | S12 | Web server-start seam 抽到 runtime-host 或独立 host facade 后，CLI 不再依赖 Web crate。 | Route C / active debt |

## 4.1 S5 LLM / Memory / Context 迁移状态

S5 已建立 LLM、Memory、Context 的 provider-neutral service contract、runtime-host service provider wrapper、SDK focused clients 和 Web runtime-backed service client。Kernel 对 `macaca-llm` / `macaca-memory` 的直接 Cargo 依赖已移除：LLM legacy provider 类型经 `macaca-agent::LlmProvider` 暴露，kernel memory adapter 兼容实现已退出 kernel 边界。CLI 对 `macaca-llm` 的直接依赖也已移除，兼容 provider 通过 `macaca-agent::LlmProvider` 实现。Allowlist 中 Web 的相关依赖仍未删除，原因是 Cargo 直接依赖边还存在，用于 DTO、兼容字段、显式 memory tools、model selection 兼容路径和 UI/view model。删除剩余 allowlist 必须等依赖门禁和 `cargo metadata` 证明直接边消失。

| Edge | Current S5 status | Remaining debt |
| --- | --- | --- |
| `macaca-web -> macaca-llm` | Framework model dispatch 已经通过 `SystemLlmClient` / LLM Service；旧 provider fields 保留为 deprecated 搜索锚点。 | `llm_router` 仍用于 model selection 兼容解析，后续应迁到 LLM Service model-selection command。 |
| `macaca-web -> macaca-memory` | Context active recall、preflight recall、knowledge digest hot path 已经通过 `SystemMemoryClient` / Memory Service。 | 显式 memory tools 和兼容 `WebMemoryRuntime` 仍保留，后续需要工具服务化或 memory tool client 化后删除。 |
| `macaca-web -> macaca-context` | Context service 已注册，framework context path 已把 memory recall 底层切到 Memory Service。 | `ContextReportingChatModel` 仍在 Web 本地组装 provider catalog 和调用 `ContextFacade`，后续需要完整 `SystemContextClient` / `ContextAssembler` seam。 |

## 4.2 S6 Driver / Skill / MCP 迁移状态

S6 已建立 Driver、Skill、MCP 的 provider-neutral service contract、runtime-host service provider wrapper、SDK focused clients 和 Web runtime-backed service client。Web startup 会注册并启动 Driver/Skill/MCP services；driver status/reload route、MCP status route、Driver/Skill framework tool catalog 和 tool invocation 已优先走 service client。旧 direct runtime 字段保留为 deprecated 搜索锚点。

| Edge | Current S6 status | Remaining debt |
| --- | --- | --- |
| `macaca-web -> macaca-driver` | Driver status/reload route 与 framework driver tool catalog/invocation 已通过 `SystemDriverClient` / Driver Service。 | Web 仍保留 `DriverRuntime` / `DriverRegistry` deprecated 字段作为 startup/provider adapter 兼容锚点，Cargo 直接边仍存在。删除需等 driver runtime 构造移动到 runtime-host composition 或 service factory。 |
| `macaca-web -> macaca-skill` | Skill snapshot cache path、capability catalog 和 framework skill tool catalog/invocation 已通过 `SystemSkillClient` / Skill Service，失败时回退旧 facade。 | Web startup 仍直接加载 knowledge skill catalog 与 executable skill compatibility tools；部分 routes 仍保留 deprecated fallback，Cargo 直接边仍存在。 |
| `macaca-web -> macaca-tools` | Driver/Skill service-backed tool adapter 已替代对应 direct tool registration。 | Web 仍负责 host-local framework `Toolkit` composition、base workspace tools、memory tools、todo tools；该边在 S12 thin shell 或 dedicated Tool Service/Toolkit Service 前不能删除。 |
| `macaca-runtime-host -> macaca-driver` | Runtime-host 新增 `DriverSystemServiceProvider`，这是 service provider ownership，不是 presentation/provider hub。 | 属于 S6 目标架构边，不应放入 presentation/kernel allowlist；后续如引入 remote service，可替换为 provider factory。 |
| `macaca-runtime-host -> macaca-skill` | Runtime-host 新增 `SkillSystemServiceProvider` 与 MCP skill-backed definition conversion，属于 service provider ownership。 | 属于 S6 目标架构边。 |
| MCP host-local Toolkit attach | MCP Service 已支持 register/probe/catalog/status/snapshot/cleanup DTO 与 provider。 | Framework `Toolkit` 是 host-local 可变对象，global MCP 与 skill-backed MCP attach 仍通过 deprecated `McpRuntimeFacade::register_definitions`。过期条件：实现 service-owned toolkit handle/proxy registry 后，Web 不再直接调用 MCP runtime attach。 |

## 4.3 S7 Application Framework 迁移状态

S7 已建立 Application Service provider-neutral DTO、runtime-host provider wrapper、SDK focused client 和 Web runtime-backed service client。Web startup 会注册并启动 Application Service；application discover/start/status/session envelope 和 GenUI surface 查询已优先走 `SystemApplicationClient`。旧 `AppRuntime` startup API、Web `runtime`/`registry` 字段和部分 manifest fallback 保留为 deprecated 搜索锚点。

| Edge | Current S7 status | Remaining debt |
| --- | --- | --- |
| `macaca-web -> macaca-app` | Web application startup、app list/detail/reload、chat entry-agent preflight、session envelope 和 GenUI surface query 已通过 Application Service 优先路径。 | Web 仍需要 `macaca-app` DTO/registry fallback、framework prompt/tool policy manifest reads、skill/MCP overlay reads 和 compatibility fields；Cargo direct edge 仍存在，allowlist 不可删除。 |
| `macaca-runtime-host -> macaca-app` | Runtime-host 新增 `ApplicationSystemServiceProvider`，这是 Application Service provider ownership。 | 属于 S7 目标架构边，不应作为 presentation/kernel allowlist 债务；后续如引入 remote Application Service，可替换为 provider factory。 |
| `macaca-sdk -> macaca-app` | SDK Application client 不依赖 `macaca-app`，只依赖 `macaca-proto` Application Service DTO。 | 目标状态已达成；后续新增 SDK helper 不得重新引入 `macaca-app` direct dependency。 |
| `macaca-app -> macaca-runtime-host` | S7 移除了 app crate 对 runtime-host entitlement facade 的直接依赖，改为本地 authorizer trait。 | 目标状态已达成；后续 app crate 不得反向依赖 runtime-host、Web 或 SDK runtime composition。 |
| Web framework manifest fallback | Chat coordinator、framework runner、toolkit、loop manager、skill MCP 仍有 direct manifest reads。 | 这些路径用于 prompt semantics、context overrides、MCP overlay、tool policy、task decomposition compatibility。过期条件：Application Service 或 dedicated Application Metadata Service 提供 sanitized semantics views 后，Web 不再读取 raw manifest。 |

## 4.4 S9 Store / Entitlement 迁移状态

S9 已建立 Store / Entitlement 的 provider-neutral DTO、runtime-host service provider wrapper、SDK focused clients，并在 Web startup 注册和启动 Store/Entitlement services。`macaca-app` 与 `macaca-skill` 的商业授权入口仍通过本地 authorizer trait 保持 Dependency Inversion；direct Phase 08 helper/facade path 保留为 deprecated compatibility anchor，直到 S12 thin shell 与后续 package manager 迁移证明不再需要。

| Edge | Current S9 status | Remaining debt |
| --- | --- | --- |
| `macaca-web -> macaca-runtime-host` | Web startup 注册 Store/Entitlement built-in providers，并通过 runtime-backed `SystemStoreClient` / `SystemEntitlementClient` 访问服务。 | Web 仍是当前 composition root。过期条件：S12 或后续 host composition 把 provider registration 移到 shared runtime bootstrap，Web 只接收 `SystemFacade`。 |
| `macaca-web -> macaca-persist` | Entitlement service provider 复用当前 Web-owned `EventLog` 和 in-memory entitlement store 进行 audit/metering 兼容。 | Store/Entitlement persistence 应迁到 Store/Entitlement service-owned repository factory 或 Persistence Service。 |
| `macaca-sdk -> Store/Entitlement provider` | SDK 没有 runtime-host/app/skill/web/cli direct dependency，只通过 `SystemServiceClient` 分发 provider-neutral commands。 | 目标状态已达成；后续 SDK 不得引入 provider concrete dependency。 |
| `macaca-app` commercial guard | `CommercialPackageGuard` 通过 `ApplicationEntitlementAuthorizer` trait seam 授权，free/open fast path 保持不变。 | 需要在后续消费迁移中注入 service-backed authorizer 作为默认生产 authorizer，并逐步减少 direct Phase 08 facade 使用。 |
| `macaca-skill` encrypted package hook | `EncryptedPackageLoader` 通过 `EncryptedPackageAuthorizer` trait seam 授权，deny/unavailable 时不会进入 decrypt hook。 | 需要在后续 skill package runtime 中注入 service-backed authorizer，当前 hook 本身保持 provider-neutral。 |
| package manager Web/CLI surfaces | 当前代码库没有独立 Store marketplace UI；Web 已持有 serviceized clients 供后续 route 使用。 | 当 package inspect/install/status API 或 CLI command 增加时，必须使用 `SystemStoreClient` / `SystemEntitlementClient`，禁止新增 direct helper path。 |

## 4.5 S10 Payment / A2A 迁移状态

S10 已建立 Payment / A2A 的 provider-neutral DTO、runtime-host service provider wrapper、SDK focused client，并在 Web startup 注册和启动 Payment Service。旧 `macaca-kernel::A2ACoordinator` / `A2APaymentFacade` / `A2AProtocolAdapter` / `LocalSimulatedA2AAdapter` 已标记为 deprecated compatibility anchor，便于后续消费迁移检索；新生产路径必须优先使用 `PaymentSystemServiceProvider` + `SystemPaymentClient`。

| Edge | Current S10 status | Remaining debt |
| --- | --- | --- |
| `macaca-web -> macaca-runtime-host` | Web startup 注册 built-in local-simulated Payment provider，并通过 runtime-backed `SystemPaymentClient` 访问服务。 | Web 仍是当前 composition root。过期条件：S12 或后续 host composition 把 provider registration 移到 shared runtime bootstrap，Web 只接收 `SystemFacade`。 |
| `macaca-web -> macaca-persist` | Payment Service 当前使用 Web-owned in-memory payment store 作为 bootstrap repository，保持无网络、无真实钱包、无链依赖。 | Payment repository 应迁到 Payment Service-owned repository factory 或 Persistence Service；生产 payment provider 不应由 Web 直接构造。 |
| `macaca-sdk -> Payment provider` | SDK 没有 runtime-host/web/provider concrete dependency，只通过 `SystemServiceClient` 分发 provider-neutral Payment commands。 | 目标状态已达成；后续 SDK 不得引入 payment provider concrete dependency。 |
| `macaca-kernel` A2A compatibility | 旧 kernel A2A coordinator/facade/adapter 保留但已 deprecated，用于测试和迁移检索。 | 后续上层消费必须迁到 `SystemPaymentClient`；当 GitNexus 和 `rg` 证明没有 production direct caller 后，旧接口可进入删除提案。 |
| Payment provider strategy | Runtime-host 新增 `PaymentAdapterStrategy` 与 local simulated adapter，这是 service provider ownership，不属于 kernel/presentation。 | 真实支付、远程 agent 交易、Web3 proof 或 EVM settlement 必须作为 adapter strategy / optional module 接入，不得回写 kernel 或 Web。 |

## 4.6 S11 Web3 / EVM Optional Module 迁移状态

S11 已建立 Web3 / EVM 的 provider-neutral service DTO、runtime-host optional service provider wrapper、SDK focused clients，并在 Web startup 注册和启动默认 unavailable Web3/EVM Services。旧 `macaca-kernel::Web3Facade` / `UnavailableWeb3Adapter` / `MockWeb3Adapter` / `EvmFacade` / `UnavailableEvmAdapter` / `MockEvmAdapter` 已标记为 deprecated compatibility anchor，便于后续消费迁移检索；新生产路径必须优先使用 `Web3SystemServiceProvider` / `EvmSystemServiceProvider` + `SystemWeb3Client` / `SystemEvmClient`。

| Edge | Current S11 status | Remaining debt |
| --- | --- | --- |
| `macaca-web -> macaca-runtime-host` | Web startup 注册 built-in unavailable Web3/EVM providers，并通过 runtime-backed `SystemWeb3Client` / `SystemEvmClient` 访问服务。 | Web 仍是当前 composition root。过期条件：S12 或后续 host composition 把 provider registration 移到 shared runtime bootstrap，Web 只接收 `SystemFacade`。 |
| `macaca-sdk -> Web3/EVM provider` | SDK 没有 runtime-host/web/provider concrete dependency，只通过 `SystemServiceClient` 分发 provider-neutral Web3/EVM commands。 | 目标状态已达成；后续 SDK 不得引入 wallet、RPC、chain、EVM engine 或 provider concrete dependency。 |
| `macaca-kernel` Web3/EVM compatibility | 旧 kernel Web3/EVM facades/adapters 保留但已 deprecated，用于测试和迁移检索。 | 后续上层消费必须迁到 `SystemWeb3Client` / `SystemEvmClient`；当 GitNexus 和 `rg` 证明没有 production direct caller 后，旧接口可进入删除提案。 |
| Web3/EVM provider strategy | Runtime-host 新增 unavailable/mock Web3/EVM providers，这是 optional service provider ownership，不属于 kernel/presentation。 | 真实 chain/RPC/wallet/EVM adapter 必须作为 optional module strategy/plugin 接入，不得回写 kernel 或 Web。 |
| Payment/Web3/EVM integration | S11 不实现链上 payment settlement，也不把 Web3/EVM 作为 Payment provider。 | 后续 integration 必须通过 Payment Service adapter 或 explicit optional module proposal，不得让 Web3/EVM 直接拥有 payment lifecycle。 |

## 4.7 S12 Web / CLI Thin Shell Completion 迁移状态

S12 completion 已引入 runtime-host owned `RouteCOptionalServicesBootstrap`，把 S9-S11 service families（Store/Entitlement、Payment/A2A、Web3/EVM）的 provider registration/startup 从 Web procedural startup 抽到 runtime-host bootstrap boundary。Web 仍提供当前本地 repository/facade handles，并继续通过 runtime-backed SDK clients 访问服务；这一步没有删除 Cargo direct edges，因此 allowlist rows 只能标记为 narrowed，不能删除。

| Edge | Current S12 status | Remaining debt |
| --- | --- | --- |
| `macaca-web -> macaca-runtime-host` | Web 不再直接逐个注册/启动 S9-S11 providers；它调用 `bootstrap_route_c_optional_services` 并接收 sanitized diagnostics。 | Web 仍拥有 overall host composition 和 S5-S8 provider startup inputs。过期条件：Application/LLM/Memory/Context/Driver/Skill/MCP startup 也迁入 typed host bootstrap，Web 只接收 service clients/facade。 |
| `macaca-web -> macaca-persist` | S9/S10 repositories 仍由 Web startup 创建并传入 runtime-host bootstrap，保持现有 storage behavior。 | Persistence Service 或 service-owned repository factory 尚未完成；直接边不可删除。 |
| `macaca-cli -> macaca-web` | CLI `web` command 现在只调用公开 `WebServerBuilder` server-start seam，并用注释和日志明确禁止复制 Web runtime/provider/session/service 语义。 | 该 Cargo edge 仍存在，只能标记为 server-start-only narrowed debt。过期条件：Web server-start seam 抽到 runtime-host 或独立 host facade 后删除 direct edge。 |
| Deprecated Web provider/runtime fields | 本次切片未删除 `AppState` deprecated compatibility anchors。 | 后续 route/toolkit/session slices 需要逐项迁到 SDK focused clients，并用 guard 防止新增 direct callers。 |

## 5. 新增例外流程

1. 创建或更新 OpenSpec change，解释为什么短期不能立即迁移。
2. 在本文件新增 allowlist 行，必须填写迁移阶段和过期条件。
3. 在依赖门禁测试的 test-local allowlist 中新增同等记录。
4. 运行 `openspec validate <change-id> --strict`。
5. 运行 `cargo test -p macaca-integration-tests route_c_dependency_boundaries`。

优先删除 allowlist 行，而不是延长例外。任何无法说明替代 service/facade path 的新增依赖，都不应进入 Route C 主线。
