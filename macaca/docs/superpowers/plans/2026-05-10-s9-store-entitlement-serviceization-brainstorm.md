# S9 Store / Entitlement 服务化 Brainstorm

## 背景

S9 来自 `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`，目标是把 Store / Entitlement 从 Phase 08 的运行时守卫和合同基线升级为真正的 system service。S9 需要统一管理 package source、签名、license、subscription、metering、encrypted package，以及 package install/start/call 的授权路径。

必须遵守：

- `macaca/docs/agent-os-microkernel-boundaries.md`
- `macaca/docs/route-c-serviceization-allowlist.md`
- `macaca/docs/route-c-architecture-governance.md`
- `macaca/docs/route-c-regression-matrix.md`

当前诊断：

- `macaca-proto/src/commerce.rs` 已有 provider-neutral commerce / entitlement / metering DTO。
- `macaca-persist/src/entitlement_store.rs` 已有 `EntitlementStore` repository contract 和 in-memory adapter。
- `macaca-runtime-host/src/entitlement.rs` 已有 `EntitlementRuntimeFacade`，能做 install/start/capability call 决策和 metering event emission。
- `macaca-app/src/commercial_package.rs` 已通过 `ApplicationEntitlementAuthorizer` trait 避免 app crate 反向依赖 runtime-host。
- `macaca-skill/src/encrypted_package.rs` 已有 encrypted package authorizer/decryptor seam。
- `macaca-sdk/src/package_client.rs` 仍只是 S3 的 empty package inspection client，没有 Store/Entitlement focused client。
- 还没有 provider-neutral Store Service / Entitlement Service command contract、runtime-host service provider、SDK focused clients、Web/CLI package manager 迁移路径。
- Phase 08 `add-store-entitlement-v0` 已完成的是 commerce contract、persistence contract、runtime guard facade、encrypted skill hook 和 app commercial guard；S9 不应重复这些基线，而应把它们挂到 `ServiceRuntime` 和 `SystemFacade` 下成为真实系统服务。

S9 不包含：

- 真实 payment settlement、quote、intent、receipt 和 A2A 支付编排，属于 S10。
- Web3/EVM 链上授权或钱包签名，属于 S11。
- 完整 Store marketplace UI、推荐系统、搜索排名和商业运营逻辑。
- 删除 Phase 08 guard/facade 旧 API；旧 API 应保留并标记 deprecated，作为迁移搜索锚点。
- 新增具体 Store vendor、payment provider、chain name、app name、workflow name、driver name 或业务名称硬编码。

## 设计模式候选

### Facade

建立 focused service boundary：

- `StoreService`：package source、package metadata、install request、package status、store catalog snapshot。
- `EntitlementService`：entitlement query/upsert/revoke、install/start/call authorization、audit query、metering record。
- `SystemStoreClient` / `SystemEntitlementClient`：SDK shell-facing facade。

优点：

- Web/CLI/Gateway/Application 不再直接拿 `EntitlementRuntimeFacade` 或 repository。
- Store 和 Entitlement 可以被本地 provider、企业 Store、远程 marketplace、离线 license store 替换。
- 和 S1/S3/S5/S6/S7 的 focused service/client 模式一致。

风险：

- Store Service 和 Entitlement Service 容易合并成一个巨型 commerce macro-service。
- Mitigation：Store owns distribution metadata and package source; Entitlement owns authorization state and metering. 首版可以共用一个 runtime-host provider factory，但 contract 分开。

### Chain of Responsibility

package install/start/call 进入授权链：

`source trust -> signature metadata -> compatibility -> entitlement state -> subscription/license policy -> metering policy -> audit`

优点：

- 与现有 `PackageRuntimeGuard` 和 Phase 08 `EntitlementRuntimeFacade` 思路一致。
- 每个步骤可测试、可替换、可审计。
- 后续企业 policy、region compliance、offline grace、usage budget 可以作为链条节点加入。

风险：

- 如果每个 consumer 自己组链，会出现绕过路径。
- Mitigation：chain 只在 Entitlement Service provider 内部组装；app/skill/web/cli 只能通过 service client 或已标记 deprecated 的兼容 facade 调用。

### Strategy

可替换策略：

- package source resolution strategy
- signature verification strategy
- entitlement resolution strategy
- license policy strategy
- subscription state strategy
- metering aggregation strategy
- offline grace strategy
- encrypted package decrypt authorization strategy

优点：

- 不绑定具体 Store vendor、支付网络、数据库或 license 规则。
- 小白用户可以替换默认本地/in-memory 实现为企业 Store 或第三方 Store adapter。
- 性能开销可控：策略在 service provider 内部组合，不影响 DTO 稳定性。

风险：

- 过早开放太多 trait 会过度设计。
- Mitigation：首版只稳定 service command/result DTO、provider trait 和两到三个核心策略 seam；复杂策略留扩展字段和 provider-private implementation。

### Adapter / Bridge

把现有 Phase 08 组件桥接进 service：

