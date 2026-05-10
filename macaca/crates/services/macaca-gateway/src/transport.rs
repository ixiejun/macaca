//! Gateway transport boundary.

use async_trait::async_trait;
use macaca_proto::error::MacacaResult;

use crate::message::GatewayReply;

/// Platform transport abstraction used by gateway orchestration.
#[async_trait]
pub trait GatewayTransport: Send + Sync {
    /// Human-readable transport name.
    fn name(&self) -> &str;

    /// Send a gateway reply through this transport.
    async fn send_reply(&self, reply: &GatewayReply) -> MacacaResult<()>;

    /// Stop the transport.
    async fn stop(&self) -> MacacaResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct MockTransport {
        sent: AtomicUsize,
    }

    #[async_trait]
    impl GatewayTransport for MockTransport {
        fn name(&self) -> &str {
            "mock"
        }

        async fn send_reply(&self, _reply: &GatewayReply) -> MacacaResult<()> {
            self.sent.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn stop(&self) -> MacacaResult<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn transport_sends_reply() {
        let transport = MockTransport {
            sent: AtomicUsize::new(0),
        };
        let reply = GatewayReply::text("mock", "c1", "done");

        transport.send_reply(&reply).await.unwrap();

        assert_eq!(transport.name(), "mock");
        assert_eq!(transport.sent.load(Ordering::SeqCst), 1);
    }
}
