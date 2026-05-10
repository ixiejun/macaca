//! Telegram formatting helpers.

/// Telegram Bot API message length limit.
pub(crate) const TELEGRAM_MAX_LEN: usize = 4096;

/// Split `text` into chunks of at most `max_len` characters.
///
/// Prefers splitting at newline boundaries to preserve formatting. Falls
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

        let window = &remaining[..max_len];
        let split_at = window.rfind('\n').map(|i| i + 1).unwrap_or(max_len);

        chunks.push(remaining[..split_at].to_string());
        remaining = &remaining[split_at..];
    }

    chunks
}
