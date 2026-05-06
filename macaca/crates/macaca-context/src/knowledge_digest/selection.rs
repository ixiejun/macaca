//! Digest-vs-raw **Strategy** for overlapping memory context.
//!
//! Active vector recall injects a single fenced [`ContextCandidate`] with id
//! [`ACTIVE_VECTOR_RECALL_SOURCE_ID`]. Compiled governance digests emit one row per
//! [`ContextCandidateKind::KnowledgeDigest`] claim **before** this recall row in the provider sort
//! order, carrying stable `evidence_memory_ids` copied from [`macaca_memory::governance::ClaimEvidence::source_id`].
//!
//! When enabled, this module removes the raw recall candidate only when **strong** digests provide a
//! superset of evidence keys for the rows surfaced inside the recall fence. Stale digests (low
//! freshness) never participate in suppression, satisfying the OpenSpec “stale digest must not bury
//! fresh recall” requirement.

use std::collections::HashSet;

use macaca_proto::config::KnowledgeDigestContextConfig;

use crate::composer::{ContextCandidate, ContextCandidateKind};
use crate::report::{ContextDecisionReport, ContextDecisionSeverity};

/// `ContextCandidate::source_id` chosen by [`crate::memory_active_recall_provider::MemoryActiveRecallContextProvider`].
pub const ACTIVE_VECTOR_RECALL_SOURCE_ID: &str = "active_vector_memory/recall";

/// Builds a canonical per-claim id used inside composer plans (stable for dedupe / reporting).
#[must_use]
pub fn digest_candidate_id(claim_id: &str) -> String {
    format!("knowledge_digest/claim/{claim_id}")
}

fn digest_is_strong(c: &ContextCandidate, cfg: &KnowledgeDigestContextConfig) -> bool {
    let Some(metrics) = c.digest_strength else {
        return false;
    };
    if c.kind != ContextCandidateKind::KnowledgeDigest {
        return false;
    }
    if metrics.confidence < cfg.min_confidence_for_suppression {
        return false;
    }
    if metrics.freshness <= cfg.stale_freshness_cutoff {
        return false;
    }
    if metrics.freshness < cfg.min_freshness_strong {
        return false;
    }
    if c.evidence_memory_ids.len() < cfg.min_evidence_count {
        return false;
    }
    true
}

