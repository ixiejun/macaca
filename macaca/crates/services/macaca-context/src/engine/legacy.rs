//! Passthrough compatibility context engine (**Strategy** / migration seam).
//!
//! Upper layers always talk to a [`ContextEngine`]; `legacy` preserves pre-engine
//! behavior (no mutation) while still emitting structured [`ContextReport`] data
//! for trace and audit consumers.

use async_trait::async_trait;
use macaca_proto::MacacaResult;

use super::helpers::build_legacy_report;
use super::types::{
    ContextAssembleInput, ContextAssembleResult, ContextEngine, ContextEngineInfo,
    ContextOptionsPatch, LEGACY_ENGINE_ID,
};

/// Compatibility engine that preserves the old passthrough behavior.
///
/// This is the additive migration seam: upper layers always talk to a context
/// engine, but `legacy` keeps behavior unchanged while still producing reports.
#[derive(Debug, Clone, Default)]
pub struct LegacyContextEngine;

impl LegacyContextEngine {
    pub const ID: &'static str = LEGACY_ENGINE_ID;
}

#[async_trait]
impl ContextEngine for LegacyContextEngine {
    fn info(&self) -> ContextEngineInfo {
        ContextEngineInfo::new(Self::ID, "Legacy Context Engine")
    }

    async fn assemble(&self, input: ContextAssembleInput) -> MacacaResult<ContextAssembleResult> {
        let report = build_legacy_report(&input);
        Ok(ContextAssembleResult {
            messages: input.base_messages,
            options: input.options,
            options_patch: ContextOptionsPatch::default(),
            report,
        })
    }
}
