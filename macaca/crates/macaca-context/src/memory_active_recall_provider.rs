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
use macaca_proto::LlmRole;
use macaca_proto::MacacaResult;
use tokio::time::timeout;

use crate::active_recall::{active_recall_diagnostics_from_prefetch, render_active_recall_fence};
use crate::{
    ContextCacheClass, ContextCandidate, ContextCandidateKind, ContextComposeContext,
    ContextProvider, ContextProviderDiagnostics, ContextProviderOutcome, ContextProviderStage,
    ContextScope, ContextTarget,
};
use crate::estimate::estimate_text_tokens;
use crate::memory::ActiveRecallCapability;
use crate::memory::MemoryRecallQuery;
use crate::prompt::TrustLevel;

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

/// Locates the most recent non-empty **user** message, which we treat as the retrieval query.
///
/// This mirrors the web runtime's framework message codec behaviour without depending on JSON
/// serialisation here: composer input is already normalised to [`macaca_proto::LlmMessage`].
fn last_user_query(messages: &[macaca_proto::LlmMessage]) -> Option<String> {
    for message in messages.iter().rev() {
        if message.role == LlmRole::User {
            let trimmed = message.content.trim();
            if !trimmed.is_empty() {
                return Some(message.content.clone());
            }
        }
    }
    None
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
        let Some(query_text) = last_user_query(&input.base_messages) else {
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
                let telemetry =
                    active_recall_diagnostics_from_prefetch(self.capability.provider_id(), &prefetch, latency_ms);

                if prefetch.snippets.is_empty() {
                    return Ok(ContextProviderOutcome {
                        candidates: Vec::new(),
                        diagnostics: Vec::new(),
                        active_recall_report: Some(telemetry),
                    });
                }

                let fence_body = render_active_recall_fence(&prefetch);
                let token_estimate = estimate_text_tokens(&fence_body).max(1);
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
