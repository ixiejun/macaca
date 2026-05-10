# S11 Web3 / EVM Optional Module 真实化 Brainstorm

## 背景

S11 来自 `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`，目标是把 Web3 / EVM 从 Phase 10/11 的 contract skeleton 推进到 Route C 下的真实 optional module service path：可安装、可缺失、可禁用、可 trace，并且不能让 Web3/EVM 变成 base OS、kernel、Web shell 或应用框架的强依赖。

本轮必须严格遵守：

- `macaca/docs/agent-os-microkernel-boundaries.md`
- `macaca/docs/route-c-serviceization-allowlist.md`
- `macaca/docs/route-c-architecture-governance.md`
- `macaca/docs/design_patterns.md`

当前诊断：

- `add-optional-web3-node-v0` 已经建立 Web3 proto、kernel optional facade/null/mock adapter、SDK/Web status surface 和 absent-safe 行为。
- `add-optional-evm-dapp-v0` 已经建立 EVM proto、kernel optional facade/null/mock adapter、SDK/Web status surface 和 absent-safe 行为。
- `macaca-proto/src/web3.rs` 与 `macaca-proto/src/evm.rs` 已有 provider-neutral value objects，但还缺少面向 `ServiceRuntime` 的 service command DTO、snapshot DTO 和 runtime admission DTO。
- `macaca-kernel/src/web3.rs` 与 `macaca-kernel/src/evm.rs` 仍然承载 optional facade、null/mock adapter 和测试路径。它们适合作为 compatibility anchor，但不应继续成为 Web3/EVM 的生产执行 owner。
- `macaca-sdk/src/evm.rs` 目前提供 SDK facade，但后续上层 consumer 应通过 focused `SystemWeb3Client` / `SystemEvmClient` 调用 runtime service，而不是直接依赖 kernel optional facade。
- `macaca-web/src/web3_status.rs` 目前只适合做 shell/status adapter。Web shell 不能定义链、钱包、签名、gas、contract call、provider selection 或 EVM execution 语义。
- `macaca-runtime-host` 已经有 ServiceRuntime 和多类 system service provider。S11 应把 Web3/EVM 注册为 runtime-host owned optional services，并通过 unavailable/default provider、mock/dev provider、future external adapter provider 扩展。

S11 不应重复 v0 skeleton。它要解决的是：

- Web3/EVM 的真实 service ownership 在 runtime-host，而不是 kernel。
- base OS 在没有 Web3/EVM provider 时仍完整可用。
- Web3/EVM command 必须通过 trace、policy、capability、entitlement/resource hook。
- Mock/dev provider 不能被误认为真实链、真实钱包或真实 EVM。
- 所有接口都必须 provider-neutral，不硬编码 chain、wallet、RPC、application、gateway、business name。

S11 不包含：

- 真实链节点、RPC provider、钱包私钥、签名密钥、助记词、keystore、gas 付款或链上交易广播。
- 自研 EVM、Substrate/Frontier 集成、DApp marketplace、链上 payment settlement。
- 删除现有 kernel Web3/EVM facade。旧接口只标记 deprecated，保留迁移检索锚点。
- 新增 `macaca-web3` / `macaca-evm` crate。当前阶段优先在现有 crate 内以文件/模块结构服务化，避免不必要的 crate 扩张。

## 设计模式候选

### Facade

建立 focused service facade：

- `Web3 Service`：availability、wallet list、signing request admission、transaction prepare/admit、chain query、snapshot。
- `EVM Service`：availability、contract deploy/call/read admission、gas estimate、receipt query、event subscription admission、snapshot。
- `SystemWeb3Client` / `SystemEvmClient`：SDK-facing facade。

优点：

- Web/CLI/Gateway/Application 不接触 runtime-host provider concrete type。
- Future RPC、wallet、node、EVM adapter 可以替换，不影响 consumer。
- 与 S5/S6/S7/S9/S10 的 focused client 方向一致。

风险：

- Facade 容易膨胀成 Web3 macro-service。
- Mitigation：Web3 只负责 wallet/signing/transaction/chain query admission；EVM 只负责 contract/EVM semantic admission；Payment、Store、Entitlement、Application lifecycle 不并入 Web3/EVM。

