//! Config-driven gateway construction.
#![allow(deprecated)]

use std::sync::Arc;

use macaca_proto::config::GatewayConfig;
use macaca_proto::error::MacacaResult;

use crate::{DefaultEventHandler, DiscordAdapter, EventHandler, Gateway, TelegramAdapter};

/// Builds a gateway from config without caller-side platform branching.
pub struct GatewayBuilder {
    config: GatewayConfig,
    handler: Arc<dyn EventHandler>,
}

/// Started gateway lifecycle handle returned by [`GatewayBuilder`].
///
/// This keeps legacy gateway lifecycle calls inside `macaca-gateway` while
/// giving upper crates a non-deprecated shutdown path.
pub struct RunningGateway {
    gateway: Gateway,
}

impl RunningGateway {
    /// Number of configured gateway adapters.
    pub fn adapter_count(&self) -> usize {
        self.gateway.adapter_count()
    }

    /// Stop all adapters owned by this running gateway.
    #[allow(deprecated)]
    pub async fn stop(&self) -> MacacaResult<()> {
        self.gateway.stop_all().await
    }
}

impl GatewayBuilder {
    /// Create a builder with the default event handler.
    #[allow(deprecated)]
    pub fn new(config: GatewayConfig) -> Self {
        Self {
            config,
            handler: Arc::new(DefaultEventHandler),
        }
    }

    /// Override the event handler.
    pub fn with_handler(mut self, handler: Arc<dyn EventHandler>) -> Self {
        self.handler = handler;
        self
    }

    /// Build a gateway using configured adapters.
    #[allow(deprecated)]
    pub fn build(self) -> Gateway {
        let mut gateway = Gateway::new(self.handler);

        if self.config.enabled {
            if let Some(config) = self.config.telegram {
                if config.enabled {
                    gateway.register_adapter(Box::new(TelegramAdapter::new(config)));
                }
            }

            if let Some(config) = self.config.discord {
                if config.enabled {
                    gateway.register_adapter(Box::new(DiscordAdapter::new(config)));
                }
            }
        }

        gateway
    }

    /// Build and start the configured gateway.
    #[allow(deprecated)]
    pub async fn start(self) -> MacacaResult<RunningGateway> {
        let gateway = self.build();
        gateway.start_all().await?;
        Ok(RunningGateway { gateway })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::config::{DiscordConfig, TelegramConfig};

    fn telegram(enabled: bool) -> TelegramConfig {
        TelegramConfig {
            enabled,
            bot_token_env: "TEST_TELEGRAM_TOKEN".into(),
            allowed_user_ids: vec![],
        }
    }

    fn discord(enabled: bool) -> DiscordConfig {
        DiscordConfig {
            enabled,
            bot_token_env: "TEST_DISCORD_TOKEN".into(),
            command_prefix: "!".into(),
        }
    }

    #[test]
    fn disabled_gateway_registers_no_adapters() {
        let gateway = GatewayBuilder::new(GatewayConfig {
            enabled: false,
            telegram: Some(telegram(true)),
            discord: Some(discord(true)),
        })
        .build();

        assert_eq!(gateway.adapter_count(), 0);
    }

    #[test]
    fn enabled_telegram_registers_one_adapter() {
        let gateway = GatewayBuilder::new(GatewayConfig {
            enabled: true,
            telegram: Some(telegram(true)),
            discord: Some(discord(false)),
        })
        .build();

        assert_eq!(gateway.adapter_count(), 1);
    }

    #[test]
    fn enabled_telegram_and_discord_register_two_adapters() {
        let gateway = GatewayBuilder::new(GatewayConfig {
            enabled: true,
            telegram: Some(telegram(true)),
            discord: Some(discord(true)),
        })
        .build();

        assert_eq!(gateway.adapter_count(), 2);
    }
}
