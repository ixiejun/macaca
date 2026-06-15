//! Passthrough context engine (**Strategy**).
//!
//! Upper layers always talk to a [`ContextEngine`]; the passthrough strategy
//! preserves incoming messages without mutation while still emitting structured
//! [`ContextReport`] data for trace and audit consumers.

use async_trait::async_trait;
use macaca_proto::MacacaResult;

use super::helpers::build_passthrough_report;
use super::types::{
    ContextAssembleInput, ContextAssembleResult, ContextEngine, ContextEngineInfo,
    ContextOptionsPatch, PASSTHROUGH_ENGINE_ID,
};

/// Engine that preserves incoming messages and options while producing reports.
#[derive(Debug, Clone, Default)]
pub struct PassthroughContextEngine;

impl PassthroughContextEngine {
    pub const ID: &'static str = PASSTHROUGH_ENGINE_ID;
}

#[async_trait]
impl ContextEngine for PassthroughContextEngine {
    fn info(&self) -> ContextEngineInfo {
        ContextEngineInfo::new(Self::ID, "Passthrough Context Engine")
    }

    async fn assemble(&self, input: ContextAssembleInput) -> MacacaResult<ContextAssembleResult> {
        let report = build_passthrough_report(&input);
        Ok(ContextAssembleResult {
            messages: input.base_messages,
            options: input.options,
            options_patch: ContextOptionsPatch::default(),
            report,
        })
    }
}
