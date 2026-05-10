# Macaca OS 路线 C 架构治理规则

## 1. 治理目标

路线 C 的目标是让 Macaca 成为微内核 Agent OS，而不是宏内核 Agent 平台。治理规则用于阻止后续实现中出现短期可跑、长期不可维护的捷径。

## 2. 架构层级规则

### 2.1 Kernel Rule

Kernel 只负责系统不变量。新增能力进入 kernel 前必须证明：

- 所有 application 都无法绕开该能力。
- 该能力不依赖具体 provider。
- 该能力不包含业务 workflow。
- 该能力不会因为第三方生态扩展频繁变化。

### 2.2 Service Rule

如果能力可以被替换 provider、第三方实现或远程实现替换，它必须是 system service。

### 2.3 Plugin Rule

如果能力由第三方扩展系统表面，它必须通过 plugin manifest、service registry、permission、trace、lifecycle 接入。

### 2.4 Optional Module Rule

Web3、EVM、特定 gateway、特定 driver、特定 paid package 都必须是可选模块。缺失时返回结构化 unavailable，不影响 base OS。

### 2.5 Presentation Rule

Web/CLI/Frontend 只能是 shell 和 adapter，不得定义核心 session、task、trace、payment、package 语义。

## 3. 设计模式规则

优先使用：

- Facade：隐藏底层系统复杂度。
- Adapter / Bridge：隔离外部 provider、driver、gateway、MCP、Web3。
- Strategy：调度、policy、provider、计费策略可替换。
- Command：service call、UI event、payment intent、contract call。
- Observer：trace/audit/event stream。
- State：session、task、plugin、payment lifecycle。
- Memento：session checkpoint、task history、receipt、event replay。
- Specification：manifest、permission、compatibility、entitlement validation。

禁止用大段 if/else 判断 app/provider/driver/gateway 名称来替代可扩展模式。

## 4. 可观测性规则

以下动作必须有 trace：

- app lifecycle。
- agent lifecycle。
- task lifecycle。
- service call。
- driver call。
- skill/MCP call。
- plugin lifecycle。
- entitlement check。
- payment intent。
- Web3 transaction。
- EVM contract call。
- UI event。

无 trace，不执行。

## 5. 权限规则

所有 capability call 必须经过 policy：

- application permissions。
- plugin permissions。
- user approvals。
- spending budgets。
- region compliance。
- optional module availability。

无权限，不调用。

## 6. 兼容规则

- 现有 YAML application 在正式迁移路径出现前必须保持一等支持。
- 现有 `/api/chat/v2`、trace、task board、resume 链路不得退化。
- 新 package/ABI/Store/Web3 能力必须 additive-first。

## 7. 依赖边界门禁

路线 C S0 引入可执行依赖边界门禁：

- 测试文件：`macaca/crates/macaca-integration-tests/tests/route_c_dependency_boundaries.rs`
- Allowlist 文档：`macaca/docs/route-c-serviceization-allowlist.md`
- 运行命令：`cargo test -p macaca-integration-tests route_c_dependency_boundaries`

该门禁使用 `cargo metadata --no-deps --format-version 1` 检查 workspace 直接依赖边，并用 Specification + Visitor 的方式执行以下治理规则：

- Kernel 不得新增 direct provider dependency。
- Web/CLI 等 presentation shell 不得新增 provider construction hub 依赖。
- CLI 不得长期依赖 Web internals。
- Optional module 不得成为 base OS 必需依赖。
- Service provider 不得反向依赖 presentation shell。

当前仍存在的宏内核式依赖必须显式写入 allowlist。Allowlist 不是架构批准，只是迁移债务快照；每一行都必须包含 rule id、from crate、to crate、当前原因、替代 service/facade path、目标迁移阶段、过期条件和 owner/status。

新增例外必须先走 OpenSpec，并同步更新 allowlist 文档和测试内 allowlist。禁止只在代码中静默放行新增依赖。

## 8. ServiceRuntime 治理

路线 C S1 引入 host-owned `ServiceRuntime`。它的职责是运行时编排，而不是 provider 迁移：

