//! Gateway mediator boundary.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::error::MacacaResult;
use macaca_proto::types::GatewayEvent;

use crate::message::{GatewayInboundMessage, GatewayReply};

/// Event sink used by the mediator.
#[async_trait]
pub trait GatewayEventSink: Send + Sync {
    /// Dispatch a shared gateway event.
    async fn dispatch(&self, event: GatewayEvent) -> MacacaResult<()>;
}

/// Coordinates inbound gateway messages and event dispatch.
pub struct GatewayMediator {
    sink: Arc<dyn GatewayEventSink>,
}

impl GatewayMediator {
    /// Create a mediator with an event sink.
    pub fn new(sink: Arc<dyn GatewayEventSink>) -> Self {
        Self { sink }
    }

    /// Handle one inbound message.
    pub async fn handle_inbound(
        &self,
        message: GatewayInboundMessage,
    ) -> MacacaResult<Option<GatewayReply>> {
        self.sink.dispatch(message.to_task_request_event()).await?;
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingSink {
        count: AtomicUsize,
    }

    #[async_trait]
    impl GatewayEventSink for CountingSink {
        async fn dispatch(&self, event: GatewayEvent) -> MacacaResult<()> {
            match event {
                GatewayEvent::TaskRequest { content, .. } => {
                    assert_eq!(content, "build app");
                }
                other => panic!("expected TaskRequest, got {other:?}"),
            }
            self.count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn mediator_dispatches_task_request() {
        let sink = Arc::new(CountingSink {
            count: AtomicUsize::new(0),
        });
        let mediator = GatewayMediator::new(sink.clone());

        mediator
            .handle_inbound(GatewayInboundMessage::text(
                "telegram",
                "u1",
                "c1",
                "build app",
            ))
            .await
            .unwrap();

        assert_eq!(sink.count.load(Ordering::SeqCst), 1);
    }
}
