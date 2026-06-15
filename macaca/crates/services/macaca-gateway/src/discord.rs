//! Discord stub adapter.
//!
//! This adapter implements the [`ImAdapter`] trait but does not connect to the
//! real Discord API. It serves as a structural placeholder until a production
//! transport provider is installed.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::info;

use macaca_proto::config::DiscordConfig;
use macaca_proto::error::MacacaResult;

use crate::adapter::{EventHandler, ImAdapter};
use crate::message::GatewayReply;
use crate::transport::GatewayTransport;

/// Stub Discord adapter backed by [`DiscordConfig`].
///
/// All methods log their invocation but perform no real I/O.
pub struct DiscordAdapter {
    config: DiscordConfig,
}

impl DiscordAdapter {
    /// Create a new Discord adapter from configuration.
    pub fn new(config: DiscordConfig) -> Self {
        Self { config }
    }

    /// Access the underlying configuration.
    pub fn config(&self) -> &DiscordConfig {
        &self.config
    }
}

#[async_trait]
impl ImAdapter for DiscordAdapter {
    fn name(&self) -> &str {
        "discord"
    }

    async fn start(&self, _handler: Arc<dyn EventHandler>) -> MacacaResult<()> {
        info!(
            bot_token_env = %self.config.bot_token_env,
            command_prefix = %self.config.command_prefix,
            "Discord adapter started (stub)"
        );
        Ok(())
    }

    async fn send_message(&self, channel_id: &str, content: &str) -> MacacaResult<()> {
        info!(
            channel_id = %channel_id,
            content_len = content.len(),
            "Discord send_message (stub)"
        );
        Ok(())
    }

    async fn stop(&self) -> MacacaResult<()> {
        info!("Discord adapter stopped (stub)");
        Ok(())
    }
}

#[async_trait]
impl GatewayTransport for DiscordAdapter {
    fn name(&self) -> &str {
        "discord"
    }

    async fn send_reply(&self, reply: &GatewayReply) -> MacacaResult<()> {
        self.send_message(&reply.channel_id, &reply.content).await
    }

    async fn stop(&self) -> MacacaResult<()> {
        ImAdapter::stop(self).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> DiscordConfig {
        DiscordConfig {
            enabled: true,
            bot_token_env: "TEST_DISCORD_TOKEN".into(),
            command_prefix: "!".into(),
        }
    }

    #[test]
    fn discord_adapter_name() {
        let adapter = DiscordAdapter::new(test_config());
        assert_eq!(ImAdapter::name(&adapter), "discord");
    }

    #[test]
    fn discord_adapter_config_access() {
        let cfg = test_config();
        let adapter = DiscordAdapter::new(cfg.clone());
        assert_eq!(adapter.config().bot_token_env, "TEST_DISCORD_TOKEN");
        assert_eq!(adapter.config().command_prefix, "!");
    }

    #[tokio::test]
    async fn discord_adapter_start_stop() {
        use crate::gateway::DefaultEventHandler;

        let adapter = DiscordAdapter::new(test_config());
        let handler: Arc<dyn EventHandler> = Arc::new(DefaultEventHandler);
        adapter.start(handler).await.unwrap();
        ImAdapter::stop(&adapter).await.unwrap();
    }

    #[tokio::test]
    async fn discord_adapter_send_message() {
        let adapter = DiscordAdapter::new(test_config());
        adapter
            .send_message("channel_456", "Hello from Discord test")
            .await
            .unwrap();
    }
}