### Adapter / Bridge

把不同 Web3/EVM 后端桥接成 Macaca provider-neutral contract：

- unavailable provider
- mock/dev provider
- future RPC provider
- future wallet provider
- future Substrate/Frontier/EVM adapter

优点：

- Macaca 不绑定具体链、钱包、节点、RPC vendor 或 EVM 实现。
- Provider 可以本地、远程、插件化或进程外运行。
- Mock provider 可用于 contract test，不污染 production path。

风险：

- 如果 adapter DTO 暴露 provider raw payload，会把供应商协议泄漏到系统 contract。
- Mitigation：service DTO 只保留 provider-neutral fields；provider raw response 只能作为 redacted artifact handle 或 debug-only metadata。

### Strategy

需要可替换策略：

- network policy strategy
- signing policy strategy
- fee/gas policy strategy
- transaction admission strategy
- contract call admission strategy
- availability policy strategy
- provider selection strategy

优点：

- 不硬编码 chain、region、wallet、gas policy 或 application special case。
- 以后可以按 tenant/session/application/agent/capability 配置策略。
- 性能开销可控：首版只实现 unavailable strategy 与 mock/dev strategy。

风险：

- 过早抽象过多 trait 会增加维护成本。
- Mitigation：首版只稳定 command DTO、provider trait 和 policy hook；高级策略通过 metadata/capability extension 预留，不实现真实 chain logic。

### Command

所有 Web3/EVM 操作必须先表达为 typed command，再进入 `ServiceRuntime`：

- `web3.availability.get`
- `web3.wallet.list`
- `web3.signing.request`
- `web3.transaction.prepare`
- `web3.chain.query`
- `web3.snapshot.get`
- `evm.availability.get`
- `evm.contract.deploy`
- `evm.contract.call`
- `evm.contract.read`
- `evm.gas.estimate`
- `evm.receipt.get`
- `evm.event.subscribe`
- `evm.snapshot.get`

优点：

- 每个入口可 trace、可审计、可 replay、可远程化。
- SDK/Web/CLI/Application/Gateway 复用同一 contract。
- 后续安装式 optional module 不需要暴露 Rust concrete type。

风险：

- Command payload 可能携带私钥、raw signed transaction、raw ABI 或 secret。
- Mitigation：DTO 明确禁止 private key、mnemonic、raw signature secret、provider credential；ABI/bytecode 仅允许 bounded reference 或 redacted digest。

### Null Object

未安装、被禁用、provider missing、policy denied 时提供结构化 unavailable behavior：

- availability 返回 unavailable/disabled/policy-denied reason。
- mutating command fail closed。
- read/snapshot/list 可返回 empty view + diagnostics。
- 普通非 Web3/EVM application 不受影响。

优点：

- Web3/EVM 保持 optional，不成为 base OS 启动前提。
- User/Application 能区分 absent、disabled、unsupported、denied、provider-error。

风险：

- unavailable 被 consumer 静默吞掉导致功能假成功。
- Mitigation：mutating command 必须返回 structured error；trace/audit 必须记录 unavailable reason。

### Proxy

真实 provider 未来可能是本地节点、远程 RPC、插件进程或外部托管服务：

- Service provider 对 consumer 表现为同一个 proxy。
- RPC/插件/remote transport 只存在 provider 内部或 IPC adapter 内部。

优点：

- 支持本地/远程/插件部署形态。
- 不让 Web/SDK/Application 关心节点位置。

风险：

- Proxy 容易隐藏安全边界，导致远程 provider 被当作 trusted local。
- Mitigation：provider descriptor 必须暴露 trust level、capability、policy status、redaction guarantees 和 audit mode。

### Observer

关键节点必须产生 structured log / trace / audit：

- service registered/started/stopped
- provider selected/unavailable/disabled
- availability queried
- signing request admitted/denied
- transaction prepared/denied
- chain query admitted/denied
- contract deploy/call/read admitted/denied
- gas estimate requested
- receipt queried
- snapshot queried

优点：

- 满足 Route C “无 trace 不执行”。
- Web UI、session replay、审计和后续 incident review 有事实来源。