- `ServiceRuntime` 归属 `macaca-runtime-host`，负责 provider-neutral service 的注册、启动、调用、停止、清理、事件和 snapshot。
- Kernel 仍只拥有 service registry、service call adapter、trace/audit primitive 等系统不变量，不拥有 provider runtime orchestration。
- Runtime service call 必须先经过 trace-required decorator 和 policy decorator，再进入 service bus。
- Runtime policy 必须是 Strategy，可以被后续权限、预算、地域、entitlement、optional module availability 策略替换。
- Runtime decorator chain 必须是可组合的，后续 resource、entitlement、metering enforcement 通过 decorator 增加，不得写进 provider-specific 分支。
- `ServiceRuntime` 必须通过 descriptor/factory 注册 service，不得硬编码 app、workflow、provider、driver、gateway、model、chain 或业务名称。
- Task、LLM、Memory、Driver、Skill、MCP、Gateway、Payment、Web3、EVM 等 provider 迁移发生在后续 S 阶段，S1 不移除 allowlist 债务。

## 9. Kernel Provider Compatibility

路线 C S2 开始收敛 `macaca-kernel` 的 provider 依赖面：

- `Kernel` 和 `KernelBuilder` 可以暂时保留 deprecated 兼容入口，但新构造逻辑应优先通过 provider compatibility bundle、facade 或后续 `ServiceRuntime` 路径。
- provider-facing 兼容代码必须隔离在专门的 compat 边界中，不得回流成 kernel core 的默认架构。
- deprecated 构造入口必须保留在代码库中以便迁移检索，但新生产代码不应再以它们作为默认路径。
- 当 compat 边界不再需要某个 provider crate 时，应优先删除对应依赖而不是继续扩大 allowlist。

## 10. SDK/SystemFacade 收敛

路线 C S3 将 Web/CLI 等 presentation shell 收敛到 SDK-owned `SystemFacade`：

- `SystemFacade` 只能作为 shell-facing Facade，负责组合 command-driven capability clients，不得构造 provider、driver、gateway、application workflow 或业务专用实现。
- SDK client 必须按能力拆分为 Task、Status、Service、Trace、Package 等 Strategy 边界，后续 S4-S12 可以逐个替换为 `ServiceRuntime` 或远程 service adapter。
- Web/CLI 只能负责输入解析、输出格式化、HTTP/terminal/SSE 映射和 presentation logging，不得定义 session、task、trace、service、package 核心语义。
- 尚未迁移到具体 service 的能力必须返回结构化 empty 或 unavailable，不得 panic、阻塞等待或静默调用 provider crate。
- SDK 命令类型必须可序列化、可审计、provider-neutral，并在关键执行节点记录 trace/log，方便 Route C regression 和 GitNexus blast-radius 审查。
- S3 不迁移 PlanLoop/WorkerLoop/review、LLM/Memory/Context、Driver/Skill/MCP、Application lifecycle、Gateway、Payment、Web3、EVM provider 行为；这些由后续阶段单独治理。

## 11. LLM / Memory / Context Service Ownership

路线 C S5 将 LLM、Memory、Context 收敛为可替换 system service，而不是 presentation shell 或 kernel 内置 provider：

- `macaca-llm` 只拥有 provider-neutral LLM command/result/snapshot DTO 和领域 adapter，不依赖 kernel、runtime-host、Web 或 CLI。
- `macaca-memory` 只拥有 MemoryScope、AgentPrivate、SessionShared、topology labels、remember/recall/prefetch/forget/status/snapshot DTO 和 memory fabric 策略。
- `macaca-context` 只拥有 context assemble、active recall orchestration、provider/engine inventory、context report DTO 和 context engine strategy。
- `macaca-runtime-host` 拥有 LLM/Memory/Context 的 `SystemService` provider wrapper、service lifecycle、trace-required dispatch、policy/decorator chain 和 snapshot emission。
- `macaca-sdk` 只提供 `SystemLlmClient`、`SystemMemoryClient`、`SystemContextClient`、`SystemFacade` 这类 shell-facing client/facade，不构造 provider、memory backend 或 context engine。
- Web、CLI、framework 只能作为 adapter：Web 负责 HTTP/SSE/UI 映射，CLI 负责 terminal 输出，framework 负责 ChatModel/ContextAssembler seam，不得定义 LLM provider、memory store、context engine 的核心语义。
- 缺少 runtime-backed service 时必须返回结构化 unavailable 或空 inventory，不得 panic、阻塞等待或静默构造 stub provider 冒充真实服务。
- Context Service 可以通过 Memory Service client 主动召回长期记忆，但不得直接绑定具体 memory backend；LLM 调用属于 LLM Service，不属于 Context Service。
- 任何新增 LLM/Memory/Context 调用路径必须记录 trace id、application/session/agent scope、command、completion/failure 和 snapshot/event，不得泄露 prompt body、memory body、embedding、API key 或 secret。

