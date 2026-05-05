//! Active recall orchestration for memory sources.
//!
//! The active recall layer is deliberately implemented as a Strategy/Adapter
//! boundary. It asks a `MemorySourceProvider` for candidates, applies a bounded
//! policy, and returns dynamic/untrusted `ContextSnippet` values. It does not
//! assemble prompts and it does not write anything back to canonical session
//! history.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::MacacaResult;

use crate::memory::{
    ActiveRecallCapability, ActiveRecallPolicy, ActiveRecallReport, MemoryPrefetchResult,
    MemoryRecallQuery, MemorySourceProvider, RecallCandidate,
};
use crate::report::{ContextDecisionReport, ContextDecisionSeverity};

/// Default active recall provider backed by a generic memory source.
pub struct DefaultActiveRecallProvider {
    provider_id: String,
    memory: Arc<dyn MemorySourceProvider>,
    policy: ActiveRecallPolicy,
}

impl DefaultActiveRecallProvider {
    /// Create a provider from a source provider and an explicit policy.
    pub fn new(
        provider_id: impl Into<String>,
        memory: Arc<dyn MemorySourceProvider>,
        policy: ActiveRecallPolicy,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            memory,
            policy,
        }
    }

    /// Build a diagnostics report without copying full memory content.
    pub fn report_from_result(
        &self,
        result: &MemoryPrefetchResult,
        latency_ms: u64,
    ) -> ActiveRecallReport {
        ActiveRecallReport {
            provider_id: self.provider_id.clone(),
            source_breakdown: result
                .snippets
                .iter()
                .map(|snippet| snippet.to_report())
                .collect(),
            decisions: result.decisions.clone(),
            total_candidates: result.candidates.len(),
            selected_candidates: result.snippets.len(),
            latency_ms,
        }
    }
}

#[async_trait]
impl ActiveRecallCapability for DefaultActiveRecallProvider {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    async fn prefetch(&self, query: MemoryRecallQuery) -> MacacaResult<MemoryPrefetchResult> {
        let started = std::time::Instant::now();
        let items = self.memory.recall(query).await?;
        let mut candidates = Vec::new();
        let mut snippets = Vec::new();
        let mut decisions = Vec::new();
        let mut used_hits = 0usize;
        let mut used_chars = 0usize;
        let mut used_tokens = 0u32;

        for item in items {
            let decision = self
                .policy
                .admit_candidate(&item, used_hits, used_chars, used_tokens);
            let score = item.confidence.value() as u32;
            if decision.selected {
                let snippet = item.clone().into_untrusted_snippet();
                used_hits += 1;
                used_chars += snippet.byte_size;
                used_tokens = used_tokens.saturating_add(snippet.estimated_tokens);
                snippets.push(snippet);
            } else {
                decisions.push(ContextDecisionReport {
                    code: "active_recall_skipped".into(),
                    severity: ContextDecisionSeverity::Info,
                    message: decision.reason.clone(),
                });
            }
            candidates.push(RecallCandidate {
                item,
                score,
                freshness_rank: 0,
                decision,
            });
        }

        decisions.push(ContextDecisionReport {
            code: "active_recall_completed".into(),
            severity: ContextDecisionSeverity::Info,
            message: format!(
                "active recall selected {} of {} candidates in {}ms",
                snippets.len(),
                candidates.len(),
                started.elapsed().as_millis()
            ),
        });

        Ok(MemoryPrefetchResult {
            candidates,
            snippets,
            decisions,
        })
    }
}

/// Convert recall snippets into a fenced dynamic section string.
pub fn render_active_recall_fence(result: &MemoryPrefetchResult) -> String {
    let body = result
        .snippets
        .iter()
        .enumerate()
        .map(|(index, snippet)| {
            format!(
                "[{}] source={} trust=untrusted\n{}",
                index + 1,
                snippet.source.label,
                snippet.text
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    format!("[Active memory recall - reference only, verify before acting]\n{body}")
}

/// Standard degradation result used when active recall is disabled or unavailable.
pub fn active_recall_degraded(message: impl Into<String>) -> MemoryPrefetchResult {
    MemoryPrefetchResult {
        candidates: Vec::new(),
        snippets: Vec::new(),
        decisions: vec![ContextDecisionReport {
            code: "active_recall_degraded".into(),
            severity: ContextDecisionSeverity::Warning,
            message: message.into(),
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{
        memory_source, ConfidenceScore, ContextSourceProvenance, MemoryRecallItem, PrivacyTier,
    };

    struct FakeMemorySource {
        items: Vec<MemoryRecallItem>,
    }

    #[async_trait]
    impl MemorySourceProvider for FakeMemorySource {
        fn provider_id(&self) -> &str {
            "fake-memory"
        }

        async fn recall(&self, _query: MemoryRecallQuery) -> MacacaResult<Vec<MemoryRecallItem>> {
            Ok(self.items.clone())
        }
    }

    fn item(id: &str, text: &str) -> MemoryRecallItem {
        MemoryRecallItem {
            source: memory_source("fake-memory", id, "memory"),
            text: text.into(),
            provenance: ContextSourceProvenance {
                provider_id: "fake-memory".into(),
                source_id: id.into(),
                evidence: vec!["test".into()],
            },
            confidence: ConfidenceScore::new(90),
            privacy_tier: PrivacyTier::Workspace,
        }
    }

    #[tokio::test]
    async fn default_active_recall_enforces_hit_budget() {
        let source = Arc::new(FakeMemorySource {
            items: vec![item("a", "alpha"), item("b", "beta")],
        });
        let provider = DefaultActiveRecallProvider::new(
            "active",
            source,
            ActiveRecallPolicy {
                budget: crate::memory::ActiveRecallBudget {
                    max_hits: 1,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let result = provider
            .prefetch(MemoryRecallQuery {
                query: "test".into(),
                session_id: Some("s1".into()),
                max_tokens: 100,
            })
            .await
            .unwrap();

        assert_eq!(result.snippets.len(), 1);
        assert!(result
            .candidates
            .iter()
            .any(|candidate| !candidate.decision.selected));
    }

    #[tokio::test]
    async fn active_recall_snippets_are_dynamic_untrusted() {
        let source = Arc::new(FakeMemorySource {
            items: vec![item("a", "remembered fact")],
        });
        let provider =
            DefaultActiveRecallProvider::new("active", source, ActiveRecallPolicy::default());
        let result = provider
            .prefetch(MemoryRecallQuery {
                query: "fact".into(),
                session_id: None,
                max_tokens: 100,
            })
            .await
            .unwrap();

        let snippet = result.snippets.first().unwrap();
        assert_eq!(snippet.trust_level, crate::TrustLevel::Untrusted);
        assert_eq!(snippet.source.kind, crate::ContextSourceKind::Memory);
    }

    #[tokio::test]
    async fn active_recall_report_omits_full_content() {
        let full_text = "sensitive complete memory text";
        let source = Arc::new(FakeMemorySource {
            items: vec![item("a", full_text)],
        });
        let provider =
            DefaultActiveRecallProvider::new("active", source, ActiveRecallPolicy::default());
        let result = provider
            .prefetch(MemoryRecallQuery {
                query: "memory".into(),
                session_id: None,
                max_tokens: 100,
            })
            .await
            .unwrap();
        let report = provider.report_from_result(&result, 12);
        let serialized = serde_json::to_string(&report).unwrap();

        assert!(!serialized.contains(full_text));
        assert!(serialized.contains("source_breakdown"));
    }
}
