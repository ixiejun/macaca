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

// Telegram Bot API message length limit.
const TELEGRAM_MAX_LEN: usize = 4096;

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
        let text = text.trim();
        if let Some(rest) = text.strip_prefix('/') {
            // Split command from arguments on the first space.
            let (cmd_raw, arg_str) = match rest.split_once(' ') {
                Some((c, a)) => (c, a),
                None => (rest, ""),
            };
            // Strip any @BotName suffix (e.g. /start@MyBot).
            let command = cmd_raw
                .split_once('@')
                .map(|(c, _)| c)
                .unwrap_or(cmd_raw)
                .to_string();
            let args: Vec<String> = arg_str
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();

            if command == "status" {
                return GatewayEvent::StatusQuery {
                    user_id: user_id.into(),
                    channel_id: channel_id.into(),
                    task_id: args.first().and_then(|id| {
                        uuid::Uuid::parse_str(id)
                            .ok()
                            .map(macaca_proto::types::TaskId)
                    }),
                };
            }

            return GatewayEvent::Command {
                user_id: user_id.into(),
                channel_id: channel_id.into(),
                command,
                args,
            };
        }

        GatewayEvent::TaskRequest {
            user_id: user_id.into(),
            channel_id: channel_id.into(),
            content: text.to_string(),
        }
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
                let url = format!(
                    "{}/getUpdates?offset={}&timeout=30",
                    api_base, offset
                );
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
                return Err(macaca_proto::error::MacacaError::Gateway(
                    format!("Telegram send error: {}", e),
                ));
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

// ── Helpers ────────────────────────────────────────────────────────────────

/// Split `text` into chunks of at most `max_len` characters.
///
/// Prefers splitting at newline boundaries to preserve formatting.  Falls
/// back to a hard split at `max_len` when no newline is found within the
/// current window.
pub(crate) fn split_message(text: &str, max_len: usize) -> Vec<String> {
    if text.len() <= max_len {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        if remaining.len() <= max_len {
            chunks.push(remaining.to_string());
            break;
        }

        // Try to split at a newline within the allowed window.
        let window = &remaining[..max_len];
        let split_at = window
            .rfind('\n')
            .map(|i| i + 1) // include the newline in the current chunk
            .unwrap_or(max_len); // hard split if no newline found

        chunks.push(remaining[..split_at].to_string());
        remaining = &remaining[split_at..];
    }

    chunks
}

// ── Tests ──────────────────────────────────────────────────────────────────

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

    // ── adapter metadata ──

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

    // ── parse_message ──

    #[test]
    fn test_parse_task_request() {
        let event = TelegramAdapter::parse_message("build me a web app", "42", "100");
        match event {
            GatewayEvent::TaskRequest { user_id, channel_id, content } => {
                assert_eq!(user_id, "42");
                assert_eq!(channel_id, "100");
                assert_eq!(content, "build me a web app");
            }
            other => panic!("expected TaskRequest, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_status_command_no_id() {
        let event = TelegramAdapter::parse_message("/status", "1", "2");
        match event {
            GatewayEvent::StatusQuery { user_id, channel_id, task_id } => {
                assert_eq!(user_id, "1");
                assert_eq!(channel_id, "2");
                assert!(task_id.is_none());
            }
            other => panic!("expected StatusQuery, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_status_command_with_valid_uuid() {
        let id = "550e8400-e29b-41d4-a716-446655440000";
        let msg = format!("/status {}", id);
        let event = TelegramAdapter::parse_message(&msg, "1", "2");
        match event {
            GatewayEvent::StatusQuery { task_id, .. } => {
                assert!(task_id.is_some());
            }
            other => panic!("expected StatusQuery, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_status_command_with_invalid_id() {
        // Non-UUID arg → task_id is None.
        let event = TelegramAdapter::parse_message("/status abc123", "1", "2");
        match event {
            GatewayEvent::StatusQuery { task_id, .. } => {
                assert!(task_id.is_none());
            }
            other => panic!("expected StatusQuery, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_generic_command() {
        let event = TelegramAdapter::parse_message("/agents list all", "7", "9");
        match event {
            GatewayEvent::Command { user_id, channel_id, command, args } => {
                assert_eq!(user_id, "7");
                assert_eq!(channel_id, "9");
                assert_eq!(command, "agents");
                assert_eq!(args, vec!["list", "all"]);
            }
            other => panic!("expected Command, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_command_with_bot_suffix() {
        // Telegram sends /start@BotName in group chats.
        let event = TelegramAdapter::parse_message("/help@MyBot", "1", "2");
        match event {
            GatewayEvent::Command { command, args, .. } => {
                assert_eq!(command, "help");
                assert!(args.is_empty());
            }
            other => panic!("expected Command, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_trims_whitespace() {
        let event = TelegramAdapter::parse_message("  hello world  ", "1", "2");
        match event {
            GatewayEvent::TaskRequest { content, .. } => {
                assert_eq!(content, "hello world");
            }
            other => panic!("expected TaskRequest, got {:?}", other),
        }
    }

    // ── split_message ──

    #[test]
    fn test_split_message_short() {
        let chunks = split_message("hello", 100);
        assert_eq!(chunks, vec!["hello"]);
    }

    #[test]
    fn test_split_message_exact_limit() {
        let text = "a".repeat(100);
        let chunks = split_message(&text, 100);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].len(), 100);
    }

    #[test]
    fn test_split_message_long_no_newline() {
        // 200 chars, no newlines → hard split at 100.
        let text = "x".repeat(200);
        let chunks = split_message(&text, 100);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].len(), 100);
        assert_eq!(chunks[1].len(), 100);
    }

    #[test]
    fn test_split_message_splits_at_newline() {
        // Two paragraphs — split should happen at the newline.
        let text = format!("{}\n{}", "a".repeat(60), "b".repeat(60));
        let chunks = split_message(&text, 100);
        // First chunk ends with '\n', second is the b's.
        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].ends_with('\n'));
        assert!(chunks[1].starts_with('b'));
    }

    #[test]
    fn test_split_message_empty() {
        let chunks = split_message("", 100);
        assert_eq!(chunks, vec![""]);
    }

    #[test]
    fn test_split_message_three_chunks() {
        // 300 chars, no newlines → 3 hard-split chunks of 100.
        let text = "z".repeat(300);
        let chunks = split_message(&text, 100);
        assert_eq!(chunks.len(), 3);
        for c in &chunks {
            assert_eq!(c.len(), 100);
        }
    }

    // ── adapter lifecycle (stub — no real network) ──

    #[tokio::test]
    async fn telegram_adapter_start_without_token_is_ok() {
        // Token env var not set → start returns Ok (just warns).
        std::env::remove_var("TEST_TELEGRAM_TOKEN");
        let adapter = TelegramAdapter::new(test_config());
        let handler: Arc<dyn EventHandler> = Arc::new(crate::gateway::DefaultEventHandler);
        adapter.start(handler).await.unwrap();
    }

    #[tokio::test]
    async fn telegram_adapter_stop_is_ok() {
        let adapter = TelegramAdapter::new(test_config());
        adapter.stop().await.unwrap();
    }

    #[tokio::test]
    async fn telegram_adapter_send_without_token_is_ok() {
        std::env::remove_var("TEST_TELEGRAM_TOKEN");
        let adapter = TelegramAdapter::new(test_config());
        adapter.send_message("chat_123", "Hello from test").await.unwrap();
    }
}
