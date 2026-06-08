//! Token estimation helpers for compression threshold checks.

use crate::message::Msg;

// ---------------------------------------------------------------------------
// Token estimation
// ---------------------------------------------------------------------------

/// Rough token estimate: each char is ~0.375 tokens for mixed content.
pub fn estimate_tokens(text: &str) -> usize {
    let count = text.chars().count();
    (count * 3 + 7) / 8 // ceiling division equivalent of count * 3/8
}

/// Estimate tokens for a list of messages.
pub fn estimate_messages_tokens(msgs: &[Msg]) -> usize {
    msgs.iter().map(|m| estimate_tokens(&m.get_text())).sum()
}
