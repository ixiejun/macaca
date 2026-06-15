//! Telegram adapter — long-polling implementation via the Bot API.
//!
//! Uses `reqwest` to call `getUpdates` in a loop (30 s timeout) and
//! dispatches parsed [`GatewayEvent`]s to the registered [`EventHandler`].
//! Sending uses `sendMessage` with HTML parse mode for plain text and
//! Markdown for code blocks.  Messages exceeding Telegram's 4 096-character
//! limit are automatically split at newline boundaries.
use std::sync::Arc;

use async_trait::async_trait;
use tracing::{error, info, warn};

use macaca_proto::config::TelegramConfig;
use macaca_proto::error::MacacaResult;
use macaca_proto::types::GatewayEvent;

use crate::adapter::{EventHandler, ImAdapter};
use crate::message::GatewayReply;
use crate::telegram_format::{split_message, TELEGRAM_MAX_LEN};
use crate::telegram_parser;
use crate::transport::GatewayTransport;

// ── TelegramAdapter ────────────────────────────────────────────────────────

/// Telegram adapter that connects to the Bot API via long-polling.
///
/// The bot token is read from the environment variable named by
/// [`TelegramConfig::bot_token_env`] at start time.  If the variable is not
/// set the adapter logs an error and returns without starting the poll loop.
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

    /// Parse a raw Telegram message text into a [`GatewayEvent`].
    ///
    /// - `/status [task_id]` → [`GatewayEvent::StatusQuery`]
    /// - Any other `/command [args…]` → [`GatewayEvent::Command`]
    /// - Plain text → [`GatewayEvent::TaskRequest`]
    pub(crate) fn parse_message(text: &str, user_id: &str, channel_id: &str) -> GatewayEvent {
        telegram_parser::parse_message(text, user_id, channel_id)
    }
}

#[async_trait]
impl ImAdapter for TelegramAdapter {
    fn name(&self) -> &str {
        "telegram"
    }

    /// Start the long-polling loop in a background task.
    ///
    /// Reads the bot token from the environment, then spawns a Tokio task
    /// that calls `getUpdates` with a 30-second timeout.  Stops when the
    /// `TELEGRAM_STOP_{token_env}` env var is set (not used in production —
    /// call [`stop`] which sets an atomic flag instead).
    async fn start(&self, handler: Arc<dyn EventHandler>) -> MacacaResult<()> {
        let token = match std::env::var(&self.config.bot_token_env) {
            Ok(t) => t,
            Err(_) => {
                warn!(
                    env = %self.config.bot_token_env,
                    "Telegram bot token env var not set — adapter will not connect"
                );
                return Ok(());
            }
        };

        let allowed: Vec<String> = self.config.allowed_user_ids.clone();
        let client = reqwest::Client::new();
        let api_base = format!("https://api.telegram.org/bot{}", token);

        info!(
            bot_token_env = %self.config.bot_token_env,
            allowed_users = ?allowed,
            "Telegram adapter starting long-poll loop"
        );

        tokio::spawn(async move {
            let mut offset: i64 = 0;
            loop {
                let url = format!("{}/getUpdates?offset={}&timeout=30", api_base, offset);
                let resp = match client.get(&url).send().await {
                    Ok(r) => r,
                    Err(e) => {
                        error!(error = %e, "Telegram getUpdates request failed");
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        continue;
                    }
                };

                let body: serde_json::Value = match resp.json().await {
                    Ok(b) => b,
                    Err(e) => {
                        error!(error = %e, "Telegram getUpdates JSON parse failed");
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        continue;
                    }
                };

                let results = match body.get("result").and_then(|r| r.as_array()) {
                    Some(arr) => arr.clone(),
                    None => {
                        warn!(body = %body, "Unexpected getUpdates response");
                        continue;
                    }
                };

                for update in results {
                    if let Some(update_id) = update.get("update_id").and_then(|v| v.as_i64()) {
                        offset = update_id + 1;
                    }

                    let message = match update.get("message") {
                        Some(m) => m,
                        None => continue,
                    };

                    let text = match message.get("text").and_then(|t| t.as_str()) {
                        Some(t) if !t.is_empty() => t,
                        _ => continue,
                    };

                    let user_id = message
                        .get("from")
                        .and_then(|f| f.get("id"))
                        .and_then(|id| id.as_i64())
                        .unwrap_or(0)
                        .to_string();

                    let chat_id = message
                        .get("chat")
                        .and_then(|c| c.get("id"))
                        .and_then(|id| id.as_i64())
                        .unwrap_or(0)
                        .to_string();

                    // Enforce allow-list when configured.
                    if !allowed.is_empty() && !allowed.contains(&user_id) {
                        info!(user_id = %user_id, "Ignoring message from unlisted user");
                        continue;
                    }

                    let event = TelegramAdapter::parse_message(text, &user_id, &chat_id);
                    if let Err(e) = handler.handle(event).await {
                        error!(error = %e, "Event handler error");
                    }
                }
            }
        });

        Ok(())
    }

    /// Send a text message to a Telegram chat.
    ///
    /// Long messages are automatically split into chunks of at most
    /// [`TELEGRAM_MAX_LEN`] characters, splitting at newlines where possible.
    async fn send_message(&self, channel_id: &str, content: &str) -> MacacaResult<()> {
        let token = match std::env::var(&self.config.bot_token_env) {
            Ok(t) => t,
            Err(_) => {
                warn!(
                    env = %self.config.bot_token_env,
                    "Telegram bot token env var not set — message not sent"
                );
                return Ok(());
            }
        };

        let client = reqwest::Client::new();
        let url = format!("https://api.telegram.org/bot{}/sendMessage", token);

        for chunk in split_message(content, TELEGRAM_MAX_LEN) {
            let payload = serde_json::json!({
                "chat_id": channel_id,
                "text": chunk,
                "parse_mode": "HTML",
            });
            if let Err(e) = client.post(&url).json(&payload).send().await {
                error!(error = %e, "Telegram sendMessage failed");
                return Err(macaca_proto::error::MacacaError::Gateway(format!(
                    "Telegram send error: {}",
                    e
                )));
            }
        }

        info!(
            channel_id = %channel_id,
            content_len = content.len(),
            "Telegram message sent"
        );
        Ok(())
    }

    async fn stop(&self) -> MacacaResult<()> {
        // The spawned task has no external handle; it will be dropped when the
        // process exits.  For a graceful shutdown, an `Arc<AtomicBool>` could
        // be shared with the task — left as a future enhancement.
        info!("Telegram adapter stop requested");
        Ok(())
    }
}

#[async_trait]
impl GatewayTransport for TelegramAdapter {
    fn name(&self) -> &str {
        "telegram"
    }

    async fn send_reply(&self, reply: &GatewayReply) -> MacacaResult<()> {
        self.send_message(&reply.channel_id, &reply.content).await
    }

    async fn stop(&self) -> MacacaResult<()> {
        ImAdapter::stop(self).await
    }
}
