//! Provider-neutral context candidates for compiled knowledge.
//!
//! This module intentionally does not depend on `macaca-context`. It exports a
//! small DTO that upper crates can adapt into `ContextSnippet`,
//! `MemoryRecallItem`, or any custom context system. This keeps the memory
//! system pluggable and avoids forcing Macaca's context engine onto users that
//! replace it with their own implementation.

use serde::{Deserialize, Serialize};

use super::compiler::{KnowledgeClaim, KnowledgeCompileResult};

/// Context source kind recommended for a knowledge digest item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnowledgeContextSourceKind {
    WikiDigest,
}

/// One bounded compiled-knowledge item ready for context adaptation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeContextCandidate {
    pub source_kind: KnowledgeContextSourceKind,
    pub source_id: String,
    pub label: String,
    pub text: String,
    pub evidence: Vec<String>,
    pub confidence: f32,
    pub redacted: bool,
}

impl KnowledgeContextCandidate {
    pub fn from_claim(claim: &KnowledgeClaim) -> Self {
        // Evidence strings retain `source_kind:source_id` so downstream adapters can parse or expose
        // reference-only provenance; digest-vs-raw overlap compares `source_id` with recall snippets.
        Self {
            source_kind: KnowledgeContextSourceKind::WikiDigest,
            source_id: claim.id.clone(),
            label: "compiled-knowledge".into(),
            text: claim.statement.clone(),
            evidence: claim
                .evidence
                .iter()
                .map(|evidence| format!("{}:{}", evidence.source_kind, evidence.source_id))
                .collect(),
            confidence: claim.confidence.0,
            redacted: true,
        }
    }
}

/// Convert compiler output into provider-neutral context candidates.
pub fn compiled_digest_candidates(
    result: &KnowledgeCompileResult,
) -> Vec<KnowledgeContextCandidate> {
    result
        .claims
        .iter()
        .filter(|claim| !claim.revoked)
        .map(KnowledgeContextCandidate::from_claim)
        .collect()
}
