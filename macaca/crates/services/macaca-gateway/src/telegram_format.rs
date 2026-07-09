//! Telegram formatting helpers.

/// Telegram Bot API message length limit.
pub(crate) const TELEGRAM_MAX_LEN: usize = 4096;

/// Split `text` into chunks of at most `max_len` characters.
///
/// Prefers splitting at newline boundaries to preserve formatting. Delegates to
/// the foundation UTF-8-safe [`macaca_proto::text_sanitize::split_by_chars`]
/// primitive: the previous implementation windowed with a raw byte slice
/// (`&remaining[..max_len]`), which panicked whenever `max_len` landed inside a
/// multi-byte character — i.e. on essentially any Chinese or emoji message that
/// exceeded the Telegram length limit.
pub(crate) fn split_message(text: &str, max_len: usize) -> Vec<String> {
    macaca_proto::text_sanitize::split_by_chars(text, max_len)
}
