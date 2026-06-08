//! Contract tests for memory recall governance and prompt-hash stability.
//!
//! Extracted from `memory.rs` so provider-neutral memory DTOs stay under the OS
//! 500-line constitution while provenance and privacy metadata remain guarded.

use super::*;
use crate::report::ContextSourceKind;

#[test]
fn confidence_score_is_bounded() {
    assert_eq!(ConfidenceScore::new(120).value(), 100);
    assert_eq!(ConfidenceScore::new(42).value(), 42);
}

#[test]
fn recall_item_becomes_untrusted_dynamic_candidate() {
    let item = MemoryRecallItem {
        source: memory_source("provider", "memory-1", "fact"),
        text: "remembered fact".into(),
        provenance: ContextSourceProvenance {
            provider_id: "provider".into(),
            source_id: "memory-1".into(),
            evidence: vec!["turn/1".into()],
        },
        confidence: ConfidenceScore::new(80),
        privacy_tier: PrivacyTier::Workspace,
    };

    let snippet = item.into_untrusted_snippet();
    assert_eq!(snippet.trust_level, TrustLevel::Untrusted);
    assert_eq!(snippet.source.kind, ContextSourceKind::Memory);
}

#[test]
fn wiki_digest_is_separate_source_kind() {
    let source = wiki_digest_source("wiki", "claim-1", "claim");
    assert_eq!(source.kind, ContextSourceKind::WikiDigest);
}

#[test]
fn knowledge_digest_report_preserves_governance_metadata() {
    let item = KnowledgeDigestItem {
        claim_id: "claim-1".into(),
        label: "compiled-knowledge".into(),
        statement: "deploy only after health checks pass".into(),
        provenance: ContextSourceProvenance {
            provider_id: "workspace-knowledge-digest".into(),
            source_id: "claim-1".into(),
            evidence: vec!["memory-a".into(), "memory-b".into()],
        },
        evidence_memory_ids: vec!["memory-a".into(), "memory-b".into()],
        confidence: 0.87,
        freshness: 0.92,
        privacy_tier: PrivacyTier::Workspace,
        request_only: true,
        redacted: true,
    };

    let report = item.to_report("[Governed knowledge digest]");
    assert_eq!(
        report.provenance_provider_id.as_deref(),
        Some("workspace-knowledge-digest")
    );
    assert_eq!(report.provenance_source_id.as_deref(), Some("claim-1"));
    assert_eq!(report.confidence_score, Some(87));
    assert_eq!(report.privacy_tier.as_deref(), Some("workspace"));
    assert_eq!(report.request_only, Some(true));
    assert_eq!(report.trust_level.as_deref(), Some("untrusted"));
}

#[test]
fn recall_item_report_preserves_provenance_and_privacy_metadata() {
    let item = MemoryRecallItem {
        source: memory_source("provider", "memory-1", "fact"),
        text: "remembered fact".into(),
        provenance: ContextSourceProvenance {
            provider_id: "provider".into(),
            source_id: "memory-1".into(),
            evidence: vec!["turn/1".into()],
        },
        confidence: ConfidenceScore::new(87),
        privacy_tier: PrivacyTier::UserPrivate,
    };

    let report = item.to_report();
    assert_eq!(report.provenance_provider_id.as_deref(), Some("provider"));
    assert_eq!(report.provenance_source_id.as_deref(), Some("memory-1"));
    assert_eq!(report.confidence_score, Some(87));
    assert_eq!(report.privacy_tier.as_deref(), Some("user_private"));
    assert_eq!(report.request_only, Some(true));
    assert_eq!(report.trust_level.as_deref(), Some("untrusted"));
}

#[test]
fn recall_dynamic_section_does_not_change_stable_hash() {
    let stable = crate::PromptSection::builder("core", ContextSourceKind::SystemPrompt)
        .stable()
        .trusted()
        .content("stable core")
        .build();
    let recall_a = crate::PromptSection::builder("memory", ContextSourceKind::Memory)
        .dynamic()
        .untrusted()
        .content("fact A")
        .build();
    let recall_b = crate::PromptSection::builder("memory", ContextSourceKind::Memory)
        .dynamic()
        .untrusted()
        .content("fact B")
        .build();

    let a = crate::PromptComposer::new()
        .push_section(stable.clone())
        .push_section(recall_a)
        .compile();
    let b = crate::PromptComposer::new()
        .push_section(stable)
        .push_section(recall_b)
        .compile();

    assert_eq!(a.stable_hash, b.stable_hash);
    assert_ne!(a.full_hash, b.full_hash);
}
