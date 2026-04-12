//! Context window management for the agentic loop.
//!
//! Estimates token counts and trims old messages when approaching the model's
//! context limit, while always preserving the system message and recent exchanges.

use macaca_proto::{LlmMessage, LlmRole};

/// Configuration for context window management.
#[derive(Debug, Clone)]
pub struct ContextWindowConfig {
    /// Maximum tokens allowed (default: 120_000 for most models).
    pub max_tokens: usize,
    /// Trigger trimming when usage exceeds this fraction (default: 0.8 = 80%).
    pub trim_threshold: f64,
    /// Number of recent message pairs to always preserve (default: 5).
    pub preserve_recent: usize,
}

impl Default for ContextWindowConfig {
    fn default() -> Self {
        Self {
            max_tokens: 120_000,
            trim_threshold: 0.8,
            preserve_recent: 5,
        }
    }
}

/// Manages the context window by trimming message history when needed.
pub struct ContextWindowManager {
    config: ContextWindowConfig,
}

impl ContextWindowManager {
    pub fn new(config: ContextWindowConfig) -> Self {
        Self { config }
    }

    /// Estimate token count for a list of messages.
    ///
    /// Uses a simple heuristic: ~4 chars per token for ASCII, ~1.5 chars per token for CJK.
    pub fn estimate_tokens(messages: &[LlmMessage]) -> usize {
        messages
            .iter()
            .map(|m| {
                let text = &m.content;
                let cjk_chars = text
                    .chars()
                    .filter(|c| {
                        matches!(*c as u32,
                            0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0xF900..=0xFAFF |
                            0x3000..=0x303F | 0xFF00..=0xFFEF
                        )
                    })
                    .count();
                let total_chars = text.chars().count();
                let ascii_chars = total_chars.saturating_sub(cjk_chars);

                let token_estimate = (cjk_chars as f64 / 1.5 + ascii_chars as f64 / 4.0) as usize;

                // Add overhead for role metadata and formatting
                token_estimate + 4
            })
            .sum()
    }

