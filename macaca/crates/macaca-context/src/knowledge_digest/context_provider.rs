//! [`KnowledgeDigestContextProvider`] — Adapter between [`crate::memory::KnowledgeDigestCapability`]
//! and the generic composer.
//!
//! ## Rendering model
//! The provider emits fenced sections analogous to active vector recall so operators can clearly see
//! **governed** wiki memory separately from raw fenced recall. Content is treated as dynamic /
//! untrusted by default: even curated claims deserve verification before high-impact actions.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use macaca_proto::MacacaResult;
use tokio::time::timeout;

use crate::composer::{
    ContextCacheClass, ContextCandidate, ContextCandidateKind, ContextComposeContext,
    ContextProvider, ContextProviderDiagnostics, ContextProviderOutcome, ContextProviderStage,
    ContextScope, ContextTarget, DigestStrengthSnapshot,
};
use crate::estimate::estimate_text_tokens;
use crate::knowledge_digest::selection::digest_candidate_id;
use crate::memory::KnowledgeDigestCapability;
use crate::prompt::TrustLevel;
use macaca_proto::config::KnowledgeDigestContextConfig;

/// Implements [`ContextProvider`] over a [`KnowledgeDigestCapability`] port (Adapter pattern).
#[derive(Clone)]
pub struct KnowledgeDigestContextProvider {
    inner: Arc<dyn KnowledgeDigestCapability>,
    config: KnowledgeDigestContextConfig,
}

impl KnowledgeDigestContextProvider {
    /// Wraps a port implementation such as the workspace compiler bridge in `macaca-web`.
    #[must_use]
    pub fn new(
        inner: Arc<dyn KnowledgeDigestCapability>,
        config: KnowledgeDigestContextConfig,
    ) -> Self {
        Self { inner, config }
    }
}

fn render_claim_block(row: &crate::memory::KnowledgeDigestItem) -> String {
    let refs = row.evidence_memory_ids.join(",");
    format!(
        "[Governed knowledge digest — claim {}]\nevidence_refs={}\n{}",
        row.claim_id,
        if refs.is_empty() { "none".into() } else { refs },
        row.statement
    )
}

#[async_trait]
impl ContextProvider for KnowledgeDigestContextProvider {
    fn provider_id(&self) -> &str {
        self.inner.capability_id()
    }

    fn stage(&self) -> ContextProviderStage {
        ContextProviderStage::KnowledgeDigest
    }

    async fn contribute(
        &self,
        ctx: &ContextComposeContext<'_>,
    ) -> MacacaResult<ContextProviderOutcome> {
        if !self.config.enabled {
            return Ok(ContextProviderOutcome::default());
        }

        let input = ctx.assemble_input;
        let fut = self.inner.digest_for_request(input);
        let result = timeout(Duration::from_millis(self.config.timeout_ms), fut).await;

        match result {
            Ok(Ok(rows)) => {
                if rows.is_empty() {
                    return Ok(ContextProviderOutcome::default());
                }

                let mut candidates = Vec::new();
                for row in rows {
                    let digest_strength = Some(DigestStrengthSnapshot {
                        confidence: row.confidence,
                        freshness: row.freshness,
                    });
                    let body = render_claim_block(&row);
                    let rendered_body = if row.redacted {
                        format!(
                            "{} (redacted upstream; evidence_refs={})",
                            body,
                            row.evidence_memory_ids.join(",")
                        )
                    } else {
                        body
                    };
                    let token_estimate = estimate_text_tokens(&rendered_body).max(1);
                    let source_report = row.to_report(&rendered_body);
                    let c = ContextCandidate {
                        source_id: digest_candidate_id(&row.claim_id),
                        kind: ContextCandidateKind::KnowledgeDigest,
                        scope: ContextScope::Session,
                        priority: 55,
                        trust: TrustLevel::Untrusted,
                        cache_class: ContextCacheClass::Dynamic,
                        target: ContextTarget::UserSide,
                        content: rendered_body,
                        token_estimate,
                        diagnostics: vec![
                            "governed_knowledge_digest".into(),
                            "not_canonical_transcript".into(),
                        ],
                        evidence_memory_ids: row.evidence_memory_ids.clone(),
                        digest_strength,
                        source_report: Some(source_report),
                    };
                    candidates.push(c);
                }

                Ok(ContextProviderOutcome {
                    candidates,
                    diagnostics: Vec::new(),
                    active_recall_report: None,
                })
            }
            Ok(Err(err)) => Ok(ContextProviderOutcome {
                candidates: Vec::new(),
                diagnostics: vec![ContextProviderDiagnostics {
                    provider_id: self.provider_id().into(),
                    message: format!("knowledge digest capability failed (fail-open): {err}"),
                }],
                active_recall_report: None,
            }),
            Err(_) => Ok(ContextProviderOutcome {
                candidates: Vec::new(),
                diagnostics: vec![ContextProviderDiagnostics {
                    provider_id: self.provider_id().into(),
                    message: format!(
                        "knowledge digest timed out after {}ms (fail-open)",
                        self.config.timeout_ms
                    ),
                }],
                active_recall_report: None,
            }),
        }
    }
}

