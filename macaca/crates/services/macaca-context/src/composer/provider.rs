//! [`ContextProvider`] trait: one link in a Chain-of-Responsibility pipeline.
//!
//! Behavior:
//! **Legacy engine path:** [`crate::LegacyContextEngine`] is the historical “pass-through” assembler
//! (no composer fan-in). Prefer [`super::facade::ContextFacade::assemble_model_context`] for OS features
//! requiring provider candidates.
//!
//! 1. `ContextFacade` sorts providers by [`ContextProviderStage`] ascending, then by
//!    `provider_id`, and calls them in that order for determinism.
//! 2. Each provider returns [`crate::composer::ContextCandidate`] values asynchronously and
//!    must **not** mutate `Vec<LlmMessage>` directly, so canonical transcripts cannot be altered
//!    accidentally.
//! 3. Rich diagnostics flow through [`ContextProviderOutcome`] and end up in [`ContextPlan`]
//!    summaries.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::MacacaResult;
use serde::{Deserialize, Serialize};

use crate::composer::candidate::ContextCandidate;
use crate::engine::ContextAssembleInput;

/// Stage index for ordering providers (sort key only; the composer still performs cuts).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ContextProviderStage {
    /// Stable profile / guide material.
    StableProfile = 0,
    /// Capability catalogs (skills / MCP summaries, etc.).
    CapabilityIndex = 1,
    /// Compiled knowledge digests (governed claims) — runs **before** raw vector injection.
    KnowledgeDigest = 2,
    /// Active recall and similar dynamic memory.
    ActiveRecall = 3,
    /// Runtime injection (trace, diagnostics).
    RuntimeDynamic = 4,
    /// Diagnostics-only (may leave `content` empty if not meant for the model).
    Diagnostics = 5,
}

/// Read-only snapshot passed into providers for one assembly pass.
#[derive(Debug, Clone)]
pub struct ContextComposeContext<'a> {
    /// Snapshot of the input about to enter the context engine.
    pub assemble_input: &'a ContextAssembleInput,
}

/// Lightweight diagnostic line emitted by a provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextProviderDiagnostics {
    pub provider_id: String,
    pub message: String,
}

/// Outcome of one `contribute` call.
#[derive(Debug, Clone, Default)]
pub struct ContextProviderOutcome {
    pub candidates: Vec<ContextCandidate>,
    pub diagnostics: Vec<ContextProviderDiagnostics>,
    /// Optional bounded recall telemetry merged into [`crate::report::ContextReport::active_recall`].
    pub active_recall_report: Option<crate::report::ActiveRecallDiagnostics>,
}

/// Provider contract: add candidates only; never touch raw LLM message vectors.
#[async_trait]
pub trait ContextProvider: Send + Sync {
    /// Stable id for sorting/logging; type name or registry name, no randomness.
    fn provider_id(&self) -> &str;

    fn stage(&self) -> ContextProviderStage;

    /// Optional semver for diagnostics (registry/catalog may wrap providers with a [`crate::catalog::VersionedContextProvider`]).
    fn implementation_version(&self) -> Option<&'static str> {
        None
    }

    /// Return candidates for this model call.
    async fn contribute(
        &self,
        ctx: &ContextComposeContext<'_>,
    ) -> MacacaResult<ContextProviderOutcome>;
}

/// Deterministic sort of `Arc<dyn ContextProvider>` slices for the facade.
pub fn sort_providers(providers: &[Arc<dyn ContextProvider>]) -> Vec<Arc<dyn ContextProvider>> {
    let mut list: Vec<_> = providers.to_vec();
    list.sort_by(|a, b| {
        a.stage()
            .cmp(&b.stage())
            .then_with(|| a.provider_id().cmp(b.provider_id()))
    });
    list
}

/// Minimal validation: invalid rows become skip decisions without aborting the pipeline.
pub fn validate_candidate(c: &ContextCandidate) -> std::result::Result<(), String> {
    if c.source_id.is_empty() {
        return Err("source_id must be non-empty".into());
    }
    Ok(())
}