- `EntitlementStore` repository -> Entitlement Service persistence adapter。
- `EntitlementRuntimeFacade` -> Entitlement Service provider implementation。
- `CommercialPackageGuard` -> service-backed authorizer。
- `EncryptedPackageLoader` -> service-backed authorizer。
- `PackageRuntimeGuard` -> Store/Entitlement admission step。

优点：

- 不重写已验证逻辑。
- 保持 additive-first，可回滚。
- 旧 API 可以继续作为 deprecated compatibility anchor。

风险：

- 如果 runtime-host provider 直接暴露 facade concrete type，会把新 service contract 绑死。
- Mitigation：service contract 只用 `macaca-proto` DTO；runtime-host adapter 内部使用 `EntitlementRuntimeFacade`，外部不可见。

### Command

所有 Store/Entitlement 操作使用 typed command，再转为 `ServiceCommand` payload：

- `store.package.inspect`
- `store.package.resolve`
- `store.package.install`
- `store.package.status`
- `store.snapshot`
- `entitlement.query`
- `entitlement.upsert`
- `entitlement.revoke`
- `entitlement.authorize.install`
- `entitlement.authorize.start`
- `entitlement.authorize.call`
- `entitlement.audit.query`
- `entitlement.metering.record`
- `entitlement.snapshot`

优点：

- 每个入口都携带 trace、application/session/package/developer/capability scope。
- Web/CLI/Gateway/Application/Skill 都能复用同一 service path。
- 未来 remote Store Service 不需要暴露 Rust concrete type。

风险：

- command payload 如果直接塞 raw manifest、encrypted bytes 或 secrets，会泄漏敏感信息。
- Mitigation：默认 command/result 只传 sanitized manifest metadata、ids、version、license、runtime kind、capability id、quantity、unit、status；raw package bytes 和 encrypted payload 只通过 resource handle 或 provider-private channel。

### Specification

集中验证：

- trace required
- package id / developer id / license scope
- operation kind
- entitlement state precedence
- paid family license policy
- metering required fields
- audit redaction rules
- encrypted package entitlement readiness

优点：

- 避免 Web、App、Skill、runtime-host 各写 if/else。
- 可测试，可审计。
- 与 Route C dependency gate 和 PackageRuntimeGuard 思路一致。

风险：

- 规格过多导致首版变慢。
- Mitigation：首版实现 Store/Entitlement service admission specs，不做完整 enterprise compliance engine。

### Observer

关键节点必须记录 structured logs / trace / audit：

- store provider register/start/stop
- package inspect/resolve/install/status
- entitlement query/upsert/revoke
- install/start/call authorization allow/deny
- metering record emitted
- encrypted package authorization
- audit query
- service unavailable / policy denied

优点：

- 满足 Route C “无 trace 不执行”。
- 支撑 RC-APP-001、RC-SKILL-001、RC-TRACE-001。
- 后续 payment / receipt / dispute 可以复盘。

风险：

- 审计日志泄漏 license secret、store credentials、encrypted payload、user private data。
- Mitigation：日志只包含 bounded identifiers、状态、counts、trace id、operation、reason code；禁止记录 token、private key、raw package body、raw encrypted bytes、prompt body、API key。

### Null Object

缺少 Store Service 或 Entitlement Service 时：

- free/open package install/start 仍可走结构化 allow。
- paid/subscription/metered package 返回 structured unavailable 或 entitlement missing。
- package catalog 返回 empty snapshot + diagnostics。
- encrypted package decrypt 不执行，返回 entitlement service unavailable。

优点：

- base OS 不依赖具体 Store。
- 本地开发和开源 package 不被商业系统阻断。
- 缺失付费能力不会 panic/hang。

风险：

- Null Object 被误认为真实授权成功。
- Mitigation：free/open fast-path 和 paid unavailable 必须区分；结果必须带 service id、trace id、reason、operation、license type。

### Memento

Store/Entitlement snapshot：

- package source ids/counts
- installed package metadata
- entitlement state counts
- decision audit summary
- metering event counts
- provider health and last sync time
- sanitized diagnostics

优点：

- Web/CLI 可展示系统状态而不读 repository。
- restart/recovery 后可审计 package 和 entitlement 状态。
- 后续 remote Store sync 和 cache recovery 有扩展点。

风险：

- snapshot 过大或泄漏敏感数据。
- Mitigation：默认 snapshot 是 metadata/counts；raw audit detail 通过 paginated audit query 且必须有 trace/policy。

## 可选方案

### 方案 A：继续使用 `EntitlementRuntimeFacade`，只补 SDK helper

做法：

- 在 SDK 加几个 helper，Web/CLI 直接调 runtime-host facade 或 empty client。
- 不新增 Store/Entitlement system service provider。

优点：

- 变更小，能快速让上层调用更集中。

缺点：

- 不满足 S9 “服务化”目标。
- runtime-host facade 仍是 helper，不是可替换 service。
- Web/CLI 仍可能绕过 ServiceRuntime trace/policy/decorator。

结论：拒绝。可作为兼容 fallback，但不能作为目标架构。

### 方案 B：建立 Entitlement Service，Store Service 暂缓

做法：

- 只把 entitlement query/authorize/metering/audit 做成 service。
- package source / install / status 仍由 application/package runtime 直接处理。

