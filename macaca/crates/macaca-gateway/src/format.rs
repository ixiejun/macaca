//! Gateway reply formatting strategies.

use crate::message::GatewayReply;
use crate::telegram_format::{split_message, TELEGRAM_MAX_LEN};

/// Strategy for formatting gateway replies before transport sending.
pub trait GatewayReplyFormatter: Send + Sync {
    /// Format a reply into platform-specific message chunks.
    fn format(&self, reply: &GatewayReply) -> Vec<String>;
}

/// Plain text formatter that leaves content unchanged.
pub struct PlainTextFormatter;

impl GatewayReplyFormatter for PlainTextFormatter {
    fn format(&self, reply: &GatewayReply) -> Vec<String> {
        vec![reply.content.clone()]
    }
}

/// Telegram formatter preserving the existing split behavior.
pub struct TelegramFormatter;

impl GatewayReplyFormatter for TelegramFormatter {
    fn format(&self, reply: &GatewayReply) -> Vec<String> {
        split_message(&reply.content, TELEGRAM_MAX_LEN)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_formatter_keeps_short_reply() {
        let reply = GatewayReply::text("discord", "c1", "hello");
        let chunks = PlainTextFormatter.format(&reply);
        assert_eq!(chunks, vec!["hello"]);
    }

    #[test]
    fn telegram_formatter_splits_at_newline() {
        let reply = GatewayReply::text(
            "telegram",
            "c1",
            format!("{}\n{}", "a".repeat(4000), "b".repeat(200)),
        );
        let chunks = TelegramFormatter.format(&reply);
        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].ends_with('\n'));
        assert!(chunks[1].starts_with('b'));
    }

    #[test]
    fn telegram_formatter_hard_splits_without_newline() {
        let reply = GatewayReply::text("telegram", "c1", "x".repeat(200));
        let chunks = split_message(&reply.content, 100);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].len(), 100);
        assert_eq!(chunks[1].len(), 100);
    }
}
