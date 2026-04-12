use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tokio::sync::broadcast;
use tracing::{debug, warn};

use macaca_proto::{IpcMessage, MacacaError, MacacaResult};

use crate::bus::{MessageReceiver, MessageSender};

const CHANNEL_CAPACITY: usize = 256;

/// Shared topic registry for the local bus.
type TopicMap = Arc<Mutex<HashMap<String, broadcast::Sender<IpcMessage>>>>;

/// In-process message bus that wires `LocalSender` / `LocalReceiver` pairs.
#[derive(Clone, Default)]
pub struct LocalBus {
    topics: TopicMap,
}

impl LocalBus {
    pub fn new() -> Self {
        Self {
            topics: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a sender that shares this bus's topic registry.
    pub fn sender(&self) -> LocalSender {
        LocalSender {
            topics: Arc::clone(&self.topics),
        }
    }

    /// Create a receiver that shares this bus's topic registry.
    pub fn receiver(&self) -> LocalReceiver {
        LocalReceiver {
            topics: Arc::clone(&self.topics),
            subscriptions: HashMap::new(),
        }
    }

    /// Obtain or create a broadcast channel for `topic`.
    fn get_or_create(topics: &TopicMap, topic: &str) -> broadcast::Sender<IpcMessage> {
        let mut map = topics.lock().unwrap();
        map.entry(topic.to_owned())
            .or_insert_with(|| {
                let (tx, _) = broadcast::channel(CHANNEL_CAPACITY);
                tx
            })
            .clone()
    }
}

// ── Sender ──────────────────────────────────────────────────────────────────

pub struct LocalSender {
    topics: TopicMap,
}

#[async_trait]
impl MessageSender for LocalSender {
    async fn send(&self, msg: IpcMessage) -> MacacaResult<()> {
        // Direct messages are delivered on the recipient's agent-id topic.
        let topic = match &msg.to {
            Some(id) => id.0.to_string(),
            None => {
                return Err(MacacaError::Ipc(
                    "send() requires msg.to to be set".to_owned(),
                ))
            }
        };
        self.publish(&topic, msg).await
    }

    async fn publish(&self, topic: &str, msg: IpcMessage) -> MacacaResult<()> {
        let tx = LocalBus::get_or_create(&self.topics, topic);
        debug!(topic, "local publish");
        match tx.send(msg) {
            Ok(receivers) => {
                debug!(topic, receivers, "message delivered");
                Ok(())
            }
            Err(_) => {
                // No active receivers — not an error, just no subscribers yet.
                warn!(topic, "no active receivers for topic");
                Ok(())
            }
        }
    }
}

// ── Receiver ─────────────────────────────────────────────────────────────────

pub struct LocalReceiver {
    topics: TopicMap,
    /// Active subscriptions keyed by topic name.
    subscriptions: HashMap<String, broadcast::Receiver<IpcMessage>>,
}

#[async_trait]
impl MessageReceiver for LocalReceiver {
    async fn recv(&mut self) -> MacacaResult<IpcMessage> {
        // Drain subscriptions until we get a message.
        loop {
            for rx in self.subscriptions.values_mut() {
                match rx.try_recv() {
                    Ok(msg) => return Ok(msg),
                    Err(broadcast::error::TryRecvError::Empty) => continue,
                    Err(broadcast::error::TryRecvError::Lagged(n)) => {
                        warn!(skipped = n, "local receiver lagged, messages dropped");
                        continue;
                    }
                    Err(broadcast::error::TryRecvError::Closed) => {
                        // channel closed, skip
                        continue;
                    }
                }
            }

            // No message found — yield to the runtime before trying again.
            tokio::task::yield_now().await;
        }
    }

    async fn subscribe(&mut self, topic: &str) -> MacacaResult<()> {
        if self.subscriptions.contains_key(topic) {
            return Ok(());
        }
        let tx = LocalBus::get_or_create(&self.topics, topic);
        let rx = tx.subscribe();
        self.subscriptions.insert(topic.to_owned(), rx);
        debug!(topic, "subscribed");
        Ok(())
    }

    async fn unsubscribe(&mut self, topic: &str) -> MacacaResult<()> {
        if self.subscriptions.remove(topic).is_some() {
            debug!(topic, "unsubscribed");
        }
        Ok(())
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

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
            payload: serde_json::json!({"hello": "world"}),
            timestamp: Utc::now(),
        }
    }

    #[tokio::test]
    async fn publish_and_receive() {
        let bus = LocalBus::new();
        let sender = bus.sender();
        let mut receiver = bus.receiver();

        receiver.subscribe("greet").await.unwrap();

        let msg = make_msg("greet");
        sender.publish("greet", msg.clone()).await.unwrap();

        let received = receiver.recv().await.unwrap();
        assert_eq!(received.id, msg.id);
        assert_eq!(received.topic, "greet");
    }

    #[tokio::test]
    async fn send_direct_requires_to() {
        let bus = LocalBus::new();
        let sender = bus.sender();
        let msg = make_msg("direct");
        // No `to` set — should error
        let result = sender.send(msg).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn send_direct_delivers_to_agent_topic() {
        let bus = LocalBus::new();
        let sender = bus.sender();
        let mut receiver = bus.receiver();

        let agent_id = AgentId::new();
        receiver.subscribe(&agent_id.0.to_string()).await.unwrap();

        let mut msg = make_msg("ping");
        msg.to = Some(agent_id);
        sender.send(msg.clone()).await.unwrap();

        let received = receiver.recv().await.unwrap();
        assert_eq!(received.id, msg.id);
    }

    #[tokio::test]
    async fn unsubscribe_stops_delivery() {
        let bus = LocalBus::new();
        let sender = bus.sender();
        let mut receiver = bus.receiver();

        receiver.subscribe("events").await.unwrap();
        receiver.unsubscribe("events").await.unwrap();

        sender.publish("events", make_msg("events")).await.unwrap();

        // Receiver has no subscriptions — try_recv returns nothing.
        assert!(receiver.subscriptions.is_empty());
    }

    #[tokio::test]
    async fn publish_with_no_subscribers_is_ok() {
        let bus = LocalBus::new();
        let sender = bus.sender();
        // No receiver subscribed — should not error.
        let result = sender.publish("orphan", make_msg("orphan")).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn multiple_topics() {
        let bus = LocalBus::new();
        let sender = bus.sender();
        let mut receiver = bus.receiver();

        receiver.subscribe("a").await.unwrap();
        receiver.subscribe("b").await.unwrap();

        let msg_a = make_msg("a");
        let msg_b = make_msg("b");
        sender.publish("a", msg_a.clone()).await.unwrap();
        sender.publish("b", msg_b.clone()).await.unwrap();

        let mut received_ids = std::collections::HashSet::new();
        received_ids.insert(receiver.recv().await.unwrap().id);
        received_ids.insert(receiver.recv().await.unwrap().id);

        assert!(received_ids.contains(&msg_a.id));
        assert!(received_ids.contains(&msg_b.id));
    }
}
