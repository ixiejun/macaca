//! Telegram stub adapter.
//!
//! This adapter implements the [`ImAdapter`] trait but does not connect to
//! the real Telegram Bot API. It serves as a structural placeholder until
//! the `teloxide` SDK is integrated.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::info;

use macaca_proto::config::TelegramConfig;
use macaca_proto::error::MacacaResult;

use crate::adapter::{EventHandler, ImAdapter};

/// Stub Telegram adapter backed by [`TelegramConfig`].
///
/// All methods log their invocation but perform no real I/O.
pub struct TelegramAdapter {
    config: TelegramConfig,
}

impl TelegramAdapter {
    /// Create a new Telegram adapter from configuration.
    pub fn new(config: TelegramConfig) -> Self {
        Self { config }
    }

    /// Access the underlying configuration.
    pub fn config(&self) -> &TelegramConfig {
        &self.config
    }
}

#[async_trait]
impl ImAdapter for TelegramAdapter {
    fn name(&self) -> &str {
        "telegram"
    }

    async fn start(&self, _handler: Arc<dyn EventHandler>) -> MacacaResult<()> {
        info!(
            bot_token_env = %self.config.bot_token_env,
            allowed_users = ?self.config.allowed_user_ids,
            "Telegram adapter started (stub)"
        );
        Ok(())
    }

    async fn send_message(&self, channel_id: &str, content: &str) -> MacacaResult<()> {
        info!(
            channel_id = %channel_id,
            content_len = content.len(),
            "Telegram send_message (stub)"
        );
        Ok(())
    }

    async fn stop(&self) -> MacacaResult<()> {
        info!("Telegram adapter stopped (stub)");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> TelegramConfig {
        TelegramConfig {
            enabled: true,
            bot_token_env: "TEST_TELEGRAM_TOKEN".into(),
            allowed_user_ids: vec!["user1".into(), "user2".into()],
        }
    }

    #[test]
    fn telegram_adapter_name() {
        let adapter = TelegramAdapter::new(test_config());
        assert_eq!(adapter.name(), "telegram");
    }

    #[test]
    fn telegram_adapter_config_access() {
        let cfg = test_config();
        let adapter = TelegramAdapter::new(cfg.clone());
        assert_eq!(adapter.config().bot_token_env, "TEST_TELEGRAM_TOKEN");
        assert_eq!(adapter.config().allowed_user_ids.len(), 2);
    }

    #[tokio::test]
    async fn telegram_adapter_start_stop() {
        use crate::gateway::DefaultEventHandler;

        let adapter = TelegramAdapter::new(test_config());
        let handler: Arc<dyn EventHandler> = Arc::new(DefaultEventHandler);
        adapter.start(handler).await.unwrap();
        adapter.stop().await.unwrap();
    }

    #[tokio::test]
    async fn telegram_adapter_send_message() {
        let adapter = TelegramAdapter::new(test_config());
        adapter.send_message("chat_123", "Hello from test").await.unwrap();
    }
}
