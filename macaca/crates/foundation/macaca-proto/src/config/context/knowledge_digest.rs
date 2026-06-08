//! Knowledge digest provider and digest-vs-raw merge tunables.

use serde::{Deserialize, Serialize};

/// Controls [`macaca_context::KnowledgeDigestContextProvider`] **and** digest-vs-raw suppression in
/// [`macaca_context::ContextFacade::assemble_model_context`].
///
/// The memory governance layer does the heavy lifting (claims, tombstones, redaction). This struct
/// only tunes how compiled digests enter the composer and how they interact with raw vector recall
/// candidates (Strategy pattern — tunable thresholds without altering memory fabric code).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeDigestContextConfig {
    /// Enables the provider family and the post-provider digest-vs-raw merge pass.
    #[serde(default)]
    pub enabled: bool,
    /// Wall-clock ceiling for `KnowledgeDigestCapability::digest_for_request` inside the provider.
    #[serde(default = "default_knowledge_digest_timeout_ms")]
    pub timeout_ms: u64,
    /// Minimum [`KnowledgeClaim::confidence`] scalar for a digest to participate in **suppression**
    /// of overlapping raw recall (0..=1 domain matching memory compiler conventions).
    #[serde(default = "default_knowledge_digest_min_confidence")]
    pub min_confidence_for_suppression: f32,
    /// Minimum [`KnowledgeClaim::freshness`] for a digest to be classified as **strong** (not stale).
    #[serde(default = "default_knowledge_digest_min_freshness_strong")]
    pub min_freshness_strong: f32,
    /// Freshness at or below this cutoff is **stale**: such digests never suppress fresh raw recall.
    #[serde(default = "default_knowledge_digest_stale_cutoff")]
    pub stale_freshness_cutoff: f32,
    /// Minimum number of evidence rows referenced by a claim before suppression logic activates.
    #[serde(default = "default_knowledge_digest_min_evidence")]
    pub min_evidence_count: usize,
    /// Result row ceiling forwarded to the workspace adapter when synthesizing compiler inputs.
    #[serde(default = "default_knowledge_digest_max_compiler_rows")]
    pub max_compiler_rows: usize,
}

fn default_knowledge_digest_timeout_ms() -> u64 {
    1_200
}

fn default_knowledge_digest_min_confidence() -> f32 {
    0.55
}

fn default_knowledge_digest_min_freshness_strong() -> f32 {
    0.35
}

fn default_knowledge_digest_stale_cutoff() -> f32 {
    0.22
}

fn default_knowledge_digest_min_evidence() -> usize {
    1
}

fn default_knowledge_digest_max_compiler_rows() -> usize {
    24
}

impl Default for KnowledgeDigestContextConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            timeout_ms: default_knowledge_digest_timeout_ms(),
            min_confidence_for_suppression: default_knowledge_digest_min_confidence(),
            min_freshness_strong: default_knowledge_digest_min_freshness_strong(),
            stale_freshness_cutoff: default_knowledge_digest_stale_cutoff(),
            min_evidence_count: default_knowledge_digest_min_evidence(),
            max_compiler_rows: default_knowledge_digest_max_compiler_rows(),
        }
    }
}
