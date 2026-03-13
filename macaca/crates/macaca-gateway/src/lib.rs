//! `aos-gateway` — IM gateway adapter layer for Agent OS.
//!
//! Provides a pluggable architecture for connecting instant-messaging
//! platforms (Telegram, Discord, etc.) to the Agent OS kernel. Each
//! platform implements the [`ImAdapter`] trait, and the [`Gateway`]
//! manages the lifecycle of all registered adapters.
//!
//! # Architecture
//!
//! ```text
//! ┌───────────┐     ┌───────────┐     ┌──────────────┐
//! │ Telegram  │────▶│  Gateway  │────▶│ EventHandler │
//! └───────────┘     │           │     └──────────────┘
//! ┌───────────┐     │           │
//! │  Discord  │────▶│           │
//! └───────────┘     └───────────┘
//! ```

pub mod adapter;
pub mod discord;
pub mod gateway;
pub mod telegram;

pub use adapter::{EventHandler, ImAdapter};
pub use discord::DiscordAdapter;
pub use gateway::{DefaultEventHandler, Gateway};
pub use telegram::TelegramAdapter;
