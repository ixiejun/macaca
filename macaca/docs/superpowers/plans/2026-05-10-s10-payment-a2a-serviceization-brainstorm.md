# S10 Payment / A2A 服务化 Brainstorm

## 背景

S10 来自 `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`，目标是把 A2A quote、payment intent、budget、approval、settlement、receipt、execution proof 从 kernel helper / coordinator 形态推进到真实的 Payment Service。Kernel 只保留 payment policy primitive、service registry、trace/audit primitive 和系统不变量，不继续拥有 payment adapter、payment lifecycle orchestration 或 A2A settlement provider。

本轮必须严格遵守：

- `macaca/docs/agent-os-microkernel-boundaries.md`
- `macaca/docs/route-c-serviceization-allowlist.md`
- `macaca/docs/route-c-architecture-governance.md`
- `macaca/docs/design_patterns.md`

当前诊断：

- `macaca-proto/src/a2a.rs` 已有 provider-neutral A2A/payment value objects：agent identity、remote capability、quote、payment intent、budget policy、approval policy、receipt、execution proof 和 structured `A2AError`。
- `macaca-kernel/src/a2a.rs` 当前仍拥有 `A2ACoordinator`、`A2AProtocolAdapter`、local simulated adapter、full request/pay pipeline 和 event sink glue。这违背 Route C 的长期边界，因为 adapter execution 和 lifecycle orchestration 属于 replaceable service capability。
- `macaca-kernel/src/payment_policy.rs` 已有 `PaymentPolicyEngine` 和 default conservative policy。该部分更接近 kernel policy primitive，可以保留为 facade/strategy boundary，但不应执行 adapter 或持久化 payment artifacts。
- `macaca-kernel/src/a2a_event.rs` 已有 bounded payment lifecycle event observer contract。它可以继续作为 trace/audit primitive 或迁出为 proto/runtime-host DTO，但生产写入应通过 Payment Service provider。
- `macaca-persist/src/payment_store.rs` 已有 `PaymentStore` 和 `InMemoryPaymentStore`，可作为 Payment Service 的 repository adapter，而不是 kernel 直接依赖的 store。
- `macaca-runtime-host` 目前已有 ServiceRuntime、service provider、decorator、Store/Entitlement providers，但还没有 Payment Service provider。
- `macaca-sdk` 目前没有 `SystemPaymentClient` / `SystemA2AClient` focused clients。
- `macaca-web` / `macaca-cli` 目前没有 payment shell surface；后续如果新增 payment approval、receipt、A2A service call UI/CLI，必须通过 SDK/SystemFacade，而不是直接调用 kernel coordinator。

S10 不包含：

- 真实外部支付 provider、银行卡、企业账单、链上支付、钱包签名或区块链 settlement。这些必须作为后续 payment adapter plugin 或 S11 Web3/EVM optional module 接入。
- Web3/EVM 链上交易、智能合约、DApp execution 或 node lifecycle，属于 S11。
- Store/Entitlement 授权本身，属于 S9；S10 只负责 payment quote/intent/receipt，未来可被 Store/Entitlement 作为 metering/paid flow 的策略依赖。
- 完整 marketplace billing UI 或订阅商业运营后台。
- 删除现有 kernel A2A coordinator；首版应 additive-first，先提供 service path，再把旧入口标注 deprecated 并逐步迁移。

## 设计模式候选

### Facade

建立 focused Payment Service boundary：

- `PaymentService`：quote、create intent、evaluate policy、approve intent、settle intent、record receipt、query receipt、query transitions、snapshot。
- `SystemPaymentClient`：SDK shell-facing facade。
- `SystemA2AClient` 或 payment client 的 A2A focused methods：面向 agent-to-agent paid capability negotiation，不暴露 provider concrete type。

优点：

- Web/CLI/Gateway/Application/Agent 不再直接拿 kernel coordinator。
- Payment 实现可以替换为 local simulated、enterprise billing、remote payment service、future Web3 adapter。
- 与 S5/S6/S7/S9 的 `ServiceRuntime + focused SDK client` 模式一致。

风险：

- Payment 和 A2A 容易合并成一个巨型 macro-service。
- Mitigation：Payment Service 只拥有 money-like lifecycle；A2A protocol message formatting 仍属于 `macaca-framework` / protocol layer，remote agent discovery/routing 不写入 Payment provider。

### Mediator

Payment Service provider 作为 A2A paid capability 的 mediator：

`quote request -> adapter quote strategy -> intent builder -> policy evaluation -> approval state -> settlement adapter -> receipt/proof persistence -> trace/audit`

优点：

- 把跨 adapter、policy、store、trace 的复杂协作集中在 service provider，而不是散落在 kernel/Web/agent runtime。
- 后续可以为 local、remote、enterprise、chain-backed payment adapters 复用同一 lifecycle。

风险：

