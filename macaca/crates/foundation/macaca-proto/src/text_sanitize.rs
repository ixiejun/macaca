//! Provider-neutral string sanitization primitives for the Macaca OS platform.
//!
//! This module is the single source of truth for two observability-safety
//! concerns that the 2026-07-08 constitution audit found scattered and
//! incorrectly implemented across the workspace:
//!
//! 1. **UTF-8-safe truncation.** Many call sites truncated strings with raw byte
//!    slices (`&text[..n]`). When the byte offset `n` falls inside a multi-byte
//!    UTF-8 character — which is common for Chinese text and emoji — slicing a
//!    `&str` panics. Every production truncation MUST route through
//!    [`safe_char_prefix`] / [`truncate_with_marker`], which compute a valid
//!    character boundary and therefore never panic.
//!
//! 2. **Structural secret masking.** Log/trace surfaces previously retained
//!    secret *prefixes* (e.g. the first 12 characters of an `sk-` key) or relied
//!    on keyword deny-lists that let raw secret *values* through. [`mask_secret`]
//!    instead classifies a value by its *shape* (an allow-list philosophy: emit
//!    the value only when it does not look like a credential) and fully redacts
//!    anything secret-shaped, never leaking a usable prefix.
//!
//! Design pattern: **pure strategy functions** with a single source of truth.
//! The functions are deliberately side-effect free so they remain deterministic
//! and unit-testable; the only tracing emitted is a bounded `debug` marker when
//! [`mask_secret`] actually redacts, so operators can observe *that* a secret was
//! masked without seeing its content. Callers own higher-level `info`/`warn`
//! logging at their execution nodes.
//!
//! This module holds no application-specific logic and no hardcoded
//! application/provider/model names; it operates purely on value shape.

/// Marker appended by [`truncate_with_marker`] to make truncation visible.
///
/// The dropped-character count is reported so downstream readers can tell a
/// truncated value apart from a naturally short one.
const TRUNCATION_SUFFIX: &str = "…";

/// Return the prefix of `text` containing at most `max_chars` Unicode scalar
/// values (`char`s), always ending on a valid character boundary.
///
/// This is the UTF-8-safe replacement for `&text[..n]`. It counts *characters*,
/// not bytes, which is both panic-free and the intuitive meaning of "keep the
/// first N". Complexity is O(min(max_chars, len)) because [`str::char_indices`]
/// walks only until the boundary is found.
///
/// # Examples
/// A boundary that would split a multi-byte character is impossible here because
/// the split point is always the byte index *of* a character, never inside one.
pub fn safe_char_prefix(text: &str, max_chars: usize) -> &str {
    // `char_indices().nth(max_chars)` yields the byte offset at which the
    // (max_chars)-th character *starts*; slicing up to that offset keeps exactly
    // `max_chars` characters. If the string has fewer characters, `nth` returns
    // `None` and we return the whole string unchanged.
    match text.char_indices().nth(max_chars) {
        Some((boundary_byte_index, _)) => &text[..boundary_byte_index],
        None => text,
    }
}

/// Truncate `text` to at most `max_chars` characters, appending a visible marker
/// and the number of dropped characters when truncation actually occurs.
///
/// Returns an owned `String` because the marker must be concatenated. When the
/// input already fits, the original text is returned without a marker so
/// non-truncated values stay byte-for-byte identical.
pub fn truncate_with_marker(text: &str, max_chars: usize) -> String {
    // Count characters lazily: we only need to know whether there are more than
    // `max_chars`, so we avoid a full `chars().count()` on very long inputs by
    // taking the prefix first and comparing.
    let prefix = safe_char_prefix(text, max_chars);
    if prefix.len() == text.len() {
        // No truncation happened (prefix is the whole string).
        return text.to_string();
    }
    let dropped = text.chars().count() - prefix.chars().count();
    format!("{prefix}{TRUNCATION_SUFFIX} [truncated {dropped} chars]")
}

/// Split `text` into chunks of at most `max_chars` characters each, preferring a
/// newline boundary within each window to preserve formatting.
///
/// This is the UTF-8-safe replacement for windowed byte slicing used by message
/// transports (e.g. chat gateways with a hard per-message length limit). It never
/// slices inside a multi-byte character.
pub fn split_by_chars(text: &str, max_chars: usize) -> Vec<String> {
    // Guard against a zero window, which would otherwise loop forever.
    // An empty input is treated as a single empty chunk so callers that always
    // expect at least one element (e.g. message transports) keep that contract.
    if max_chars == 0 || text.is_empty() {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        // Take a character-bounded window from the front of the remaining text.
        let window = safe_char_prefix(remaining, max_chars);
        if window.len() == remaining.len() {
            // The remaining text fits entirely within one window.
            chunks.push(remaining.to_string());
            break;
        }

        // Prefer to break at the last newline inside the window so we do not cut
        // a formatted block mid-line. `rfind` returns a byte index that is always
        // a valid boundary because `\n` is single-byte ASCII. `+ 1` keeps the
        // newline attached to the preceding chunk. When no newline exists we fall
        // back to the full character-bounded window length.
        let split_at = window.rfind('\n').map(|i| i + 1).unwrap_or(window.len());

        chunks.push(remaining[..split_at].to_string());
        remaining = &remaining[split_at..];
    }

    chunks
}

