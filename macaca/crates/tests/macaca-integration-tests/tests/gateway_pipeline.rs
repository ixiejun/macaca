//! Integration tests for the Gateway adapter pipeline:
//! adapter registration, start/stop, and event handling.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;

use macaca_gateway::{
    DiscordAdapter, GatewayBuilder, GatewayEventSink, GatewayInboundMessage, GatewayMediator,
    GatewayReply, GatewayTransport, TelegramAdapter,
};
use macaca_proto::config::{DiscordConfig, GatewayConfig, TelegramConfig};
use macaca_proto::{GatewayEvent, MacacaResult};

// ---------------------------------------------------------------------------
// Counting event sink
// ---------------------------------------------------------------------------

struct CountingEventSink {
    count: AtomicUsize,
}

impl CountingEventSink {
    fn new() -> Self {
        Self {
            count: AtomicUsize::new(0),
        }
    }

    fn event_count(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl GatewayEventSink for CountingEventSink {
    async fn dispatch(&self, _event: GatewayEvent) -> MacacaResult<()> {
        self.count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Config helpers
// ---------------------------------------------------------------------------

fn telegram_config() -> TelegramConfig {
    TelegramConfig {
        enabled: true,
        bot_token_env: "TEST_TELEGRAM_TOKEN".into(),
        allowed_user_ids: vec!["user1".into()],
    }
}

fn discord_config() -> DiscordConfig {
    DiscordConfig {
        enabled: true,
        bot_token_env: "TEST_DISCORD_TOKEN".into(),
        command_prefix: "!".into(),
    }
}

fn gateway_config(telegram_enabled: bool, discord_enabled: bool) -> GatewayConfig {
    GatewayConfig {
        enabled: true,
        telegram: Some(TelegramConfig {
            enabled: telegram_enabled,
            ..telegram_config()
        }),
        discord: Some(DiscordConfig {
            enabled: discord_enabled,
            ..discord_config()
        }),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Register both Telegram and Discord adapters, start all, stop all.
#[tokio::test]
async fn builder_registers_and_starts_configured_adapters() {
    let gateway = GatewayBuilder::new(gateway_config(true, true))
        .start()
        .await
        .unwrap();
    assert_eq!(gateway.adapter_count(), 2);

    gateway.stop().await.unwrap();
}

/// GatewayMediator dispatches inbound messages to its event sink.
#[tokio::test]
async fn mediator_dispatches_inbound_message_to_event_sink() {
    let sink = Arc::new(CountingEventSink::new());
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

    assert_eq!(sink.event_count(), 1);
}

/// Gateway builder respects disabled top-level config.
#[tokio::test]
async fn builder_disabled_gateway_registers_no_adapters() {
    let gateway = GatewayBuilder::new(GatewayConfig {
        enabled: false,
        telegram: Some(telegram_config()),
        discord: Some(discord_config()),
    })
    .start()
    .await
    .unwrap();

    assert_eq!(gateway.adapter_count(), 0);
    gateway.stop().await.unwrap();
}

/// Empty builder output starts and stops without error.
#[tokio::test]
async fn builder_empty_gateway_start_stop() {
    let gateway = GatewayBuilder::new(gateway_config(false, false))
        .start()
        .await
        .unwrap();
    assert_eq!(gateway.adapter_count(), 0);
    gateway.stop().await.unwrap();
}

/// GatewayTransport send_reply works on stub adapters.
#[tokio::test]
async fn transport_send_reply() {
    let telegram = TelegramAdapter::new(telegram_config());
    telegram
        .send_reply(&GatewayReply::text(
            "telegram",
            "chat_123",
            "Hello from integration test",
        ))
        .await
        .unwrap();

    let discord = DiscordAdapter::new(discord_config());
    discord
        .send_reply(&GatewayReply::text(
            "discord",
            "channel_456",
            "Hello from integration test",
        ))
        .await
        .unwrap();
}
