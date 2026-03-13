use async_trait::async_trait;
use macaca_proto::{MacacaResult, IpcMessage};

/// Sends messages to agents or topics.
#[async_trait]
pub trait MessageSender: Send + Sync {
    /// Send a direct message to the agent specified in `msg.to`.
    async fn send(&self, msg: IpcMessage) -> MacacaResult<()>;

    /// Publish a message to a named topic.
    async fn publish(&self, topic: &str, msg: IpcMessage) -> MacacaResult<()>;
}

/// Receives messages from agents or topics.
#[async_trait]
pub trait MessageReceiver: Send + Sync {
    /// Receive the next message.
    async fn recv(&mut self) -> MacacaResult<IpcMessage>;

    /// Subscribe to a named topic.
    async fn subscribe(&mut self, topic: &str) -> MacacaResult<()>;

    /// Unsubscribe from a named topic.
    async fn unsubscribe(&mut self, topic: &str) -> MacacaResult<()>;
}
