## Context

`macaca-ipc` 已经提供：

- `IpcTransport`
- `DynMessageSender`
- `DynMessageReceiver`
- `IpcTransportFactory`

但上层消费层仍然保留旧习惯：

- 直接使用 `LocalBus::sender()`
- `IpcServiceAdapter` 仍按泛型 sender 建模

这会让上层继续依赖具体 bus API，而不是新 bridge contract。

## Goals / Non-Goals

- Goals
  - 迁移全部已发现的 deprecated IPC 上层调用点
  - 让 `macaca-kernel` 的 IPC adapter 构造边界直接消费 `DynMessageSender`
  - 保持行为 1:1 不变

- Non-Goals
  - 不引入新的 transport 配置来源
  - 不在无真实需求的模块里强行接入 `IpcTransportFactory`
  - 不改 `macaca-ipc` 的 transport 行为本身

## Decisions

### Decision: 只迁移真实上层消费点，不制造 fake factory usage

目前真实 deprecated 调用点只有 `macaca-kernel/src/services.rs` 测试。  
如果为了“全量迁移”而把 factory 硬塞到其他 crate，会变成凭空增加复杂度，而不是实际消费迁移。

### Decision: `IpcServiceAdapter` 收敛到 `DynMessageSender`

把 `IpcServiceAdapter` 从：

- `IpcServiceAdapter<S: MessageSender>`

收敛到：

- `IpcServiceAdapter { sender: DynMessageSender }`

原因：

- 上层真正依赖的是发送行为，不是 sender 的具体类型参数
- 这样可以直接承接 `IpcTransport::create_sender()`
- 未来若需要由 `IpcTransportFactory` 构造 sender，不需要再次改 adapter 边界

## Risks / Trade-offs

- 风险：`IpcServiceAdapter` 由泛型变成 trait object，会引入动态分发
  - Mitigation：IPC 发送不是 CPU 热路径，动态分发开销可以接受

- 风险：本轮只迁到 `create_sender()`，没有把 factory 推到更多上层
  - Mitigation：这是刻意控制范围；真实 transport selection 需求出现时再开下一轮 change

## Migration Plan

1. 扫描并确认所有 deprecated IPC 上层调用点
2. 收敛 `IpcServiceAdapter` 到 `DynMessageSender`
3. 将已发现调用点迁移到 `create_sender()`
4. 运行 `kernel` 和 workspace 验证
