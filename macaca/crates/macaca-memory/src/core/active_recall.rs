//! Active recall capability for the memory fabric.
//!
//! This module keeps active recall inside the memory boundary as a replaceable
//! Strategy. The strategy receives a scoped memory facade, asks it for bounded
//! prefetch/search results, and returns diagnostics-rich candidates. It does
//! not render prompts and it does not persist recalled content.

use async_trait::async_trait;
use macaca_proto::{MacacaResult, MemoryEntry};
use serde::{Deserialize, Serialize};

use super::facade::{MemoryFacade, MemorySearchRequest};
use super::scope::MemoryScope;

/// Budget applied to one active recall pass.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActiveRecallBudget {
    pub max_hits: usize,
    pub max_chars: usize,
    pub max_tokens: u32,
    pub timeout_ms: u64,
}

impl Default for ActiveRecallBudget {
    fn default() -> Self {
        Self {
            max_hits: 8,
            max_chars: 4_000,
            max_tokens: 1_000,
            timeout_ms: 1_500,
        }
    }
}

/// Request envelope for active recall.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveRecallRequest {
    pub scope: MemoryScope,
    pub query: String,
    pub budget: ActiveRecallBudget,
}

/// Candidate-level decision for observability.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActiveRecallDecision {
    pub selected: bool,
    pub reason: String,
}

impl ActiveRecallDecision {
    /// Keep a candidate.
    pub fn selected(reason: impl Into<String>) -> Self {
        Self {
            selected: true,
            reason: reason.into(),
        }
    }

    /// Skip a candidate while preserving the reason.
    pub fn skipped(reason: impl Into<String>) -> Self {
        Self {
            selected: false,
            reason: reason.into(),
        }
    }
}

/// One scored recall candidate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveRecallCandidate {
    pub entry: MemoryEntry,
    pub score: u32,
    pub estimated_tokens: u32,
    pub decision: ActiveRecallDecision,
}

/// Output of active recall.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveRecallResult {
    pub provider_id: String,
    pub candidates: Vec<ActiveRecallCandidate>,
    pub selected: Vec<MemoryEntry>,
    pub latency_ms: u64,
    pub diagnostics: Vec<String>,
}

/// Replaceable active recall strategy boundary.
#[async_trait]
pub trait ActiveRecallCapability: Send + Sync {
    async fn prefetch(&self, request: ActiveRecallRequest) -> MacacaResult<ActiveRecallResult>;
}

/// Default strategy that delegates recall to an existing scoped memory facade.
pub struct DefaultActiveRecallStrategy<'a> {
    provider_id: String,
    facade: &'a dyn MemoryFacade,
}

impl<'a> DefaultActiveRecallStrategy<'a> {
    /// Create a default active recall strategy over a scoped facade.
    pub fn new(provider_id: impl Into<String>, facade: &'a dyn MemoryFacade) -> Self {
        Self {
            provider_id: provider_id.into(),
            facade,
        }
    }
}

#[async_trait]
impl ActiveRecallCapability for DefaultActiveRecallStrategy<'_> {
    async fn prefetch(&self, request: ActiveRecallRequest) -> MacacaResult<ActiveRecallResult> {
        let started = std::time::Instant::now();
        let rows = self
            .facade
            .search(MemorySearchRequest::new(
                request.scope,
                request.query,
                request.budget.max_hits.saturating_mul(2).max(1),
            ))
            .await?;
        let mut candidates = Vec::new();
        let mut selected = Vec::new();
        let mut diagnostics = Vec::new();
        let mut used_hits = 0usize;
        let mut used_chars = 0usize;
        let mut used_tokens = 0u32;

        for entry in rows {
            let estimated_tokens = estimate_memory_tokens(&entry.content);
            let decision = if used_hits >= request.budget.max_hits {
                ActiveRecallDecision::skipped("hit_budget_exhausted")
            } else if used_chars.saturating_add(entry.content.len()) > request.budget.max_chars {
                ActiveRecallDecision::skipped("char_budget_exhausted")
            } else if used_tokens.saturating_add(estimated_tokens) > request.budget.max_tokens {
                ActiveRecallDecision::skipped("token_budget_exhausted")
            } else {
                used_hits += 1;
                used_chars += entry.content.len();
                used_tokens = used_tokens.saturating_add(estimated_tokens);
                selected.push(entry.clone());
                ActiveRecallDecision::selected("within_budget")
            };
            if !decision.selected {
                diagnostics.push(format!("skipped:{}:{}", entry.id.0, decision.reason));
            }
            candidates.push(ActiveRecallCandidate {
                entry,
                score: 100u32.saturating_sub(candidates.len() as u32),
                estimated_tokens,
                decision,
            });
        }

        Ok(ActiveRecallResult {
            provider_id: self.provider_id.clone(),
            candidates,
            selected,
            latency_ms: started.elapsed().as_millis() as u64,
            diagnostics,
        })
    }
}

fn estimate_memory_tokens(text: &str) -> u32 {
    let cjk = text
        .chars()
        .filter(|ch| matches!(*ch as u32, 0x4E00..=0x9FFF | 0x3400..=0x4DBF))
        .count();
    let total = text.chars().count();
    let ascii = total.saturating_sub(cjk);
    (cjk as f64 / 1.5 + ascii as f64 / 4.0).ceil() as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{
        MemoryDeleteRequest, MemoryGetRequest, MemoryStatusReport, MemoryWriteRequest,
    };
    use crate::MemoryCapabilitySet;
    use chrono::Utc;
    use macaca_proto::{ApplicationId, MemoryId, MemoryLayer};
    use tokio::sync::RwLock;

    struct FakeFacade {
        entries: RwLock<Vec<MemoryEntry>>,
    }

    #[async_trait]
    impl MemoryFacade for FakeFacade {
        async fn remember(&self, _request: MemoryWriteRequest) -> MacacaResult<MemoryId> {
            Ok(MemoryId::new())
        }

        async fn search(&self, _request: MemorySearchRequest) -> MacacaResult<Vec<MemoryEntry>> {
            Ok(self.entries.read().await.clone())
        }

        async fn get(&self, _request: MemoryGetRequest) -> MacacaResult<Option<MemoryEntry>> {
            Ok(None)
        }

        async fn delete(&self, _request: MemoryDeleteRequest) -> MacacaResult<()> {
            Ok(())
        }

        fn status(&self) -> MemoryStatusReport {
            MemoryStatusReport::healthy("fake", MemoryCapabilitySet::basic_store_search())
        }
    }

    fn entry(text: &str) -> MemoryEntry {
        MemoryEntry {
            id: MemoryId::new(),
            layer: MemoryLayer::Session,
            content: text.into(),
            metadata: serde_json::Value::Null,
            agent_id: None,
            created_at: Utc::now(),
            expires_at: None,
        }
    }

    #[tokio::test]
    async fn default_strategy_selects_within_budget_and_records_skips() {
        let facade = FakeFacade {
            entries: RwLock::new(vec![entry("one"), entry("two")]),
        };
        let strategy = DefaultActiveRecallStrategy::new("default", &facade);
        let result = strategy
            .prefetch(ActiveRecallRequest {
                scope: MemoryScope::session_shared(ApplicationId::new(), "s1"),
                query: "anything".into(),
                budget: ActiveRecallBudget {
                    max_hits: 1,
                    ..Default::default()
                },
            })
            .await
            .unwrap();

        assert_eq!(result.selected.len(), 1);
        assert!(result
            .candidates
            .iter()
            .any(|candidate| !candidate.decision.selected));
    }
}