- Mediator 过胖会重演宏内核问题。
- Mitigation：provider 只编排 contract；policy、adapter、repository、approval provider、event sink 都是 Strategy/Adapter/Observer seam。

### Strategy

可替换策略：

- quote adapter strategy
- settlement adapter strategy
- budget policy strategy
- approval policy strategy
- receipt issuer strategy
- dispute/proof strategy
- unavailable/null payment strategy

优点：

- 不硬编码 payment provider、chain、wallet、gateway、region、application。
- 具体 adapter 可以后续由 plugin 或 optional module 提供。
- 性能开销可控：首版只实现 local simulated strategy 和 unavailable strategy。

风险：

- 过早设计太多 trait 会增加理解成本。
- Mitigation：首版稳定 service DTO、provider trait 和最小 adapter seam；高级 settlement/dispute/web3 adapter 留 metadata 和 future extension。

### Command

所有 Payment/A2A 操作先建 typed command，再转为 `ServiceCommand`：

- `payment.quote`
- `payment.intent.create`
- `payment.intent.evaluate_policy`
- `payment.intent.approve`
- `payment.intent.settle`
- `payment.receipt.get`
- `payment.receipt.list`
- `payment.transition.list`
- `payment.proof.list`
- `payment.snapshot`

优点：

- 每个入口可序列化、可审计、可 replay。
- SDK、Web、CLI、Gateway、remote transport 都能复用同一 contract。
- 未来 remote Payment Service 不需要暴露 Rust concrete type。

风险：

- command payload 如果携带 secret、private key、raw credential，会污染 trace。
- Mitigation：DTO 只允许 bounded identifiers、amount、rail code、capability id、session/task scope、operation、policy input 和 redacted metadata。

### State

Payment lifecycle 使用明确状态机：

`created -> quoted -> pending_approval -> approved -> executing -> settled -> receipt_recorded`

异常路径：

- `quoted -> rejected`
- `approved -> failed`
- `failed -> dispute_possible`

优点：

- 与现有 `PaymentIntentState::can_transition_to` 一致。
- 适合持久化 transition memento 和 UI/trace replay。
- 可以防止重复 settle、跳过 approval、无 quote 创建 receipt。

风险：

- 多 provider 可能有不同异步状态。
- Mitigation：canonical state 保持小集合；provider-specific state 放入 metadata，不改变 service contract。

### Memento

Payment artifacts 必须作为可 replay memento 保存：

- quote snapshot
- intent transitions
- receipt
- execution proof
- service snapshot

优点：

- 支持 session replay、审计、争议处理、跨进程恢复。
- 与现有 `PaymentStore` 设计一致。

风险：

- 保存过多原始 provider payload 会泄露敏感信息。
- Mitigation：memento 只保存 redacted provider-neutral view；原始凭据、签名材料、private key、raw request body 禁止进入 store。

### Observer

所有关键节点必须产生 structured logs / trace / audit：

- service provider register/start/stop
- quote requested/returned/failed
- intent created/state transitioned
- policy evaluated allow/deny/unavailable
- approval requested/granted/denied
- adapter settlement started/completed/failed
- receipt/proof persisted
- service snapshot queried

优点：

- 满足 Route C “无 trace 不执行”。
- 为 Web UI、session replay、compliance audit 和 future payment disputes 提供统一事实源。

风险：

- trace event 太多或 payload 太大影响 UI。
- Mitigation：trace 用 bounded event；大 payload 通过 resource handle 或 repository query，不进入实时 event。

### Adapter / Bridge

把现有 kernel Phase 09 A2A coordinator 能力桥接进 Payment Service：

- `A2AProtocolAdapter` 迁到 runtime-host 或 payment service provider 内部。
- `LocalSimulatedA2AAdapter` 作为 built-in local adapter。
- `PaymentStore` 作为 repository adapter。
- `A2APaymentEventSink` 桥接到 service trace/audit。
- kernel 旧 `A2ACoordinator` 保留 deprecated compatibility anchor。

优点：

- 不重写已验证的 policy/store/receipt path。
- 保持 additive-first，可逐步迁移 consumer。

风险：

- 如果 provider 直接复用 kernel coordinator，kernel 仍然拥有 adapter orchestration。
- Mitigation：S10 OpenSpec 必须要求 adapter orchestration 实现迁入 `macaca-runtime-host`，kernel 只保留 deprecated wrapper 或 policy primitive。

### Null Object

Payment Service 缺失或被 policy 禁用时：

- quote/create intent/settle 返回 structured unavailable。
- receipt list 返回 empty page + unavailable diagnostics。
- free/open package 和普通 local agent task 不受影响。
- paid A2A capability 调用必须 fail closed，不得静默 allow。

优点：

- base OS 不依赖 payment module。
- 不会因为没有 payment provider 阻塞普通应用。

风险：