/// Fully mask a value that has the *shape* of a credential, returning the safe
/// rendering to place into logs, traces, errors, or snapshots.
///
/// Unlike a keyword deny-list (which lets a raw `sk-…` value through when the
/// surrounding key is not named "secret"), this classifier inspects the value
/// itself and redacts it entirely when it looks like a credential — never
/// retaining a usable prefix. Non-secret-shaped values are returned unchanged so
/// ordinary diagnostics remain readable.
///
/// Recognized secret shapes (structural, provider-neutral):
/// - OpenAI-style keys beginning with `sk-`.
/// - HTTP `Bearer ` authorization tokens.
/// - Long, spaceless, high-symbol tokens (length ≥ 32 with no whitespace), which
///   covers most opaque API keys, JWTs, and signatures.
pub fn mask_secret(value: &str) -> String {
    if is_secret_shaped(value) {
        // Emit a bounded, content-free marker so operators can observe that a
        // redaction happened at this node without seeing the secret.
        tracing::debug!(
            target = "macaca_proto::text_sanitize",
            event = "secret_masked",
            reason_code = "secret_shaped_value"
        );
        return "[redacted-secret]".to_string();
    }
    value.to_string()
}

/// Return whether `value` has the structural shape of a credential.
///
/// Kept public so gates and callers can share the exact classification rule
/// rather than re-deriving heuristics that could drift out of alignment.
pub fn is_secret_shaped(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }
    // Vendor-neutral opaque-key prefix used by several providers.
    if trimmed.starts_with("sk-") {
        return true;
    }
    // HTTP bearer authorization header value.
    if trimmed.starts_with("Bearer ") || trimmed.starts_with("bearer ") {
        return true;
    }
    // Long, spaceless tokens are almost always credentials/JWTs/signatures.
    if trimmed.len() >= 32 && !trimmed.chars().any(|c| c.is_whitespace()) {
        // Require a credential-like alphabet (alphanumeric plus the small symbol
        // set common to base64url/JWT) so ordinary long identifiers with spaces
        // or prose are not falsely masked.
        let looks_tokenish = trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '+' | '/' | '='));
        if looks_tokenish {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_char_prefix_never_splits_multibyte() {
        // Mixed Chinese + emoji: a byte slice at an arbitrary offset would panic.
        let text = "任务执行🚀完成报告";
        // Keep the first 4 characters; the result must be valid UTF-8.
        let prefix = safe_char_prefix(text, 4);
        assert_eq!(prefix, "任务执行");
        // Requesting more characters than exist returns the whole string.
        assert_eq!(safe_char_prefix(text, 999), text);
        // Zero characters yields an empty string, not a panic.
        assert_eq!(safe_char_prefix(text, 0), "");
    }

    #[test]
    fn truncate_with_marker_reports_dropped_chars() {
        let text = "中文内容非常长需要截断";
        let out = truncate_with_marker(text, 4);
        assert!(out.starts_with("中文内容"));
        assert!(out.contains("truncated"));
        // A value that fits is returned unchanged.
        assert_eq!(truncate_with_marker("短", 4), "短");
    }

    #[test]
    fn split_by_chars_is_boundary_safe() {
        let text = "第一行\n第二行内容\n第三行";
        let chunks = split_by_chars(text, 5);
        // Reassembling the chunks must reproduce the original exactly.
        assert_eq!(chunks.concat(), text);
        // No chunk exceeds the character window.
        assert!(chunks.iter().all(|c| c.chars().count() <= 5));
    }

    #[test]
    fn mask_secret_fully_redacts_secret_shapes() {
        assert_eq!(mask_secret("sk-abc1234567890"), "[redacted-secret]");
        assert_eq!(
            mask_secret("Bearer abcdef.token.value"),
            "[redacted-secret]"
        );
        // A 32+ char opaque token is masked.
        assert_eq!(
            mask_secret("AKIA1234567890ABCDEF1234567890ABCD"),
            "[redacted-secret]"
        );
        // Ordinary prose and short values are preserved.
        assert_eq!(mask_secret("normal text"), "normal text");
        assert_eq!(mask_secret("task-123"), "task-123");
    }
}
