//! Combined digest tombstone trimming + digest-vs-raw arbitration smoke tests.

use std::collections::HashSet;

use macaca_proto::config::KnowledgeDigestContextConfig;

use crate::composer::{
    ContextCacheClass, ContextCandidate, ContextCandidateKind, ContextScope, ContextTarget,
    DigestStrengthSnapshot,
};
use crate::memory::{ContextSourceProvenance, KnowledgeDigestItem, PrivacyTier};
use crate::prompt::TrustLevel;
use crate::{
    apply_digest_vs_raw_selection, digest_candidate_id, filter_digest_items_by_tombstones,
    ACTIVE_VECTOR_RECALL_SOURCE_ID,
};

fn digest_candidate(claim: &str, evidence: &[&str], conf: f32, freshness: f32) -> ContextCandidate {
    ContextCandidate {
        source_id: digest_candidate_id(claim),
        kind: ContextCandidateKind::KnowledgeDigest,
        scope: ContextScope::Session,
        priority: 60,
        trust: TrustLevel::Untrusted,
        cache_class: ContextCacheClass::Dynamic,
        target: ContextTarget::UserSide,
        content: claim.into(),
        token_estimate: 4,
        diagnostics: vec![],
        evidence_memory_ids: evidence.iter().map(|s| (*s).to_string()).collect(),
        digest_strength: Some(DigestStrengthSnapshot {
            confidence: conf,
            freshness,
        }),
        source_report: None,
    }
}

fn recall_candidate(evidence: &[&str], body: &str) -> ContextCandidate {
    ContextCandidate {
        source_id: ACTIVE_VECTOR_RECALL_SOURCE_ID.into(),
        kind: ContextCandidateKind::MemoryRecall,
        scope: ContextScope::Session,
        priority: 40,
        trust: TrustLevel::Untrusted,
        cache_class: ContextCacheClass::Dynamic,
        target: ContextTarget::UserSide,
        content: body.into(),
        token_estimate: 10,
        diagnostics: vec![],
        evidence_memory_ids: evidence.iter().map(|s| (*s).to_string()).collect(),
        digest_strength: None,
        source_report: None,
    }
}

#[test]
fn tombstone_trim_then_digest_can_still_cover_raw() {
    let items = vec![
        KnowledgeDigestItem {
            claim_id: "c1".into(),
            label: "l1".into(),
            statement: "rule".into(),
            provenance: ContextSourceProvenance {
                provider_id: "workspace-knowledge-digest".into(),
                source_id: "c1".into(),
                evidence: vec!["m1".into(), "m2".into()],
            },
            evidence_memory_ids: vec!["m1".into(), "m2".into()],
            confidence: 0.9,
            freshness: 0.9,
            privacy_tier: PrivacyTier::Workspace,
            request_only: true,
            redacted: true,
        },
        KnowledgeDigestItem {
            claim_id: "c2".into(),
            label: "l2".into(),
            statement: "noise".into(),
            provenance: ContextSourceProvenance {
                provider_id: "workspace-knowledge-digest".into(),
                source_id: "c2".into(),
                evidence: vec!["m-hidden".into()],
            },
            evidence_memory_ids: vec!["m-hidden".into()],
            confidence: 0.95,
            freshness: 0.95,
            privacy_tier: PrivacyTier::Workspace,
            request_only: true,
            redacted: true,
        },
    ];
    let mut ts: HashSet<String> = HashSet::new();
    ts.insert("m-hidden".into());

    let trimmed = filter_digest_items_by_tombstones(items, &ts);
    assert_eq!(trimmed.len(), 1);
    assert_eq!(trimmed[0].claim_id, "c1");

    let digest = digest_candidate("c1", &["m1", "m2"], 0.9, 0.9);
    let recall = recall_candidate(&["m1", "m2"], "fence");
    let cfg = KnowledgeDigestContextConfig {
        enabled: true,
        ..KnowledgeDigestContextConfig::default()
    };
    let (out, _reports) = apply_digest_vs_raw_selection(vec![digest, recall], &cfg);
    assert!(out
        .iter()
        .all(|c| c.source_id != ACTIVE_VECTOR_RECALL_SOURCE_ID));
}

#[test]
fn stale_digest_keeps_raw_even_after_tombstone_trim() {
    let digest = digest_candidate("old", &["m1"], 0.9, 0.05);
    let recall = recall_candidate(&["m1"], "fence");
    let cfg = KnowledgeDigestContextConfig {
        enabled: true,
        stale_freshness_cutoff: 0.5,
        min_freshness_strong: 0.6,
        ..KnowledgeDigestContextConfig::default()
    };
    let (out, reports) = apply_digest_vs_raw_selection(vec![digest, recall], &cfg);
    assert!(out
        .iter()
        .any(|c| c.source_id == ACTIVE_VECTOR_RECALL_SOURCE_ID));
    assert!(reports
        .iter()
        .any(|r| r.code == "digest_stale_allows_raw_recall"));
}

#[test]
fn structured_digest_report_does_not_serialize_full_sensitive_source_text() {
    let item = KnowledgeDigestItem {
        claim_id: "claim-secret".into(),
        label: "compiled-knowledge".into(),
        statement: "summary only".into(),
        provenance: ContextSourceProvenance {
            provider_id: "workspace-knowledge-digest".into(),
            source_id: "claim-secret".into(),
            evidence: vec!["memory-secret-id".into()],
        },
        evidence_memory_ids: vec!["memory-secret-id".into()],
        confidence: 0.91,
        freshness: 0.88,
        privacy_tier: PrivacyTier::Workspace,
        request_only: true,
        redacted: true,
    };

    let rendered = "[Governed knowledge digest]\nevidence_refs=memory-secret-id\nsummary only";
    let report = item.to_report(rendered);
    let encoded = serde_json::to_string(&report).unwrap();

    assert!(encoded.contains("memory-secret-id"));
    assert!(encoded.contains("workspace-knowledge-digest"));
    assert!(!encoded.contains("full sensitive raw source text"));
}

#[test]
fn redacted_digest_rendering_keeps_evidence_refs() {
    let item = KnowledgeDigestItem {
        claim_id: "claim-redacted".into(),
        label: "compiled-knowledge".into(),
        statement: "redacted summary".into(),
        provenance: ContextSourceProvenance {
            provider_id: "workspace-knowledge-digest".into(),
            source_id: "claim-redacted".into(),
            evidence: vec!["evidence-1".into()],
        },
        evidence_memory_ids: vec!["evidence-1".into()],
        confidence: 0.9,
        freshness: 0.9,
        privacy_tier: PrivacyTier::Workspace,
        request_only: true,
        redacted: true,
    };
    let body = format!(
        "[Governed knowledge digest]\nevidence_refs={}\n{} (redacted upstream; evidence_refs={})",
        item.evidence_memory_ids.join(","),
        item.statement,
        item.evidence_memory_ids.join(",")
    );

    assert!(body.contains("evidence_refs=evidence-1"));
    assert!(!body.contains("raw memory body"));
}
