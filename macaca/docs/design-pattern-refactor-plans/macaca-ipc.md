# macaca-ipc 设计模式渐进式重构计划

## 当前职责

`macaca-ipc` 提供进程间通信抽象，包括本地通道和 NATS 等 transport。它支撑 Agent OS 后续拆进程、分布式运行和远程 worker。

重点对象：

- `MessageSender` / `MessageReceiver`。
- Local IPC。
- NATS IPC。

## 适用模式

| 位置 | 现状风险 | 设计模式 | 渐进目标 |
| --- | --- | --- | --- |
| transport 实现 | local/nats 的发送接收细节不同 | Bridge | 抽象 message channel 与 transport 实现 |
| 消息协议 | 外部 broker 格式与内部 message 不一致 | Adapter | transport adapter 负责序列化与 topic 映射 |
| sender/receiver 创建 | 根据配置选择 transport 容易写 if/else | Factory Method | `IpcTransportFactory` |
| 远程调用 | broker 网络异常、超时、重试需要统一处理 | Proxy | `ResilientIpcProxy` 包装 transport |

## 小步重构计划

1. 第一切片：明确 `IpcTransport` trait，local/nats 作为实现。
2. 第二切片：新增 `IpcTransportFactory`，旧配置解析委托给工厂。
3. 第三切片：抽出 `MessageCodec`，把 serde/json/bincode 选择从 transport 中分离。
4. 第四切片：在 proxy 层统一 timeout、retry、metrics、trace。

## 示例代码片段

```rust
pub trait IpcTransport: Send + Sync {
    async fn send(&self, envelope: IpcEnvelope) -> Result<(), IpcError>;
    async fn receive(&self) -> Result<IpcEnvelope, IpcError>;
}

pub trait MessageCodec: Send + Sync {
    fn encode(&self, msg: &IpcMessage) -> Result<Vec<u8>, IpcError>;
    fn decode(&self, bytes: &[u8]) -> Result<IpcMessage, IpcError>;
}

pub struct IpcBridge<T, C> {
    transport: T,
    codec: C,
}
```

## 验证策略

- local transport 和 nats transport 运行同一套 contract tests。
- 对消息 envelope 做 snapshot，防止 topic/session/task id 丢失。
- proxy 层重构时注入 fake transport 模拟 timeout 和 broker disconnect。

