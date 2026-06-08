//! Pruning context engine — untrusted source normalization (**Strategy** + **Adapter**).
//!
//! Delegates per-message rendering to a pluggable [`ContextRenderable`] implementation
//! so pruning policy stays swappable while report accounting remains centralized.

use async_trait::async_trait;
use macaca_proto::MacacaResult;

use crate::estimate::estimate_text_tokens;
use crate::prompt::TrustLevel;
use crate::report::{ContextReportBuilder, ContextSourceKind, ContextSourceReport};
use crate::source::{
    decision_for_snippet, ContextRenderInput, ContextRenderable, ContextSourceReference,
    DefaultSourceRenderer,
};

use super::helpers::message_source_kind;
use super::types::{
    ContextAssembleInput, ContextAssembleResult, ContextEngine, ContextEngineInfo,
    ContextOptionsPatch,
};

/// Engine that normalizes large or untrusted sources through a renderer.
///
/// Instead of making pruning decisions inline, the engine delegates each
/// message to `ContextRenderable`. That keeps pruning policy pluggable while
/// centralizing report accounting in one place.
#[derive(Debug, Clone, Default)]
pub struct PruningContextEngine<R = DefaultSourceRenderer> {
    renderer: R,
}

impl<R> PruningContextEngine<R> {
    pub const ID: &'static str = "pruning";

    /// Build a pruning engine with a caller-supplied renderer/policy stack.
    pub fn new(renderer: R) -> Self {
        Self { renderer }
    }
}

#[async_trait]
impl<R> ContextEngine for PruningContextEngine<R>
where
    R: ContextRenderable,
{
    fn info(&self) -> ContextEngineInfo {
        ContextEngineInfo::new(Self::ID, "Pruning Context Engine")
    }

    async fn assemble(&self, input: ContextAssembleInput) -> MacacaResult<ContextAssembleResult> {
        let mut report_builder = ContextReportBuilder::new(Self::ID)
            .identity(
                input.app_id,
                input.session_id.clone(),
                input.agent_name.clone(),
                input.model.clone(),
            )
            .budget(input.budget);

        let mut messages = Vec::with_capacity(input.base_messages.len());
        for (idx, mut message) in input.base_messages.into_iter().enumerate() {
            let kind = message_source_kind(&message);
            let trust_level = if matches!(
                kind,
                ContextSourceKind::Memory
                    | ContextSourceKind::WikiDigest
                    | ContextSourceKind::Trace
                    | ContextSourceKind::ToolResult
                    | ContextSourceKind::External
                    | ContextSourceKind::Workspace
            ) {
                TrustLevel::Untrusted
            } else {
                TrustLevel::Trusted
            };
            let snippet = self.renderer.render(ContextRenderInput::new(
                ContextSourceReference::new(
                    format!("message/{idx}"),
                    kind,
                    format!("{:?}", message.role),
                ),
                message.content.clone(),
                trust_level,
            ));
            report_builder = report_builder
                .source(snippet.to_report())
                .decision(decision_for_snippet(&snippet));
            message.content = snippet.text;
            messages.push(message);
        }

        if let Some(tools) = input.options.tools.as_ref() {
            let bytes = serde_json::to_vec(tools).map(|v| v.len()).unwrap_or(0);
            let token_estimate = serde_json::to_string(tools)
                .map(|text| estimate_text_tokens(&text))
                .unwrap_or(0);
            report_builder = report_builder.source(ContextSourceReport::included(
                "tool_schema",
                ContextSourceKind::ToolSchema,
                "LLM tool schema",
                token_estimate,
                bytes,
            ));
        }

        Ok(ContextAssembleResult {
            messages,
            options: input.options,
            options_patch: ContextOptionsPatch::default(),
            report: report_builder.build(),
        })
    }
}
