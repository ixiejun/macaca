# macaca-gateway 设计模式渐进式重构计划

## 当前职责

`macaca-gateway` 负责外部消息通道接入，例如 Telegram、Discord 等，把外部用户消息转成 Agent OS 内部会话和事件。

重点对象：

- Gateway 核心调度。
- Telegram / Discord adapters。
- 外部消息事件处理。

## 适用模式

| 位置 | 现状风险 | 设计模式 | 渐进目标 |
| --- | --- | --- | --- |
| 平台接入 | 每个平台协议字段不同，容易污染内部 session 语义 | Adapter | 平台消息统一转换成 `GatewayMessage` |
| 多平台协调 | gateway 同时处理连接、消息、回复、错误 | Mediator | `GatewayMediator` 协调 transport、session、reply |
| 消息推送 | 平台事件和 Agent OS event 是发布订阅关系 | Observer | 每个 adapter 订阅内部 event stream |
| 平台发送策略 | 不同平台 rate limit、formatting、reply threading 不同 | Strategy | `GatewayTransportStrategy` |

## 小步重构计划

1. 第一切片：抽出 `GatewayMessage` 和 `GatewayReply`，让 adapter 只做协议转换。
2. 第二切片：定义 `GatewayTransport` trait，Telegram/Discord 实现该 trait。
3. 第三切片：引入 `GatewayMediator`，把 session lookup、chat_v2 调用、event replay 集中。
4. 第四切片：为不同平台增加 markdown/plain text formatting strategy。

## 示例代码片段

```rust
pub trait GatewayTransport: Send + Sync {
    async fn poll(&self) -> Result<GatewayMessage, GatewayError>;
    async fn send(&self, reply: GatewayReply) -> Result<(), GatewayError>;
}

pub struct GatewayMediator<T> {
    transport: T,
    session_router: GatewaySessionRouter,
}

impl<T: GatewayTransport> GatewayMediator<T> {
    pub async fn run_once(&self) -> Result<(), GatewayError> {
        let msg = self.transport.poll().await?;
        let session = self.session_router.resolve(&msg).await?;
        let reply = session.dispatch(msg).await?;
        self.transport.send(reply).await
    }
}
```

## 验证策略

- 用平台无关 fixture 测试 GatewayMessage 到 session input 的映射。
- 保留 Telegram/Discord 协议字段 snapshot，确保 adapter 不丢字段。
- 每次迁移一个 transport，避免同时影响多个平台。

