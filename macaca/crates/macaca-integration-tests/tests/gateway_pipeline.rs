//! Integration tests for the Gateway adapter pipeline:
//! adapter registration, start/stop, and event handling.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;

use macaca_gateway::{
    DefaultEventHandler, DiscordAdapter, EventHandler, Gateway, ImAdapter, TelegramAdapter,
};
use macaca_proto::config::{DiscordConfig, TelegramConfig};
use macaca_proto::{GatewayEvent, MacacaResult};

// ---------------------------------------------------------------------------
// Counting event handler
// ---------------------------------------------------------------------------

struct CountingEventHandler {
    count: AtomicUsize,
}

impl CountingEventHandler {
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
impl EventHandler for CountingEventHandler {
    async fn handle(&self, _event: GatewayEvent) -> MacacaResult<()> {
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Register both Telegram and Discord adapters, start all, stop all.
#[tokio::test]
async fn register_and_start_stop_adapters() {
    let handler: Arc<dyn EventHandler> = Arc::new(DefaultEventHandler);
    let mut gateway = Gateway::new(handler);

    gateway.register_adapter(Box::new(TelegramAdapter::new(telegram_config())));
    gateway.register_adapter(Box::new(DiscordAdapter::new(discord_config())));
    assert_eq!(gateway.adapter_count(), 2);

    gateway.start_all().await.unwrap();
    gateway.stop_all().await.unwrap();
}

/// CountingEventHandler tracks events dispatched through it.
#[tokio::test]
async fn counting_handler_tracks_multiple_event_types() {
    let handler = Arc::new(CountingEventHandler::new());
    assert_eq!(handler.event_count(), 0);

    // Simulate several event types
    handler
        .handle(GatewayEvent::TaskRequest {
            user_id: "u1".into(),
            channel_id: "c1".into(),
            content: "build app".into(),
        })
        .await
        .unwrap();
    assert_eq!(handler.event_count(), 1);

    handler
        .handle(GatewayEvent::StatusQuery {
            user_id: "u2".into(),
            channel_id: "c2".into(),
            task_id: None,
        })
        .await
        .unwrap();
    assert_eq!(handler.event_count(), 2);

    handler
        .handle(GatewayEvent::UserReply {
            user_id: "u3".into(),
            channel_id: "c3".into(),
            content: "looks good".into(),
            context_id: "ctx1".into(),
        })
        .await
        .unwrap();
    assert_eq!(handler.event_count(), 3);

    handler
        .handle(GatewayEvent::Command {
            user_id: "u4".into(),
            channel_id: "c4".into(),
            command: "help".into(),
            args: vec!["--verbose".into()],
        })
        .await
        .unwrap();
    assert_eq!(handler.event_count(), 4);
}

/// Gateway with counting handler: start adapters, fire events, stop.
#[tokio::test]
async fn gateway_with_counting_handler_lifecycle() {
    let handler = Arc::new(CountingEventHandler::new());
    let mut gateway = Gateway::new(handler.clone() as Arc<dyn EventHandler>);

    gateway.register_adapter(Box::new(TelegramAdapter::new(telegram_config())));
    gateway.register_adapter(Box::new(DiscordAdapter::new(discord_config())));

    gateway.start_all().await.unwrap();

    // Simulate events through the handler
    handler
        .handle(GatewayEvent::TaskRequest {
            user_id: "u1".into(),
            channel_id: "c1".into(),
            content: "hello".into(),
        })
        .await
        .unwrap();
    handler
        .handle(GatewayEvent::Command {
            user_id: "u2".into(),
            channel_id: "c2".into(),
            command: "status".into(),
            args: vec![],
        })
        .await
        .unwrap();

    assert_eq!(handler.event_count(), 2);

    gateway.stop_all().await.unwrap();
}

/// Empty gateway starts and stops without error.
#[tokio::test]
async fn empty_gateway_start_stop() {
    let handler: Arc<dyn EventHandler> = Arc::new(DefaultEventHandler);
    let gateway = Gateway::new(handler);
    gateway.start_all().await.unwrap();
    gateway.stop_all().await.unwrap();
}

/// Adapter send_message works on stub adapters.
#[tokio::test]
async fn adapter_send_message() {
    let telegram = TelegramAdapter::new(telegram_config());
    telegram
        .send_message("chat_123", "Hello from integration test")
        .await
        .unwrap();

    let discord = DiscordAdapter::new(discord_config());
    discord
        .send_message("channel_456", "Hello from integration test")
        .await
        .unwrap();
}
