//! Platform-neutral gateway message primitives.

use macaca_proto::types::GatewayEvent;

/// A platform-neutral inbound message received from an external gateway.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayInboundMessage {
    pub platform: String,
    pub user_id: String,
    pub channel_id: String,
    pub content: String,
}

impl GatewayInboundMessage {
    /// Build a text inbound message.
    pub fn text(
        platform: impl Into<String>,
        user_id: impl Into<String>,
        channel_id: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            platform: platform.into(),
            user_id: user_id.into(),
            channel_id: channel_id.into(),
            content: content.into(),
        }
    }

    /// Convert this inbound message to the existing compatibility event.
    pub fn to_task_request_event(&self) -> GatewayEvent {
        GatewayEvent::TaskRequest {
            user_id: self.user_id.clone(),
            channel_id: self.channel_id.clone(),
            content: self.content.clone(),
        }
    }
}

/// A platform-neutral outbound message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayOutboundMessage {
    pub platform: String,
    pub channel_id: String,
    pub content: String,
}

impl GatewayOutboundMessage {
    /// Build a text outbound message.
    pub fn text(
        platform: impl Into<String>,
        channel_id: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            platform: platform.into(),
            channel_id: channel_id.into(),
            content: content.into(),
        }
    }
}

/// A reply emitted by gateway orchestration to a transport.
pub type GatewayReply = GatewayOutboundMessage;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inbound_task_request_converts_to_gateway_event() {
        let message = GatewayInboundMessage::text("telegram", "u1", "c1", "build app");
        let event = message.to_task_request_event();

        match event {
            GatewayEvent::TaskRequest {
                user_id,
                channel_id,
                content,
            } => {
                assert_eq!(user_id, "u1");
                assert_eq!(channel_id, "c1");
                assert_eq!(content, "build app");
            }
            other => panic!("expected TaskRequest, got {other:?}"),
        }
    }

    #[test]
    fn outbound_reply_carries_target_and_content() {
        let reply = GatewayReply::text("telegram", "c1", "done");
        assert_eq!(reply.platform, "telegram");
        assert_eq!(reply.channel_id, "c1");
        assert_eq!(reply.content, "done");
    }
}
