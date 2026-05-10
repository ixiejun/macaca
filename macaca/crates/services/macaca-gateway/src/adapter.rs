//! Core IM adapter and event handler traits.
#![allow(deprecated)]

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::error::MacacaResult;
use macaca_proto::types::GatewayEvent;

/// Pluggable IM adapter trait.
///
/// Each adapter represents a connection to an instant-messaging platform
/// (Telegram, Discord, etc.). The gateway manages the lifecycle of adapters
/// and dispatches incoming events to an [`EventHandler`].
#[deprecated(note = "use GatewayTransport plus GatewayMediator for new gateway integrations")]
#[async_trait]
pub trait ImAdapter: Send + Sync {
    /// Human-readable adapter name (e.g. "telegram", "discord").
    fn name(&self) -> &str;

    /// Start listening for events and dispatch them to the given handler.
    async fn start(&self, handler: Arc<dyn EventHandler>) -> MacacaResult<()>;

    /// Send a text message to a specific channel.
    async fn send_message(&self, channel_id: &str, content: &str) -> MacacaResult<()>;

    /// Gracefully stop the adapter.
    async fn stop(&self) -> MacacaResult<()>;
}

/// Handles gateway events produced by IM adapters.
#[deprecated(note = "use GatewayEventSink through GatewayMediator for new gateway integrations")]
#[async_trait]
pub trait EventHandler: Send + Sync {
    /// Process a single gateway event.
    async fn handle(&self, event: GatewayEvent) -> MacacaResult<()>;
}
