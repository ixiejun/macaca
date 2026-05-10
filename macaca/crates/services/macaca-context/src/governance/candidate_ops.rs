//! Pure transforms that implement **Decorator** semantics over [`crate::composer::ContextCandidate`].
//!
//! These helpers are side-effect free and unit-testable: they encode redaction, allow/deny
//! routing, and anti-corruption validation without referencing async runtimes or IO.

use crate::composer::ContextCandidate;
use crate::estimate::estimate_text_tokens;

/// Replaces every occurrence of configured `needles` with a neutral placeholder.
///
/// This is a conservative substring strategy suitable for known secret markers configured by
/// operators. It is **not** a general PII detector.
pub fn redact_plain_substrings(content: &str, needles: &[String]) -> String {
    let mut out = content.to_string();
    for needle in needles {
        if needle.is_empty() {
            continue;
        }
        out = out.replace(needle.as_str(), "[REDACTED]");
    }
    out
}

/// Returns `true` when candidate `source_id` must be dropped by prefix policy.
pub fn is_denied_by_prefix(source_id: &str, deny_prefixes: &[String]) -> bool {
    deny_prefixes
        .iter()
        .any(|p| !p.is_empty() && source_id.starts_with(p.as_str()))
}

/// Enforces minimum invariants for any `ContextCandidate` crossing the governance boundary.
///
/// **Anti-corruption:** upstream providers must not emit empty ids or absurdly large blobs that
/// could overwhelm token estimators or the composer builder.
pub fn validate_governed_candidate(c: &ContextCandidate) -> Result<(), String> {
    if c.source_id.trim().is_empty() {
        return Err("provider emitted empty source_id".into());
    }
    const MAX_CONTENT_BYTES: usize = 4 * 1024 * 1024;
    if c.content.len() > MAX_CONTENT_BYTES {
        return Err(format!(
            "candidate {} exceeds {MAX_CONTENT_BYTES} bytes",
            c.source_id
        ));
    }
    Ok(())
}

/// Clones a candidate, replacing `content` with `new_content` and refreshing `token_estimate`.
#[must_use]
pub fn with_rewritten_content(mut c: ContextCandidate, new_content: String) -> ContextCandidate {
    let tokens = estimate_text_tokens(&new_content).max(1);
    let byte_size = new_content.len();
    c.content = new_content;
    c.token_estimate = tokens;
    if let Some(report) = c.source_report.as_mut() {
        report.estimated_tokens = tokens;
        report.byte_size = byte_size;
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composer::{
        ContextCacheClass, ContextCandidate, ContextCandidateKind, ContextScope, ContextTarget,
    };
    use crate::prompt::TrustLevel;

    fn sample_candidate(source_id: &str, content: &str) -> ContextCandidate {
        ContextCandidate {
            source_id: source_id.into(),
            kind: ContextCandidateKind::Custom,
            scope: ContextScope::Request,
            priority: 10,
            trust: TrustLevel::Untrusted,
            cache_class: ContextCacheClass::Dynamic,
            target: ContextTarget::UserSide,
            content: content.into(),
            token_estimate: 1,
            diagnostics: Vec::new(),
            evidence_memory_ids: Vec::new(),
            digest_strength: None,
            source_report: None,
        }
    }

    #[test]
    fn redaction_is_substring_level() {
        let out = redact_plain_substrings("hello SECRET world", &["SECRET".into()]);
        assert_eq!(out, "hello [REDACTED] world");
    }

    #[test]
    fn deny_prefix_matches_starts_with() {
        assert!(is_denied_by_prefix("mcp:foo", &["mcp:".into()]));
        assert!(!is_denied_by_prefix("skill:foo", &["mcp:".into()]));
    }

    #[test]
    fn validate_rejects_empty_source_id() {
        let mut c = sample_candidate("ok", "x");
        c.source_id = "   ".into();
        assert!(validate_governed_candidate(&c).is_err());
    }

    #[test]
    fn rewritten_content_refreshes_token_estimate() {
        let c = sample_candidate("a", "hi");
        let orig_tokens = c.token_estimate;
        let longer = with_rewritten_content(c, "a much longer string for estimation".into());
        assert!(longer.token_estimate >= orig_tokens);
    }
}
