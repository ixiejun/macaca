//! Sanitization for LLM provider error bodies before they enter observability.
//!
//! Every provider adapter (Anthropic, DashScope, OpenAI, OpenAI-compatible)
//! previously interpolated a raw upstream error body straight into an
//! `MacacaError::Llm` string:
//!
//! ```ignore
//! let text = raw.text().await.unwrap_or_default();
//! return Err(MacacaError::Llm(format!("... API error {status}: {text}")));
//! ```
//!
//! The 2026-07-08 constitution audit (S7) flagged this: the body is unbounded
//! and unredacted, so a provider that echoes the request (including prompt
//! content) or embeds a credential leaks it into logs/traces. This module is the
//! single shared sanitizer all adapters route through, keeping the rule
//! consistent (DRY) and provider-neutral.

/// Maximum number of characters retained from a provider error body.
///
/// Diagnostics need enough to identify the failure class (status text, error
/// code) but must stay bounded so a large echoed payload cannot flood
/// observability surfaces.
const MAX_ERROR_BODY_CHARS: usize = 512;

/// Sanitize a raw provider error body for safe inclusion in an error/log string.
///
/// Operation:
/// 1. Split the body on whitespace and replace any token whose *shape* is a
///    credential (via [`macaca_proto::text_sanitize::mask_secret`]) with a
///    redaction marker. This catches embedded `sk-…`, bearer tokens, and long
///    opaque tokens even when no surrounding keyword names them.
/// 2. Truncate the result to [`MAX_ERROR_BODY_CHARS`] on a UTF-8 character
///    boundary so the output is bounded and never panics on multi-byte content.
///
/// Whitespace is normalized to single spaces, which is acceptable for a
/// diagnostic string and simplifies token-wise masking.
pub(crate) fn sanitize_provider_error_body(body: &str) -> String {
    let masked = body
        .split_whitespace()
        .map(macaca_proto::text_sanitize::mask_secret)
        .collect::<Vec<_>>()
        .join(" ");
    macaca_proto::text_sanitize::truncate_with_marker(&masked, MAX_ERROR_BODY_CHARS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_embedded_secret_and_bounds_length() {
        let body = "error: invalid key sk-abcdef1234567890 for request";
        let sanitized = sanitize_provider_error_body(body);
        assert!(sanitized.contains("[redacted-secret]"));
        assert!(!sanitized.contains("sk-abcdef1234567890"));
    }

    #[test]
    fn bounds_long_body() {
        let body = "x ".repeat(1000);
        let sanitized = sanitize_provider_error_body(&body);
        assert!(sanitized.contains("truncated"));
    }
}
