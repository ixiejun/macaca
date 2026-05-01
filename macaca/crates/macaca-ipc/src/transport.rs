use std::sync::Arc;

use macaca_proto::MacacaResult;

use crate::bus::{MessageReceiver, MessageSender};
use crate::local::LocalBus;
use crate::nats::NatsBus;

/// Dynamic sender type returned by the transport bridge.
pub type DynMessageSender = Arc<dyn MessageSender>;

/// Dynamic receiver type returned by the transport bridge.
pub type DynMessageReceiver = Box<dyn MessageReceiver>;

/// Selects which IPC transport implementation to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcTransportKind {
    Local,
    Nats,
}

/// Explicit configuration for building an IPC transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcTransportConfig {
    Local,
    Nats { url: String },
}

/// Bridge contract for IPC transports.
pub trait IpcTransport: Send + Sync {
    fn kind(&self) -> IpcTransportKind;

    fn create_sender(&self) -> DynMessageSender;

    fn create_receiver(&self) -> DynMessageReceiver;
}

/// Factory for transport selection from explicit configuration.
pub struct IpcTransportFactory;

impl IpcTransportFactory {
    pub async fn build(config: &IpcTransportConfig) -> MacacaResult<Box<dyn IpcTransport>> {
        match config {
            IpcTransportConfig::Local => Ok(Box::new(LocalBus::new())),
            IpcTransportConfig::Nats { url } => Ok(Box::new(NatsBus::connect(url).await?)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use macaca_proto::{AgentId, IpcMessage, MessageId};

    fn make_msg(topic: &str) -> IpcMessage {
        IpcMessage {
            id: MessageId::new(),
            from: AgentId::new(),
            to: None,
            topic: topic.to_owned(),
            payload: serde_json::json!({"hello": "bridge"}),
            timestamp: Utc::now(),
        }
    }

    #[tokio::test]
    async fn factory_builds_local_transport() {
        let transport = IpcTransportFactory::build(&IpcTransportConfig::Local)
            .await
            .unwrap();

        assert_eq!(transport.kind(), IpcTransportKind::Local);

        let sender = transport.create_sender();
        let mut receiver = transport.create_receiver();
        receiver.subscribe("factory.local").await.unwrap();

        let msg = make_msg("factory.local");
        sender.publish("factory.local", msg.clone()).await.unwrap();

        let received = receiver.recv().await.unwrap();
        assert_eq!(received.id, msg.id);
    }
}