- consumer 把 empty receipt 当成无须支付。
- Mitigation：mutating/payment-required command 必须返回 error；只有 read-only list/snapshot 可以返回 empty unavailable view。

### Specification

集中验证：

- trace required
- requester/provider/capability scope required
- amount and asset validity
- budget/approval policy presence
- lifecycle transition validity
- redaction rules
- adapter availability
- optional module availability

优点：

- 避免 Web、CLI、Gateway、Agent 各写 if/else。
- 与 dependency gate 和 Route C governance 一致。

风险：

- 规格过重影响首版交付。
- Mitigation：首版只实现 trace/scope/amount/transition/redaction specs，复杂合规留后续 policy plugin。

## 可选方案

### 方案 A：最小 SDK 包装现有 kernel coordinator

做法：

- 新增 `SystemPaymentClient`，内部仍调用 `macaca-kernel::A2ACoordinator`。
- 不新增 Payment Service provider。

优点：

- 改动小。
- 快速暴露 SDK API。

缺点：

- 没有真正服务化。
- kernel 继续拥有 adapter orchestration 和 payment store dependency。
- 不符合 `agent-os-microkernel-boundaries.md` 对 Payment/A2A 的归属要求。

结论：拒绝。

### 方案 B：一次性新增 Payment Service，并完全删除 kernel coordinator

做法：

- 新增 service DTO、provider、SDK client。
- 删除或大幅移动 kernel A2A coordinator。

优点：

- 架构最干净。
- 边界立即收敛。

缺点：

- 风险过高，容易破坏现有测试和兼容引用。
- 不符合 additive-first。
- 无法给后续迁移留搜索锚点。

结论：拒绝。

### 方案 C：Additive-first Payment Service + deprecated kernel compatibility anchor

做法：

- 新增 provider-neutral `payment_service` DTO。
- 在 `macaca-runtime-host` 新增 Payment Service provider，复用现有 `PaymentStore`、policy 和 local simulated adapter 的语义。
- 新增 `SystemPaymentClient` 和 `SystemFacade` accessor。
- Web startup 只作为 composition root 注册 built-in Payment Service。
- 旧 kernel coordinator 标注 deprecated，后续 consumer 迁移到 service path 后再删除 direct dependency。

优点：

- 真正建立 Route C service boundary。
- 风险可控，能保持当前测试通过。
- 和 S9 已落地方式一致。
- 为 S11 Web3/EVM payment adapter、plugin payment provider、remote A2A paid service 留扩展点。

缺点：

- 短期存在双路径。
- 需要 allowlist 文档明确债务和过期条件。

结论：推荐方案。

## 推荐设计

采用方案 C。

S10 应把 Payment/A2A 的生产入口收敛到 Payment Service：

- `macaca-proto` 增加 `payment_service.rs`，只定义 provider-neutral command/result/snapshot/event view。
- `macaca-runtime-host` 增加 `payment_service_provider.rs` 和 `payment_admission.rs`，负责 command decode、Specification validation、state transition、store persistence、adapter dispatch、trace/log emission。
- `macaca-sdk` 增加 `payment_client.rs`，提供 `SystemPaymentClient`、service-backed client 和 unavailable client，并挂入 `SystemFacade`。
- `macaca-web` 启动时注册内置 local simulated Payment provider，但 Web 不拥有 payment semantics。
- `macaca-kernel` 保留 `PaymentPolicyEngine` 作为 policy primitive；`A2ACoordinator`、`A2AProtocolAdapter`、`LocalSimulatedA2AAdapter` 标注 deprecated，并给出替代路径。
- `macaca-persist::PaymentStore` 保持 repository contract；后续 durable payment store 可以替换 in-memory adapter。
- 所有 payment command 必须携带 trace context；mutating command 无 trace 不执行。

## 风险与约束

- 依赖风险：如果 `macaca-runtime-host` 直接依赖 kernel coordinator，会延续宏内核边界。计划必须要求 provider 内部重新组合 policy/store/adapter，而不是把 coordinator 当黑盒。
- 数据风险：payment trace / receipt / proof 禁止记录 private key、wallet secret、provider credential、raw signed payload、API key、raw remote response、prompt body。
- 兼容风险：现有 kernel tests 必须继续通过；旧 API 只能 deprecated，不删除。
- 可选模块风险：Payment Service 缺失时普通应用不能失败；paid A2A 必须 fail closed。
- UI 风险：S10 不强制新增 Web payment UI，但后续 Web/CLI surface 必须只用 SDK client。
- 扩展风险：不得硬编码 provider、chain、gateway、application、driver、workflow 名称；payment rail 和 adapter identity 保持 string-backed + metadata extensible。

## 后续 OpenSpec 入口

建议 change id：

- `add-payment-a2a-service-v1`

建议规格：

- `payment-service`
- `payment-sdk-client`
- `payment-consumer-migration`
- `payment-audit-trace`