## 12. Driver / Skill / MCP Service Ownership

路线 C S6 将 Driver、Skill、MCP 收敛为可替换 system service，而不是 Web/CLI presentation shell 内部能力：

- `macaca-driver` 只拥有 provider-neutral Driver command/result/snapshot DTO、driver descriptor 和领域 adapter，不依赖 kernel、runtime-host、Web 或 CLI。
- `macaca-skill` 只拥有 provider-neutral Skill snapshot、executable load、tool catalog、tool invoke、status、service snapshot DTO 和领域 adapter，不依赖 kernel、runtime-host、Web 或 CLI。
- MCP service DTO 归属 `macaca-proto`，因为 SDK 需要消费 MCP service command/result 但不得依赖 `macaca-runtime-host`，避免 SDK/runtime-host/framework 形成依赖环。
- `macaca-runtime-host` 拥有 Driver/Skill/MCP 的 `SystemService` provider wrapper、service lifecycle、trace-required dispatch、policy/decorator chain 和 snapshot emission。
- `macaca-sdk` 只提供 `SystemDriverClient`、`SystemSkillClient`、`SystemMcpClient`、`SystemFacade` 这类 shell-facing client/facade，不构造 DriverRuntime、SkillRuntimeFacade、ExecutableSkillToolSet、McpRuntimeFacade 或 Toolkit。
- Web、CLI、framework 只能作为 adapter：Web 负责 HTTP/SSE/UI 映射和 host-local Toolkit 组装，CLI 负责 terminal 输出，framework 负责 agent/toolkit seam，不得定义 driver、skill、MCP 的核心语义。
- Driver/Skill tool catalog 和 invocation 应通过 service client + service-backed tool adapter；旧 direct runtime/toolset 入口必须保留为 deprecated 搜索锚点，不得作为新生产代码默认路径。
- MCP protocol lifecycle 属于 MCP Service，但当前 framework `Toolkit` 是 host-local 可变对象，MCP tool attach 仍可暂时通过 deprecated `McpRuntimeFacade::register_definitions` 作为兼容债务保留。该债务的过期条件是 MCP attach 能通过 service-owned host handle、tool proxy registry 或等价可审计资源句柄表达，而不是把 raw Toolkit 跨 service 边界移动。
- 缺少 Driver/Skill/MCP runtime-backed service 时必须返回结构化 unavailable 或空 inventory，不得 panic、阻塞等待或静默构造 provider。
- 任何新增 Driver/Skill/MCP 调用路径必须记录 trace id、application/session/agent scope、command、completion/failure 和 snapshot/event，不得泄露 env、headers、API key、provider credentials、raw command secrets 或未脱敏 tool payload。

## 13. Application Framework Service Ownership

路线 C S7 将 Application Framework 生命周期收敛为可替换 system service，而不是 Web/CLI presentation shell 内部编排：

- `macaca-app` 仍拥有 application manifest、registry、runtime assembly、ApplicationHost、ABI metadata、lifecycle projection、GenUI validation 和 admission Specification。Application Service DTO 只描述 provider-neutral command/result，不把 application 语义迁入 Web、SDK 或 runtime-host。
- `macaca-runtime-host` 拥有 `ApplicationSystemServiceProvider` wrapper、service lifecycle、trace-required dispatch、policy/decorator chain、snapshot emission 和 structured unavailable，不得拥有业务 workflow、application name special case、prompt assembly、task planning、LLM execution、Driver/Skill/MCP execution。
- `macaca-sdk` 只提供 `SystemApplicationClient` 和 `SystemFacade::application_client()` 这类 shell-facing Facade/Strategy client，不构造 `AppRuntime`、`AppRegistry`、`Kernel`、Web state、provider 或 application workflow。
- Web、CLI、Gateway 只能作为 adapter：Web 负责 HTTP/SSE/UI 映射、现有 chat coordinator 兼容执行和 host-local toolkit 组装；新 application discover/start/status/session/GenUI 查询路径必须优先通过 Application Service。
- 旧 `AppRuntime` startup APIs、Web `runtime`/`registry` fields 和直接 manifest reads 必须保留为 deprecated 搜索锚点，直到所有消费者迁移且 dependency gate 证明 Cargo direct edge 可以删除。保留旧代码不等于允许新增默认 direct path。
- Application Service logs 和 snapshots 只能包含 ids、names、counts、runtime kind、lifecycle status、trace id、safe directory metadata 和 diagnostics；不得泄露 prompt body、raw manifest body、raw agent config、env、API key、secret 或 raw host payload。
- WASM/package application 在 S7 只能 metadata-only admission；执行缺失必须返回 structured unavailable，不得 panic、阻塞等待或假装启动成功。
- `/api/chat/v2` 在 S7 只迁移 entry-agent/status/session envelope preflight。Coordinator execution、PlanLoop、WorkerLoop、EventLog persistence、RunTracer、resume signal 和 SSE shape 仍由既有路径保持兼容，不能被 Application Service 吸收。

