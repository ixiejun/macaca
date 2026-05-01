## Context

`macaca-ipc` 在重构顺序表中属于阶段 1 的基础 contract crate。  
这里的目标不是增加新业务能力，而是先把 transport contract 稳定下来，避免上层继续依赖具体 bus 类型和 transport 选择细节。

当前约束：

- 行为必须 1:1 保持。
- 不引入新的 transport 或新的协议格式。
- 不在这一轮引入 timeout/retry/proxy 语义。
- 旧入口必须保留，但要明确标记 deprecated，方便后续逐步迁移。

## Goals / Non-Goals

- Goals
  - 提供统一 `IpcTransport` bridge contract。
  - 提供 `IpcTransportFactory` 统一创建入口。
  - 让 `LocalBus` / `NatsBus` 作为兼容 facade 委托到新原语。
  - 将旧入口显式标记为 deprecated。

- Non-Goals
  - 不改变 `IpcMessage` 结构。
  - 不引入 `MessageCodec`。
  - 不引入 timeout / retry / metrics / trace proxy。
  - 不强制迁移所有上层调用方到新 API。

## Decisions

### Decision: 先引入 Bridge + Factory，不一次做 Codec / Proxy

原因：

- 当前 `macaca-ipc` 体量很小，先做完整 codec/proxy 会过度设计。
- 最迫切的问题是 transport 选择和 transport 实现耦合，而不是消息格式不够抽象。
- 先稳定 contract，再给后续切片留 additive-first 扩展位，风险最低。

### Decision: 新原语返回 boxed sender/receiver trait objects

新 transport contract 直接对外提供：

- `Arc<dyn MessageSender>`
- `Box<dyn MessageReceiver>`

原因：

- `MessageReceiver::recv(&mut self)` 天然要求可变 receiver，`Box<dyn ...>` 最直接。
- 上层真正依赖的是行为，不是具体 `LocalReceiver` / `NatsReceiver` 类型。
- 这样可以让 factory 统一返回值，而不暴露 transport 内部实现。

### Decision: 旧 bus 类型保留为 compatibility facade

`LocalBus` / `NatsBus` 保留，但：

- 构造和 transport 逻辑委托给新 bridge/factory 原语
- `sender()` / `receiver()` 旧方法标记 deprecated

原因：

- 满足 additive-first 约束
- 调用方迁移可以渐进完成
- deprecated 标记本身就是后续清理的查找点

## Risks / Trade-offs

- 风险：新旧 API 并存，短期内会增加 surface area
  - Mitigation：旧入口只做 facade，不复制逻辑

- 风险：trait object 引入少量动态分发
  - Mitigation：IPC 不是高频 CPU 热路径，这个开销远小于可维护性收益

- 风险：local/nats 订阅语义不同，抽象时可能意外改变行为
  - Mitigation：不统一语义细节，只统一创建和行为 contract；测试覆盖 direct send / publish / subscribe / unsubscribe

## Migration Plan

1. 引入 `IpcTransport`、`IpcTransportKind`、`IpcTransportConfig`、`IpcTransportFactory`
2. 让 local/nats 适配到新 contract
3. 保留旧 bus 和旧构造路径，标记 deprecated
4. 补单测，确保旧路径和新路径行为一致

## Open Questions

- 下一轮是否需要 `MessageCodec`
- 下一轮是否需要 `ResilientIpcProxy`

这两个问题本轮不实现，等上层有真实需求后再开新 change。