风险：

- trace payload 包含敏感链上操作材料或过大 ABI/bytecode。
- Mitigation：trace 只写 bounded identifiers、operation、capability、status、reason、artifact digest，不写 secret/raw payload。

### Specification

用集中规则验证：

- capability requirement
- trace context requirement
- service availability
- policy enabled/disabled
- provider descriptor compliance
- redaction rule
- command size/bounds
- no secret payload

优点：

- 边界规则可测试、可审计、可扩展。
- 防止同样 admission logic 散落在 Web、SDK、runtime-host 和 provider 内。

风险：

- Specification 如果和 provider logic 分离太远，容易产生重复或漂移。
- Mitigation：runtime-host provider 在 command decode 后统一调用 admission specification，并在测试里覆盖 denied/unavailable paths。

### State

Web3/EVM service lifecycle 与 command lifecycle 需要小状态机：

- service lifecycle：`registered -> starting -> available|unavailable|disabled -> stopping -> stopped`
- command lifecycle：`received -> admitted|denied -> dispatched -> completed|failed`
- mock/dev provider lifecycle：`created -> enabled_for_test|disabled -> stopped`

优点：

- 可解释 provider 处于 absent、disabled、policy-denied、mock-only、available 的不同状态。
- 适合 snapshot 和 UI status。

风险：

- 多 provider、多链、多 wallet 状态容易复杂。
- Mitigation：S11 只定义 canonical small state；provider-specific state 放入 redacted diagnostics。

### Memento

需要保存最小可回放 artifact：

- availability snapshot
- provider descriptor snapshot
- admitted/denied command summary
- transaction/contract operation digest
- receipt/query result digest

优点：

- 支持 session replay 和审计。
- 不需要保存 raw chain provider response。

风险：

- 保存过多会形成隐私和安全风险。
- Mitigation：只保存 redacted/bounded summary；raw secret、private key、mnemonic、signed payload、provider credential 禁止进入 memento。

## 可选方案

### 方案 A：保留 kernel facade，只加 SDK wrapper

做法：

- `macaca-kernel` 继续拥有 Web3/EVM optional facade、null/mock adapter。
- `macaca-sdk` 只包一层 `SystemWeb3Client` / `SystemEvmClient`。

优点：

- 变更最小。
- v0 代码复用最高。

问题：

- 没有完成 Route C serviceization。
- kernel 继续拥有 optional module adapter/execution semantics。
- Web3/EVM 后续会继续向 kernel 膨胀。

结论：

- 不推荐。只能作为临时 compatibility path。

### 方案 B：只服务化 Web3，EVM 继续依赖 Web3 facade

做法：

- 先把 Web3 wallet/signing/transaction/chain query 做成 ServiceRuntime provider。
- EVM 仍通过现有 kernel EVM facade 或 SDK helper 调用 Web3。

优点：

- 切片更小。
- Web3 是 EVM 的前置依赖，先做 Web3 有合理性。

问题：

- S11 明确要求 Web3 / EVM optional module 真实化，EVM 继续 skeleton 会留下半迁移状态。
- Consumer 容易形成两套调用路径。

结论：

- 可作为内部实施顺序，但不能作为最终方案。

### 方案 C：Web3 与 EVM 一起服务化，runtime-host 拥有 provider，SDK/Web 只消费 focused clients

做法：

- 在 `macaca-proto` 增加 Web3/EVM service command DTO 与 snapshot DTO。
- 在 `macaca-runtime-host` 增加 Web3/EVM optional service providers，内置 unavailable provider 和 mock/dev provider。
- 在 `macaca-sdk` 增加 `SystemWeb3Client` / `SystemEvmClient`。
- Web composition root 只注册服务并持有 SDK client，不定义业务语义。
- 旧 kernel facade 标记 deprecated，保留 behavior 和测试锚点。

优点：

- 符合 Route C：optional module 是可缺失、可禁用、可替换的 service。
- Web3/EVM 不进入 kernel 和 base OS 必需路径。
- 后续真实 provider、插件化 provider、远程 provider 有稳定 contract。

风险：