/// Ergonomic wrapper for catalog assembly (`Arc` erase).
#[must_use]
pub fn knowledge_digest_provider_arc(
    inner: Arc<dyn KnowledgeDigestCapability>,
    config: KnowledgeDigestContextConfig,
) -> Arc<dyn ContextProvider> {
    Arc::new(KnowledgeDigestContextProvider::new(inner, config))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::ContextAssembleInput;
    use crate::memory::{ContextSourceProvenance, KnowledgeDigestItem, PrivacyTier};
    use async_trait::async_trait;
    use macaca_proto::{LlmMessage, LlmOptions};

    struct StaticDigestCapability {
        rows: Vec<KnowledgeDigestItem>,
        fail: bool,
    }

    #[async_trait]
    impl KnowledgeDigestCapability for StaticDigestCapability {
        fn capability_id(&self) -> &str {
            "workspace-knowledge-digest"
        }

        async fn digest_for_request(
            &self,
            _input: &ContextAssembleInput,
        ) -> MacacaResult<Vec<KnowledgeDigestItem>> {
            if self.fail {
                return Err(macaca_proto::MacacaError::Agent("digest failed".into()));
            }
            Ok(self.rows.clone())
        }
    }

    fn compose_ctx() -> ContextComposeContext<'static> {
        let input = Box::leak(Box::new(ContextAssembleInput::legacy(
            "agent",
            "model",
            vec![LlmMessage::user("what do we know?")],
            LlmOptions::default(),
        )));
        ContextComposeContext {
            assemble_input: input,
        }
    }

    fn digest_item() -> KnowledgeDigestItem {
        KnowledgeDigestItem {
            claim_id: "claim-1".into(),
            label: "compiled-knowledge".into(),
            statement: "always verify deploy health first".into(),
            provenance: ContextSourceProvenance {
                provider_id: "workspace-knowledge-digest".into(),
                source_id: "claim-1".into(),
                evidence: vec!["memory-a".into()],
            },
            evidence_memory_ids: vec!["memory-a".into()],
            confidence: 0.82,
            freshness: 0.91,
            privacy_tier: PrivacyTier::Workspace,
            request_only: true,
            redacted: true,
        }
    }

    #[tokio::test]
    async fn knowledge_digest_provider_is_opt_in_and_invisible_by_default() {
        let provider = KnowledgeDigestContextProvider::new(
            Arc::new(StaticDigestCapability {
                rows: vec![digest_item()],
                fail: false,
            }),
            KnowledgeDigestContextConfig::default(),
        );

        let outcome = provider.contribute(&compose_ctx()).await.unwrap();
        assert!(outcome.candidates.is_empty());
        assert!(outcome.diagnostics.is_empty());
    }

    #[tokio::test]
    async fn knowledge_digest_provider_fails_open_with_diagnostic() {
        let mut cfg = KnowledgeDigestContextConfig::default();
        cfg.enabled = true;
        cfg.timeout_ms = 10;
        let provider = KnowledgeDigestContextProvider::new(
            Arc::new(StaticDigestCapability {
                rows: Vec::new(),
                fail: true,
            }),
            cfg,
        );

        let outcome = provider.contribute(&compose_ctx()).await.unwrap();
        assert!(outcome.candidates.is_empty());
        assert!(outcome
            .diagnostics
            .iter()
            .any(|d| d.message.contains("fail-open")));
    }

    #[tokio::test]
    async fn knowledge_digest_candidates_are_dynamic_untrusted_and_request_only() {
        let cfg = KnowledgeDigestContextConfig {
            enabled: true,
            ..KnowledgeDigestContextConfig::default()
        };
        let provider = KnowledgeDigestContextProvider::new(
            Arc::new(StaticDigestCapability {
                rows: vec![digest_item()],
                fail: false,
            }),
            cfg,
        );

        let ctx = compose_ctx();
        let original_messages = ctx.assemble_input.base_messages.clone();
        let outcome = provider.contribute(&ctx).await.unwrap();
        let candidate = outcome.candidates.first().expect("digest candidate");
        let report = candidate
            .source_report
            .as_ref()
            .expect("digest source report");

        assert_eq!(candidate.cache_class, ContextCacheClass::Dynamic);
        assert_eq!(candidate.trust, TrustLevel::Untrusted);
        assert!(candidate
            .diagnostics
            .iter()
            .any(|diag| diag == "not_canonical_transcript"));
        assert_eq!(report.request_only, Some(true));
        assert_eq!(report.trust_level.as_deref(), Some("untrusted"));
        assert_eq!(
            serde_json::to_value(&ctx.assemble_input.base_messages).unwrap(),
            serde_json::to_value(&original_messages).unwrap()
        );
    }
}
