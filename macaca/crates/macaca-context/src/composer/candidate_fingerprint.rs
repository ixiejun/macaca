//! SHA-256 fingerprints over composer **selected** candidates, split stable vs dynamic.
//!
//! Stable bucket uses only rows whose [`ContextCacheClass::effective_cache_class`] resolves to [`ContextCacheClass::Stable`].
//! Changing a dynamic/recall body's text therefore leaves `stable_candidate_fingerprint` untouched when stable rows are identical.

use sha2::{Digest, Sha256};

use super::candidate::{ContextCacheClass, ContextCandidate};

/// Builds `(stable_hex, dynamic_hex)` digests suitable for [`crate::report::ComposerPlanSummary`].
#[must_use]
pub fn fingerprint_selected_candidates(selected: &[ContextCandidate]) -> (String, String) {
    let mut stable_pairs: Vec<(&str, &str)> = Vec::new();
    let mut dynamic_pairs: Vec<(&str, &str)> = Vec::new();
    for c in selected {
        let id = c.source_id.as_str();
        let body = c.content.as_str();
        match c.effective_cache_class() {
            ContextCacheClass::Stable => stable_pairs.push((id, body)),
            ContextCacheClass::Dynamic | ContextCacheClass::Unknown => {
                dynamic_pairs.push((id, body))
            }
        }
    }
    stable_pairs.sort_by(|a, b| a.0.cmp(b.0).then_with(|| a.1.cmp(b.1)));
    dynamic_pairs.sort_by(|a, b| a.0.cmp(b.0).then_with(|| a.1.cmp(b.1)));
    (hash_pairs(&stable_pairs), hash_pairs(&dynamic_pairs))
}

fn hash_pairs(pairs: &[(&str, &str)]) -> String {
    let mut payload = String::new();
    for (id, body) in pairs {
        payload.push_str(id);
        payload.push('\0');
        payload.push_str(body);
        payload.push('\n');
    }
    let mut hasher = Sha256::new();
    hasher.update(payload.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composer::candidate::{ContextCandidateKind, ContextScope, ContextTarget};
    use crate::prompt::TrustLevel;

    fn candidate(id: &str, cache: ContextCacheClass, body: &str) -> ContextCandidate {
        ContextCandidate {
            source_id: id.into(),
            kind: ContextCandidateKind::ProfileBootstrap,
            scope: ContextScope::Agent,
            priority: 50,
            trust: TrustLevel::Trusted,
            cache_class: cache,
            target: ContextTarget::SystemStable,
            content: body.into(),
            token_estimate: 1,
            diagnostics: vec![],
            evidence_memory_ids: vec![],
            digest_strength: None,
            source_report: None,
        }
    }

    #[test]
    fn dynamic_body_change_does_not_move_stable_fingerprint() {
        let a = vec![
            candidate("stable/a", ContextCacheClass::Stable, "alpha"),
            candidate("dyn/r", ContextCacheClass::Dynamic, "recall-v1"),
        ];
        let b = vec![
            candidate("stable/a", ContextCacheClass::Stable, "alpha"),
            candidate("dyn/r", ContextCacheClass::Dynamic, "recall-v2"),
        ];
        let (s1, d1) = fingerprint_selected_candidates(&a);
        let (s2, d2) = fingerprint_selected_candidates(&b);
        assert_eq!(s1, s2);
        assert_ne!(d1, d2);
    }
}
