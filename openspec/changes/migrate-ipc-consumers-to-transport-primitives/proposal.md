# Change: Migrate IPC Consumers to Transport Bridge Primitives

## Why

`macaca-ipc` 已经完成第一轮基于设计模式的 `Bridge + Factory` 重构，并将旧的 `LocalBus::{sender,receiver}` / `NatsBus::{sender,receiver}` 标记为 deprecated。  
如果上层调用方继续保留旧入口，本轮重构就停留在 crate 内部，无法形成真正的消费迁移，也无法为后续 transport 配置和远程 IPC 演进建立稳定边界。

当前仓库中已发现的真实上层 deprecated 调用点位于 `macaca-kernel`，并且 `IpcServiceAdapter` 仍然对具体 sender 泛型建模，没有直接消费新的 dynamic transport sender contract。

## What Changes

- 迁移所有已发现的上层 deprecated IPC 调用点到新的 transport bridge 入口。
- 将 `macaca-kernel::IpcServiceAdapter` 的 sender 构造边界收敛到 `macaca_ipc::DynMessageSender`。
- 保持 `AgentServices::IpcService` 语义、消息发送行为和现有测试行为不变。
- 不新增伪造的 factory 调用点；只有在存在真实 transport 选择需求时才接入 `IpcTransportFactory`。

## Impact

- Affected specs:
  - `macaca-ipc-consumer-migration`
- Affected code:
  - `macaca/crates/macaca-kernel/src/services.rs`
- Compatibility:
  - deprecated 旧 IPC 入口保留，但上层已发现调用点将迁出
  - 不改变 `IpcMessage`、`IpcService`、`MessageSender` / `MessageReceiver` 语义