    /// Trim messages if they exceed the threshold.
    ///
    /// Strategy:
    /// 1. Always keep the first message if it is a system message.
    /// 2. Always keep the last `preserve_recent` message pairs.
    /// 3. Replace the middle portion with a summary placeholder.
    ///
    /// Returns the (possibly trimmed) message list.
    pub fn trim_if_needed(&self, messages: Vec<LlmMessage>) -> Vec<LlmMessage> {
        let estimated = Self::estimate_tokens(&messages);
        let threshold = (self.config.max_tokens as f64 * self.config.trim_threshold) as usize;

        if estimated <= threshold {
            return messages;
        }

        if messages.len() <= 3 {
            return messages;
        }

        let mut result = Vec::new();

        // 1. Preserve system message if the first message has System role.
        let start_idx = if messages
            .first()
            .map(|m| m.role == LlmRole::System)
            .unwrap_or(false)
        {
            result.push(messages[0].clone());
            1
        } else {
            0
        };

        // 2. Calculate how many recent messages to keep.
        let recent_count = (self.config.preserve_recent * 2).min(messages.len() - start_idx);
        let recent_start = messages.len() - recent_count;

        // 3. Replace middle portion with a summary placeholder.
        if recent_start > start_idx {
            let trimmed_count = recent_start - start_idx;
            let summary = format!(
                "[{} earlier messages trimmed to manage context window. \
                The conversation continues below with the most recent exchanges.]",
                trimmed_count
            );
            result.push(LlmMessage::system(&summary));
        }

        // 4. Append the recent messages.
        for msg in &messages[recent_start..] {
            result.push(msg.clone());
        }

        result
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::LlmMessage;

    fn make_user(content: &str) -> LlmMessage {
        LlmMessage::user(content)
    }

    fn make_system(content: &str) -> LlmMessage {
        LlmMessage::system(content)
    }

    fn make_assistant(content: &str) -> LlmMessage {
        LlmMessage::assistant(content)
    }

    /// Build a message list that is clearly under the default threshold.
    fn small_messages() -> Vec<LlmMessage> {
        vec![
            make_system("You are a helpful assistant."),
            make_user("Hello!"),
            make_assistant("Hi! How can I help you?"),
        ]
    }

    /// Build a large message list that will exceed the 80 % threshold.
    fn large_messages(n: usize) -> Vec<LlmMessage> {
        let mut msgs = vec![make_system("System prompt.")];
        // Each message is ~500 ASCII chars → ~125 tokens; 800 messages → ~100 000 tokens.
        let long_text = "A".repeat(500);
        for i in 0..n {
            if i % 2 == 0 {
                msgs.push(make_user(&long_text));
            } else {
                msgs.push(make_assistant(&long_text));
            }
        }
        msgs
    }

    #[test]
    fn test_no_trim_under_threshold() {
        let mgr = ContextWindowManager::new(ContextWindowConfig::default());
        let msgs = small_messages();
        let original_len = msgs.len();
        let result = mgr.trim_if_needed(msgs);
        assert_eq!(
            result.len(),
            original_len,
            "Messages under threshold must not be trimmed"
        );
    }

    #[test]
    fn test_trim_preserves_system_message() {
        let mgr = ContextWindowManager::new(ContextWindowConfig::default());
        let msgs = large_messages(800);
        assert!(msgs[0].role == LlmRole::System);
        let result = mgr.trim_if_needed(msgs);
        assert_eq!(
            result[0].role,
            LlmRole::System,
            "First message must remain a system message"
        );
        assert_eq!(result[0].content, "System prompt.");
    }

    #[test]
    fn test_trim_preserves_recent() {
        let config = ContextWindowConfig {
            max_tokens: 120_000,
            trim_threshold: 0.8,
            preserve_recent: 5,
        };
        let mgr = ContextWindowManager::new(config);
        let msgs = large_messages(800);
        let total = msgs.len();
        // The last 10 messages (5 pairs) should always be present.
        let last_10: Vec<_> = msgs[total - 10..]
            .iter()
            .map(|m| m.content.clone())
            .collect();

        let result = mgr.trim_if_needed(msgs);

        let result_contents: Vec<_> = result.iter().map(|m| m.content.clone()).collect();
        for expected in &last_10 {
            assert!(
                result_contents.contains(expected),
                "Recent message not found in trimmed result"
            );
        }
    }

    #[test]
    fn test_estimate_tokens_ascii() {
        // 400 ASCII chars → ~100 tokens + 4 overhead = 104
        let msgs = vec![make_user(&"A".repeat(400))];
        let tokens = ContextWindowManager::estimate_tokens(&msgs);
        assert!(
            tokens >= 100 && tokens <= 110,
            "Expected ~104 tokens for 400 ASCII chars, got {}",
            tokens
        );
    }

    #[test]
    fn test_estimate_tokens_cjk() {
        // 300 CJK chars → 300/1.5 = 200 tokens + 4 overhead = 204
        let chinese_text: String = std::iter::repeat('中').take(300).collect();
        let msgs = vec![make_user(&chinese_text)];
        let tokens = ContextWindowManager::estimate_tokens(&msgs);
        assert!(
            tokens >= 200 && tokens <= 210,
            "Expected ~204 tokens for 300 CJK chars, got {}",
            tokens
        );
    }

    #[test]
    fn test_trim_replaces_middle_with_summary() {
        let mgr = ContextWindowManager::new(ContextWindowConfig::default());
        let msgs = large_messages(800);
        let result = mgr.trim_if_needed(msgs);

        // The second element (index 1) should be the summary placeholder.
        assert!(result.len() >= 2, "Result must have at least 2 messages");
        assert_eq!(result[1].role, LlmRole::System);
        assert!(
            result[1].content.contains("earlier messages trimmed"),
            "Expected trim summary, got: {}",
            result[1].content
        );
    }
}