/// Applies digest vs. raw recall overlap policy on a flat candidate list (post-provider fan-in).
///
/// Returns a possibly shorter vector plus structured audit records suitable for `ContextReport`.
/// The function is idempotent when [`KnowledgeDigestContextConfig::enabled`] is `false`.
pub fn apply_digest_vs_raw_selection(
    mut candidates: Vec<ContextCandidate>,
    cfg: &KnowledgeDigestContextConfig,
) -> (Vec<ContextCandidate>, Vec<ContextDecisionReport>) {
    if !cfg.enabled {
        return (candidates, Vec::new());
    }

    let mut reports = Vec::new();
    let raw_idx = match candidates
        .iter()
        .position(|c| c.source_id == ACTIVE_VECTOR_RECALL_SOURCE_ID)
    {
        Some(i) => i,
        None => return (candidates, reports),
    };

    let raw_keys: HashSet<String> = candidates[raw_idx]
        .evidence_memory_ids
        .iter()
        .cloned()
        .collect();
    if raw_keys.is_empty() {
        reports.push(ContextDecisionReport {
            code: "digest_vs_raw_skipped".into(),
            severity: ContextDecisionSeverity::Info,
            message:
                "active recall candidate has no evidence_memory_ids — cannot evaluate digest overlap"
                    .into(),
        });
        return (candidates, reports);
    }

    let mut strong_union: HashSet<String> = HashSet::new();
    let mut stale_overlap = false;
    for cand in candidates
        .iter()
        .filter(|c| c.kind == ContextCandidateKind::KnowledgeDigest)
    {
        let overlap = cand
            .evidence_memory_ids
            .iter()
            .any(|id| raw_keys.contains(id));
        if digest_is_strong(cand, cfg) {
            for id in &cand.evidence_memory_ids {
                strong_union.insert(id.clone());
            }
        } else if overlap
            && cand
                .digest_strength
                .is_some_and(|m| m.freshness <= cfg.stale_freshness_cutoff)
        {
            stale_overlap = true;
        }
    }

    let covered = !strong_union.is_empty() && raw_keys.iter().all(|k| strong_union.contains(k));
    if covered {
        let removed = candidates.remove(raw_idx);
        reports.push(ContextDecisionReport {
            code: "digest_covers_raw_recall".into(),
            severity: ContextDecisionSeverity::Info,
            message: format!(
                "removed {}: governed digest evidence superset {:?} covers recall keys {:?}",
                removed.source_id, strong_union, raw_keys
            ),
        });
        return (candidates, reports);
    }

    if stale_overlap {
        reports.push(ContextDecisionReport {
            code: "digest_stale_allows_raw_recall".into(),
            severity: ContextDecisionSeverity::Info,
            message: "stale digest overlaps raw recall keys — retaining fenced recall per freshness policy"
                .into(),
        });
    }

    (candidates, reports)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composer::{ContextCacheClass, ContextScope, ContextTarget};
    use crate::prompt::TrustLevel;

    fn digest_row(id: &str, conf: f32, fresh: f32, evidence: &[&str]) -> ContextCandidate {
        ContextCandidate {
            source_id: digest_candidate_id(id),
            kind: ContextCandidateKind::KnowledgeDigest,
            scope: ContextScope::Session,
            priority: 60,
            trust: TrustLevel::Untrusted,
            cache_class: ContextCacheClass::Dynamic,
            target: ContextTarget::UserSide,
            content: "statement".into(),
            token_estimate: 3,
            diagnostics: vec!["governed_knowledge".into()],
            evidence_memory_ids: evidence.iter().map(|s| (*s).to_string()).collect(),
            digest_strength: Some(crate::DigestStrengthSnapshot {
                confidence: conf,
                freshness: fresh,
            }),
            source_report: None,
        }
    }

    fn recall_row(evidence: &[&str]) -> ContextCandidate {
        ContextCandidate {
            source_id: ACTIVE_VECTOR_RECALL_SOURCE_ID.into(),
            kind: ContextCandidateKind::MemoryRecall,
            scope: ContextScope::Session,
            priority: 40,
            trust: TrustLevel::Untrusted,
            cache_class: ContextCacheClass::Dynamic,
            target: ContextTarget::UserSide,
            content: "fence-body".into(),
            token_estimate: 10,
            diagnostics: vec![],
            evidence_memory_ids: evidence.iter().map(|s| (*s).to_string()).collect(),
            digest_strength: None,
            source_report: None,
        }
    }

    #[test]
    fn strong_digest_suppresses_raw_when_superset() {
        let cfg = KnowledgeDigestContextConfig {
            enabled: true,
            ..KnowledgeDigestContextConfig::default()
        };
        let candidates = vec![
            digest_row("c1", 0.9, 0.9, &["m1", "m2"]),
            recall_row(&["m1", "m2"]),
        ];
        let (out, reports) = apply_digest_vs_raw_selection(candidates, &cfg);
        assert_eq!(out.len(), 1);
        assert!(reports.iter().any(|r| r.code == "digest_covers_raw_recall"));
    }

    #[test]
    fn stale_digest_does_not_remove_raw() {
        let cfg = KnowledgeDigestContextConfig {
            enabled: true,
            stale_freshness_cutoff: 0.5,
            min_freshness_strong: 0.6,
            ..KnowledgeDigestContextConfig::default()
        };
        let candidates = vec![digest_row("c1", 0.9, 0.1, &["m1"]), recall_row(&["m1"])];
        let (out, reports) = apply_digest_vs_raw_selection(candidates, &cfg);
        assert_eq!(out.len(), 2);
        assert!(reports
            .iter()
            .any(|r| r.code == "digest_stale_allows_raw_recall"));
    }

    #[test]
    fn provider_error_evidence_empty_keeps_raw() {
        let cfg = KnowledgeDigestContextConfig {
            enabled: true,
            ..KnowledgeDigestContextConfig::default()
        };
        let candidates = vec![recall_row(&[])];
        let (out, _reports) = apply_digest_vs_raw_selection(candidates, &cfg);
        assert_eq!(out.len(), 1);
    }
}
