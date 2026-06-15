//! [`MemoryActiveRecallContextProvider`] bridges the memory subsystem's active recall capability
//! into the **context composer** pipeline (Chain-of-Responsibility stage
//! [`crate::composer::ContextProviderStage::ActiveRecall`]).
//!
//! ## Design constraints (from OpenSpec)
//! - Depends only on [`crate::memory::ActiveRecallCapability`] — never on a concrete vector DB.
//! - Surfaces recall as **dynamic, untrusted, fenced** [`crate::composer::ContextCandidate`] rows
//!   so the merge step injects a Hermes-style user-side block (see [`crate::composer::facade`]).
//! - Bounded telemetry is exported separately via [`ContextProviderOutcome::active_recall_report`].

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use macaca_proto::config::ActiveVectorMemoryContextConfig;
use macaca_proto::MacacaResult;
use tokio::time::timeout;

use crate::active_recall::{active_recall_diagnostics_from_prefetch, render_active_recall_fence};
use crate::estimate::estimate_text_tokens;
use crate::memory::ActiveRecallCapability;
use crate::memory::MemoryRecallQuery;
use crate::prompt::TrustLevel;
use crate::{
    ContextCacheClass, ContextCandidate, ContextCandidateKind, ContextComposeContext,
    ContextProvider, ContextProviderDiagnostics, ContextProviderOutcome, ContextProviderStage,
    ContextScope, ContextTarget,
};

/// Composer-side adapter around a narrow recall capability.
#[derive(Clone)]
pub struct MemoryActiveRecallContextProvider {
    /// Underlying recall implementation (typically [`crate::active_recall::DefaultActiveRecallProvider`]).
    capability: Arc<dyn ActiveRecallCapability>,
    /// Feature flags and routing hints copied from [`macaca_proto::config::ContextConfig`].
    config: ActiveVectorMemoryContextConfig,
}

impl MemoryActiveRecallContextProvider {
    /// Wraps a capability implementation that already embeds policy/budget decisions.
    #[must_use]
    pub fn new(
        capability: Arc<dyn ActiveRecallCapability>,
        config: ActiveVectorMemoryContextConfig,
    ) -> Self {
        Self { capability, config }
    }
}

#[async_trait]
impl ContextProvider for MemoryActiveRecallContextProvider {
    fn provider_id(&self) -> &str {
        "active_vector_memory"
    }

    fn stage(&self) -> ContextProviderStage {
        ContextProviderStage::ActiveRecall
    }

    async fn contribute(
        &self,
        ctx: &ContextComposeContext<'_>,
    ) -> MacacaResult<ContextProviderOutcome> {
        if !self.config.enabled {
            return Ok(ContextProviderOutcome::default());
        }

        let input = ctx.assemble_input;
        let Some(query_text) = input.last_user_message_text() else {
            return Ok(ContextProviderOutcome::default());
        };

        let mq = MemoryRecallQuery {
            query: query_text,
            session_id: input.session_id.clone(),
            application_id: input.app_id.map(|id| id.to_string()),
            agent_name: Some(input.agent_name.clone()),
            max_tokens: self.config.max_query_tokens,
            include_agent_private: self.config.include_agent_private,
            include_session_shared: self.config.include_session_shared,
        };

        let clock = std::time::Instant::now();
        let prefetch_future = self.capability.prefetch(mq);
        let outcome = timeout(
            Duration::from_millis(self.config.timeout_ms),
            prefetch_future,
        )
        .await;

        match outcome {
            Ok(Ok(prefetch)) => {
                let latency_ms = clock.elapsed().as_millis() as u64;
                let telemetry = active_recall_diagnostics_from_prefetch(
                    self.capability.provider_id(),
                    &prefetch,
                    latency_ms,
                );

                if prefetch.snippets.is_empty() {
                    return Ok(ContextProviderOutcome {
                        candidates: Vec::new(),
                        diagnostics: Vec::new(),
                        active_recall_report: Some(telemetry),
                    });
                }

                let fence_body = render_active_recall_fence(&prefetch);
                let token_estimate = estimate_text_tokens(&fence_body).max(1);
                let evidence_memory_ids: Vec<String> = prefetch
                    .snippets
                    .iter()
                    .map(|snippet| snippet.source.id.clone())
                    .collect();
                let candidate = ContextCandidate {
                    source_id: "active_vector_memory/recall".into(),
                    kind: ContextCandidateKind::MemoryRecall,
                    scope: ContextScope::Session,
                    priority: 40,
                    trust: TrustLevel::Untrusted,
                    cache_class: ContextCacheClass::Dynamic,
                    target: ContextTarget::UserSide,
                    content: fence_body,
                    token_estimate,
                    diagnostics: vec![
                        "fenced_dynamic_request_only".into(),
                        "not_canonical_transcript".into(),
                    ],
                    evidence_memory_ids,
                    digest_strength: None,
                    source_report: None,
                };

                Ok(ContextProviderOutcome {
                    candidates: vec![candidate],
                    diagnostics: Vec::new(),
                    active_recall_report: Some(telemetry),
                })
            }
            Ok(Err(error)) => Ok(ContextProviderOutcome {
                candidates: Vec::new(),
                diagnostics: vec![ContextProviderDiagnostics {
                    provider_id: self.provider_id().into(),
                    message: format!("active recall capability failed (fail-open): {error}"),
                }],
                active_recall_report: None,
            }),
            Err(_) => Ok(ContextProviderOutcome {
                candidates: Vec::new(),
                diagnostics: vec![ContextProviderDiagnostics {
                    provider_id: self.provider_id().into(),
                    message: format!(
                        "active recall timed out after {}ms (fail-open)",
                        self.config.timeout_ms
                    ),
                }],
                active_recall_report: None,
            }),
        }
    }
}