## 14. Store / Entitlement Service Ownership

路线 C S9 将 Store / Entitlement 收敛为可替换 system service，而不是 application、skill、Web、CLI 或 runtime helper 各自拥有商业授权逻辑：

- Store Service 只拥有 package inspect、resolve、install、status、snapshot 这类 provider-neutral package lifecycle command，不拥有支付结算、市场推荐、业务运营、driver/skill/MCP/application 执行或 Web3/EVM 证明逻辑。
- Entitlement Service 只拥有 entitlement query、upsert、revoke、install/start/call authorization、audit query、metering record 和 snapshot，不拥有具体 Store vendor、payment provider、application workflow 或 encrypted payload 解密实现。
- `macaca-proto` 拥有 Store/Entitlement command/result DTO、service ids、command names 和 sanitized views，因为 SDK、runtime-host、Web、CLI、app、skill 都必须共享同一 provider-neutral contract。
- `macaca-runtime-host` 拥有 Store/Entitlement `SystemService` provider wrapper、service lifecycle、trace-required dispatch、policy/decorator admission、sanitized logging 和 snapshot emission。它只能适配 Phase 08 facade/store/guard seam，不得把 commerce 规则写成 provider/app/workflow special case。
- `macaca-sdk` 拥有 `SystemStoreClient`、`SystemEntitlementClient`、service-backed clients、unavailable/null clients 和 `SystemFacade` accessor。SDK 不得构造 runtime-host providers、entitlement repositories、application runtime、skill runtime、Web state 或 CLI state。
- `macaca-app` 和 `macaca-skill` 只能通过 authorizer trait seam 调用 entitlement decisions。Direct Phase 08 facade/helper path 必须保留为 deprecated compatibility anchor；新生产路径应优先使用 service-backed authorizer/client。
- Web 和 CLI 只能作为 shell adapter：启动时可以注册 built-in Store/Entitlement providers，但 package/entitlement route 或 command 必须通过 `SystemFacade` / focused clients。Web/CLI 不得定义 license precedence、paid policy、metering policy 或 encrypted package authorization semantics。
- 免费/开源 package 在 Store Service 缺失时必须保持本地兼容路径；付费、订阅、metered、encrypted package 在 entitlement missing/expired/revoked/region_blocked/usage_exceeded/service_unavailable 时必须返回结构化 deny/unavailable，不得静默 allow。
- Store/Entitlement logs、audit pages 和 snapshots 只能包含 service id、command、trace id、package id、developer id、operation、state、reason code、counts、timestamps 和 sanitized diagnostics；不得泄露 raw package bytes、raw manifest body、encrypted payload、license secrets、credentials、API keys、private keys、prompt bodies 或 raw tool payload。
- S9 不实现 S10 payment settlement，也不实现 S11 Web3/EVM entitlement verification。未来 payment/Web3/EVM 只能作为 strategy/provider/module 接入 Store/Entitlement boundary，不能回写到 kernel 或 presentation shell。

## 15. 审查清单

任何 Route C OpenSpec 都必须回答：

- 这个能力属于 kernel、service、plugin、optional module、application framework 还是 presentation？
- 是否存在 provider/app/driver/gateway hardcode？
- 是否支持 trace？
- 是否支持 permission/policy？
- 缺失 optional module 时如何表现？
- 如何验证不破坏 Route C regression matrix？
- 是否触发依赖边界门禁？如果触发，是否有 OpenSpec 和 allowlist 迁移计划？
- 如果涉及 service runtime，是否证明 trace/policy/decorator/snapshot/event 设计可审计且可替换？
