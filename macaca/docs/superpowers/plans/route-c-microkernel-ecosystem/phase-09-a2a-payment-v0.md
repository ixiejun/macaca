# 阶段 9：A2A 协作与支付 v0 细分实施计划

## 目标

建立 Agent-to-Agent 协作与交易的基础协议。阶段 9 不接入真实链上支付，但必须完整建模 agent identity、capability discovery、quote、payment intent、budget、approval、receipt、trace，为未来 Ethereum/ERC/EIP、企业账单、MCP A2A 适配预留接口。

## 架构设计

A2A 不是普通 tool call。它包含服务发现、价格协商、预算控制、授权确认、执行结果、凭证和争议证据。Kernel 只负责策略和审计，不绑定具体支付协议。

推荐设计模式：

- Mediator：A2A coordinator 协调 requester、provider、payment、task、trace。
- Strategy：payment adapter 可替换。
- Command：quote request、payment intent、service request 都是 command。
- State：payment intent lifecycle 状态机。
- Memento：receipt、quote、terms、execution proof 可持久化。

## 涉及文件

- 新增：`macaca/crates/macaca-proto/src/a2a.rs`
- 新增：`macaca/crates/macaca-kernel/src/a2a.rs`
- 新增：`macaca/crates/macaca-kernel/src/payment_policy.rs`
- 新增：`macaca/crates/macaca-persist/src/payment_store.rs`
- 新增：`macaca/crates/macaca-task/src/a2a_task.rs`
- 新增测试：`macaca/crates/macaca-kernel/tests/a2a_payment.rs`

## 抽象设计

A2A 核心类型：

- `AgentIdentity`
- `RemoteCapabilityDescriptor`
- `QuoteRequest`
- `QuoteResponse`
- `PaymentIntent`
- `PaymentTerms`
- `BudgetPolicy`
- `ApprovalPolicy`
- `PaymentReceipt`
- `A2AProtocolAdapter`

Payment intent lifecycle：

```text
created -> quoted -> pending_approval -> approved -> executing -> settled -> receipt_recorded
created -> quoted -> rejected
approved -> failed -> dispute_possible
```

## 实施切片

### 切片 9.1：A2A proto

定义身份、报价、intent、terms、receipt 类型。

验证：

- quote/intent/receipt serde roundtrip。
- 金额、币种、计费单位不写死为单一链或单一法币。

### 切片 9.2：budget 与 approval policy

定义预算策略，默认所有真实支付都需要 explicit approval。模拟 payment 可以在测试中自动批准。

验证：

- 超预算 intent 被拒绝。
- 低于自动批准阈值的模拟 intent 可通过。
- 真实 payment adapter 未配置时返回 unavailable。

### 切片 9.3：local A2A adapter

实现本地模拟 A2A adapter，用于验证协议，不涉及真实支付。

验证：

- local agent 能请求 local provider quote。
- approved intent 能产生 receipt。

### 切片 9.4：trace 与 persistence

quote、approval、settlement、receipt 全部进入 EventLog/payment store。

验证：

- 每个状态转换都有 trace。
- receipt 可按 session/task 查询。

## 里程碑

- M9.1：A2A 类型稳定。
- M9.2：Budget/approval policy 生效。
- M9.3：local A2A 模拟链路跑通。
- M9.4：receipt 可追踪可持久化。

## 禁止事项

- 禁止让 agent 无预算自主支付。
- 禁止绑定某个 Ethereum 草案为 kernel 协议。
- 禁止真实支付 provider 在 policy 前接入。
- 禁止 payment event 不进入 trace。

## 验收命令

```bash
cargo test -p macaca-proto a2a
cargo test -p macaca-kernel a2a_payment
cargo test -p macaca-persist payment_store
```