/// Ergonomic wrapper for [`FrameworkRunner`] / HTTP bootstrap code.
#[must_use]
pub fn memory_active_recall_provider_arc(
    capability: Arc<dyn ActiveRecallCapability>,
    config: ActiveVectorMemoryContextConfig,
) -> Arc<dyn ContextProvider> {
    Arc::new(MemoryActiveRecallContextProvider::new(capability, config))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::ContextAssembleInput;
    use crate::memory::{
        memory_source, ConfidenceScore, ContextSourceProvenance, MemoryPrefetchResult,
        MemoryRecallItem, RecallCandidate,
    };
    use async_trait::async_trait;
    use macaca_proto::{LlmMessage, LlmOptions};

    struct StaticRecallCapability {
        result: MemoryPrefetchResult,
        fail: bool,
    }

    #[async_trait]
    impl ActiveRecallCapability for StaticRecallCapability {
        fn provider_id(&self) -> &str {
            "workspace-memory"
        }

        async fn prefetch(&self, _query: MemoryRecallQuery) -> MacacaResult<MemoryPrefetchResult> {
            if self.fail {
                return Err(macaca_proto::MacacaError::Agent("recall failed".into()));
            }
            Ok(self.result.clone())
        }
    }

    fn compose_ctx() -> ContextComposeContext<'static> {
        let input = Box::leak(Box::new(ContextAssembleInput::unscoped(
            "agent",
            "model",
            vec![LlmMessage::user("find memory")],
            LlmOptions::default(),
        )));
        ContextComposeContext {
            assemble_input: input,
        }
    }

    fn prefetch_result() -> MemoryPrefetchResult {
        let item = MemoryRecallItem {
            source: memory_source("workspace-memory", "memory-1", "layer:Vector"),
            text: "remembered fact".into(),
            provenance: ContextSourceProvenance {
                provider_id: "workspace-memory".into(),
                source_id: "memory-1".into(),
                evidence: vec!["layer:Vector".into()],
            },
            confidence: ConfidenceScore::new(90),
            privacy_tier: crate::memory::PrivacyTier::Workspace,
        };
        MemoryPrefetchResult {
            candidates: vec![RecallCandidate {
                item: item.clone(),
                score: 90,
                freshness_rank: 1,
                decision: crate::memory::RecallDecision::selected("selected"),
            }],
            snippets: vec![crate::ContextSnippet {
                source: item.source.clone(),
                text: item.text.clone(),
                estimated_tokens: 12,
                byte_size: item.text.len(),
                pruned_tokens: 0,
                trust_level: crate::prompt::TrustLevel::Untrusted,
                mode: crate::source::ContextRenderMode::Full,
            }],
            decisions: vec![crate::report::ContextDecisionReport::info(
                "active_recall_applied",
                "selected 1 row",
            )],
        }
    }

    #[tokio::test]
    async fn active_recall_provider_is_opt_in_and_invisible_by_default() {
        let provider = MemoryActiveRecallContextProvider::new(
            Arc::new(StaticRecallCapability {
                result: prefetch_result(),
                fail: false,
            }),
            ActiveVectorMemoryContextConfig::default(),
        );

        let outcome = provider.contribute(&compose_ctx()).await.unwrap();
        assert!(outcome.candidates.is_empty());
        assert!(outcome.active_recall_report.is_none());
    }

    #[tokio::test]
    async fn active_recall_provider_fails_open_with_diagnostic() {
        let mut cfg = ActiveVectorMemoryContextConfig::default();
        cfg.enabled = true;
        cfg.timeout_ms = 10;
        let provider = MemoryActiveRecallContextProvider::new(
            Arc::new(StaticRecallCapability {
                result: prefetch_result(),
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
}