优点：

- 聚焦授权闭环，风险较低。
- 能复用 Phase 08 facade 和 persistence。

缺点：

- package install/source/status 仍没有 Store Service 边界。
- 后续 Web/CLI package manager 仍会读 app/skill/package internals。
- 不能完整覆盖 S9 的 Store / Entitlement 服务化目标。

结论：不推荐作为完整 S9。可以作为第一实现切片，但计划必须包含 Store Service。

### 方案 C：建立 Store Service + Entitlement Service，runtime-host provider 适配 Phase 08 组件，并迁移 SDK/Web/CLI

做法：

- 在 `macaca-proto` 增加 Store/Entitlement service DTO。
- 在 `macaca-runtime-host` 新增 `StoreSystemServiceProvider` 和 `EntitlementSystemServiceProvider`，内部复用 `EntitlementRuntimeFacade`、`EntitlementStore`、PackageRuntimeGuard/CommercialPackageGuard seams。
- 在 `macaca-sdk` 新增 `SystemStoreClient`、`SystemEntitlementClient`，并让 `SystemPackageClient` 走 Store Service。
- Web/CLI package manager、app/skill entitlement guard 优先通过 service client；旧 facade 保留 deprecated。

优点：

- 符合 Route C 服务化边界。
- 复用已有 Phase 08 逻辑，避免重写。
- 上层统一通过 SystemFacade，方便 S12 thin shell 收敛。
- 后续 S10 Payment、S11 Web3/EVM、S13 ecosystem certification 都能接入。

缺点：

- 修改面跨 proto、runtime-host、sdk、app、skill、web/cli、integration tests。
- 需要仔细控制 DTO 大小和敏感字段。

结论：推荐。分片实施，先 Entitlement Service，后 Store Service，再消费者迁移。

### 方案 D：新增独立 `macaca-store` crate

做法：

- 创建 `macaca-store` 管 Store/Entitlement service contract、provider、policy。

优点：

- 领域边界直观。
- 长期可独立演进。

缺点：

- 当前 workspace 已有 `macaca-proto` commerce DTO、`macaca-persist` entitlement store、`macaca-runtime-host` service runtime；新增 crate 会增加依赖治理成本。
- 用户要求避免不必要新依赖和巨型扩张；S9 可先在既有 crate 内完成服务边界。

结论：暂不采用。计划保留未来拆分可能，但本次不新增 crate。

## 推荐方案

采用方案 C，但 additive-first：

1. 在 `macaca-proto` 定义 provider-neutral Store/Entitlement Service DTO 和 command/result names。
2. 在 `macaca-runtime-host` 实现 runtime-host owned service providers，内部复用 Phase 08 `EntitlementRuntimeFacade` 和 `EntitlementStore`。
3. 在 `macaca-sdk` 增加 focused clients，并让现有 `SystemPackageClient` 从 empty inspection 升级为 Store Service backed inspection/install/status。
4. 在 `macaca-app`、`macaca-skill`、Web/CLI package manager 相关路径优先使用 service-backed authorizer/client。
5. 旧 direct APIs 保留并标记 deprecated，禁止新生产路径调用。

## 主要风险与缓解

| 风险 | 影响 | 缓解 |
| --- | --- | --- |
| 免费/open package 被错误拒绝 | 阻断本地开发和现有 YAML app | free/open fast-path tests；paid/unavailable 与 free/open allow 明确区分 |
| paid capability 存在绕过路径 | entitlement bypass | service-backed authorizer 替代直接 facade；deprecated old APIs；hardcode/call-site scan |
| DTO 泄漏敏感信息 | Store credentials、encrypted bytes、license secrets 泄漏 | sanitized DTO；logs 禁止 raw package/encrypted payload/secrets |
| Store Service 变成 marketplace 巨型服务 | 架构膨胀 | Store only owns package source/status/install command；Payment、rating、recommendation、billing settlement 不进 S9 |
| runtime-host provider 反向拥有 application 语义 | 宏服务化 | runtime-host 只适配 service lifecycle；Application Framework 和 Skill 语义仍归各自 crate |
| 与 S10 Payment 边界混淆 | 支付逻辑提前进入 S9 | S9 只产出 metering/audit和 entitlement decision；quote/intent/receipt 留给 S10 |
| 文件超过 500 行 | 违反 AGENTS.md | 按 `store_service.rs`、`entitlement_service.rs`、`store_client.rs`、`entitlement_client.rs`、`service_admission.rs` 拆分 |

## 成功判定

- Store/Entitlement 作为 `ServiceRuntime` provider 注册、启动、调用、停止、snapshot。
- SDK/Web/CLI package inspection/install/status 和 entitlement authorization 优先通过 `SystemFacade` / service client。
- App commercial guard 和 encrypted skill guard 有 service-backed authorizer path。
- paid install/start/call 必须 trace + policy + audit；free/open package 不依赖 Store。
- `route_c_dependency_boundaries` 不新增 forbidden dependency；若短期保留直接边，必须更新 allowlist 并写明过期条件。
- `openspec validate`、targeted cargo tests、Route C baseline 和 GitNexus detect changes 通过。
