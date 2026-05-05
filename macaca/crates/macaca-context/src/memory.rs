//! Memory and wiki context source provider contracts.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::prompt::TrustLevel;
use crate::report::ContextSourceKind;
use crate::source::{ContextSnippet, ContextSourceReference};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyTier {
    Public,
    Workspace,
    UserPrivate,
    Secret,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConfidenceScore(u8);

impl ConfidenceScore {
    pub fn new(score: u8) -> Self {
        Self(score.min(100))
    }

    pub fn value(self) -> u8 {
        self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextSourceProvenance {
    pub provider_id: String,
    pub source_id: String,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryRecallQuery {
    pub query: String,
    pub session_id: Option<String>,
    pub max_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryRecallItem {
    pub source: ContextSourceReference,
    pub text: String,
    pub provenance: ContextSourceProvenance,
    pub confidence: ConfidenceScore,
    pub privacy_tier: PrivacyTier,
}

impl MemoryRecallItem {
    pub fn into_untrusted_snippet(self) -> ContextSnippet {
        let estimated_tokens = crate::estimate::estimate_text_tokens(&self.text);
        ContextSnippet {
            source: self.source,
            byte_size: self.text.len(),
            text: self.text,
            estimated_tokens,
            pruned_tokens: 0,
            trust_level: TrustLevel::Untrusted,
            mode: crate::source::ContextRenderMode::Full,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryProviderHookInput {
    pub session_id: String,
    pub bounded_context_summary: String,
}

#[async_trait]
pub trait MemorySourceProvider: Send + Sync {
    fn provider_id(&self) -> &str;

    async fn recall(
        &self,
        query: MemoryRecallQuery,
    ) -> macaca_proto::MacacaResult<Vec<MemoryRecallItem>>;

    async fn sync_turn(&self, _input: MemoryProviderHookInput) -> macaca_proto::MacacaResult<()> {
        Ok(())
    }

    async fn before_compaction(
        &self,
        _input: MemoryProviderHookInput,
    ) -> macaca_proto::MacacaResult<()> {
        Ok(())
    }

    async fn session_switched(
        &self,
        _input: MemoryProviderHookInput,
    ) -> macaca_proto::MacacaResult<()> {
        Ok(())
    }
}

#[async_trait]
pub trait WikiDigestSourceProvider: Send + Sync {
    fn provider_id(&self) -> &str;

    async fn digest(
        &self,
        query: MemoryRecallQuery,
    ) -> macaca_proto::MacacaResult<Vec<MemoryRecallItem>>;
}

pub fn memory_source(
    provider_id: impl Into<String>,
    source_id: impl Into<String>,
    label: impl Into<String>,
) -> ContextSourceReference {
    ContextSourceReference::new(
        source_id,
        ContextSourceKind::Memory,
        format!("memory:{}", label.into()),
    )
    .artifact_ref(provider_id.into())
}

pub fn wiki_digest_source(
    provider_id: impl Into<String>,
    source_id: impl Into<String>,
    label: impl Into<String>,
) -> ContextSourceReference {
    ContextSourceReference::new(
        source_id,
        ContextSourceKind::WikiDigest,
        format!("wiki:{}", label.into()),
    )
    .artifact_ref(provider_id.into())
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
