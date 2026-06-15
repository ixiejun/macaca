//! `aos-gateway` — IM gateway adapter layer for Agent OS.
//!
//! Provides a pluggable architecture for connecting instant-messaging
//! platforms (Telegram, Discord, etc.) to the Agent OS kernel. Each
//! platform implements adapter and transport boundaries. [`GatewayBuilder`]
//! owns lifecycle assembly, while [`GatewayMediator`] owns inbound dispatch.
//!
//! # Architecture
//!
//! ```text
//! ┌───────────┐     ┌────────────────┐     ┌──────────────────┐
//! │ Telegram  │────▶│ GatewayBuilder │────▶│ GatewayMediator  │
//! └───────────┘     └────────────────┘     └──────────────────┘
//! ┌───────────┐              │                       │
//! │  Discord  │──────────────┘                       ▼
//! └───────────┘                              GatewayEventSink
//! ```

pub mod adapter;
pub mod builder;
pub mod discord;
pub mod format;
pub mod gateway;
pub mod mediator;
pub mod message;
pub mod service_adapter;
pub mod telegram;
pub(crate) mod telegram_format;
pub(crate) mod telegram_parser;
#[cfg(test)]
mod telegram_tests;
pub mod transport;

pub use adapter::{EventHandler, ImAdapter};
pub use builder::{GatewayBuilder, RunningGateway};
pub use discord::DiscordAdapter;
pub use format::{GatewayReplyFormatter, PlainTextFormatter, TelegramFormatter};
pub use gateway::{DefaultEventHandler, Gateway};
pub use mediator::{GatewayEventSink, GatewayMediator};
pub use message::{GatewayInboundMessage, GatewayOutboundMessage, GatewayReply};
pub use service_adapter::gateway_service_descriptor;
pub use telegram::TelegramAdapter;
pub use transport::GatewayTransport;