- 涉及 proto/runtime-host/sdk/web/kernel 多 crate，修改面较广。
- 如果把 Web3/EVM 合并成一个 provider，容易形成巨型 service。

Mitigation：

- 以 additive-first 小切片实施。
- Web3 Service 与 EVM Service 分开 provider/service id，但共享 capability/policy/admission primitives。
- OpenSpec 先明确边界和 non-goals。

结论：

- 推荐。

### 方案 D：立即新增 `macaca-web3` 与 `macaca-evm` crates

做法：

- 把 optional module provider 独立成新 crates。
- runtime-host 只负责加载 crates 或插件。

优点：

- 模块边界更物理化。
- 未来第三方 provider 接入路径更清晰。

问题：

- 用户已明确代码组织不应随意增加 crate，优先在现有 crate 内用文件结构组织。
- 当前还没有真实 chain/RPC/provider，新增 crate 容易制造空壳。
- 会增加 Cargo dependency gate 和 workspace 维护成本。

结论：

- 暂不推荐。S11 先在现有 crate 内做真实 service path，后续 provider 复杂度足够时再拆。

## 推荐方案

选择方案 C。

推荐架构：

```text
Application / Agent / Web / CLI
        |
        v
macaca-sdk::SystemWeb3Client / SystemEvmClient
        |
        v
macaca-runtime-host::ServiceRuntime
        |
        +-- Web3OptionalServiceProvider
        |       +-- UnavailableWeb3Provider
        |       +-- MockWeb3Provider
        |       +-- future external adapter
        |
        +-- EvmOptionalServiceProvider
                +-- UnavailableEvmProvider
                +-- MockEvmProvider
                +-- future external adapter

macaca-kernel::web3 / evm
        |
        +-- deprecated compatibility anchors only
```

边界原则：

- Kernel 只保留 service registry、policy primitive、trace/audit primitive 和 deprecated compatibility facade。
- Runtime-host 拥有 Web3/EVM optional service provider lifecycle、admission、strategy dispatch 和 trace emission。
- SDK 拥有 focused clients。
- Web/CLI 是 thin shell，只做展示、用户确认入口和 status surface。
- Proto 拥有 provider-neutral DTO，不拥有 provider implementation。

## 风险与缓解

- 风险：Web3/EVM 变成 base OS 必需依赖。
  - 缓解：默认注册 unavailable/null provider；普通 application 不声明 capability 时不得触发 Web3/EVM path。

- 风险：Mock/dev provider 被当成真实链能力。
  - 缓解：provider descriptor 必须带 `mock_only` / `development_only` / `trust_level` / `real_chain=false` diagnostics；mutating command trace 必须写明 provider class。

- 风险：私钥、助记词、签名材料、raw transaction、provider credential 泄露到 DTO、trace、memento。
  - 缓解：OpenSpec 明确 forbidden fields；runtime admission specification 拒绝 suspicious payload；trace 只保存 digest/reference。

- 风险：Web3/EVM 绕过 Payment/Entitlement/Policy。
  - 缓解：mutating command 必须经过 trace、capability、policy、entitlement/resource hook；未来链上 payment settlement 必须通过 Payment Service adapter，不直接在 Web3/EVM 内实现支付业务。

- 风险：服务边界过大，Web3/EVM 互相耦合。
  - 缓解：两个 service id、两个 provider、两个 SDK client；共享的只是 proto primitive 和 admission helper。

- 风险：provider/chain hardcode 进入控制流。
  - 缓解：provider selection 基于 descriptor/capability/policy，不基于 chain name、vendor name、app name 或业务名称。

- 风险：OpenSpec 与实现漂移。
  - 缓解：先写 `add-web3-evm-optional-services-v1`，再实施；每个切片执行 `openspec validate --strict` 和 Route C dependency gate。

## 结论

S11 应以 `add-web3-evm-optional-services-v1` 为 OpenSpec change，把 Web3/EVM 从 v0 optional skeleton 升级为 ServiceRuntime-owned optional services。实现必须保持 additive-first：先建立 service DTO、runtime-host unavailable/mock providers、SDK clients 和 Web composition root，再把旧 kernel facade 标记 deprecated。真实 chain/RPC/wallet/private key/EVM execution 不进入本阶段。
