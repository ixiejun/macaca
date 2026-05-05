//! Context engine contracts and the legacy compatibility implementation.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{ApplicationId, LlmMessage, LlmOptions, MacacaResult, ToolDefinition};
use serde::{Deserialize, Serialize};

use crate::budget::ContextBudget;
use crate::compaction::{
    CompactionHookInput, CompactionLifecycleHook, CompactionPolicy, CompactionSummaryEnvelope,
    CompactionTrigger, NoopCompactionLifecycleHook, SessionLineage, ThresholdCompactionPolicy,
};
use crate::estimate::estimate_text_tokens;
use crate::prompt::{PromptComposer, PromptSection, PromptStability, TrustLevel};
use crate::report::{
    ContextDecisionReport, ContextDecisionSeverity, ContextReport, ContextReportBuilder,
    ContextSourceKind, ContextSourceReport,
};
use crate::source::{
    decision_for_snippet, ContextRenderInput, ContextRenderable, ContextSourceReference,
    DefaultSourceRenderer,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextEngineInfo {
    pub id: String,
    pub name: String,
    pub version: String,
}

impl ContextEngineInfo {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContextOptionsPatch {
    pub tools: Option<Vec<ToolDefinition>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextAssembleInput {
    pub app_id: Option<ApplicationId>,
    pub session_id: Option<String>,
    pub agent_name: String,
    pub model: String,
    pub base_messages: Vec<LlmMessage>,
    pub options: LlmOptions,
    pub budget: ContextBudget,
}

impl ContextAssembleInput {
    pub fn legacy(
        agent_name: impl Into<String>,
        model: impl Into<String>,
        base_messages: Vec<LlmMessage>,
        options: LlmOptions,
    ) -> Self {
        Self {
            app_id: None,
            session_id: None,
            agent_name: agent_name.into(),
            model: model.into(),
            base_messages,
            options,
            budget: ContextBudget::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextAssembleResult {
    pub messages: Vec<LlmMessage>,
    pub options: LlmOptions,
    pub options_patch: ContextOptionsPatch,
    pub report: ContextReport,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContextAfterTurnInput {
    pub app_id: Option<ApplicationId>,
    pub session_id: Option<String>,
    pub agent_name: String,
    pub report: Option<ContextReport>,
}

#[async_trait]
pub trait ContextEngine: Send + Sync {
    fn info(&self) -> ContextEngineInfo;

    async fn assemble(&self, input: ContextAssembleInput) -> MacacaResult<ContextAssembleResult>;

    async fn after_turn(&self, _input: ContextAfterTurnInput) -> MacacaResult<()> {
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextEngineSelection {
    pub engine_id: String,
    pub fallback_engine_id: String,
}

impl ContextEngineSelection {
    pub fn legacy() -> Self {
        Self {
            engine_id: LegacyContextEngine::ID.into(),
            fallback_engine_id: LegacyContextEngine::ID.into(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct LegacyContextEngine;

impl LegacyContextEngine {
    pub const ID: &'static str = "legacy";
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

#[derive(Debug, Clone)]
pub struct WindowedContextEngine {
    preserve_recent: usize,
}

impl Default for WindowedContextEngine {
    fn default() -> Self {
        Self {
            preserve_recent: 10,
        }
    }
}

impl WindowedContextEngine {
    pub const ID: &'static str = "windowed";
}

#[async_trait]
impl ContextEngine for WindowedContextEngine {
    fn info(&self) -> ContextEngineInfo {
        ContextEngineInfo::new(Self::ID, "Windowed Context Engine")
    }

    async fn assemble(&self, input: ContextAssembleInput) -> MacacaResult<ContextAssembleResult> {
        let original_len = input.base_messages.len();
        let original_tokens = estimate_messages_tokens(&input.base_messages);
        let messages = trim_to_budget(
            input.base_messages.clone(),
            input.budget,
            self.preserve_recent,
        );
        let trimmed_tokens = estimate_messages_tokens(&messages);
        let mut report = build_report_for_messages(Self::ID, &input, &messages);
        if messages.len() < original_len || trimmed_tokens < original_tokens {
            report.pruned_tokens = original_tokens.saturating_sub(trimmed_tokens);
            report.decisions.push(ContextDecisionReport::info(
                "context_window_trimmed",
                format!(
                    "Windowed engine reduced messages from {original_len} to {}.",
                    messages.len()
                ),
            ));
        }
        Ok(ContextAssembleResult {
            messages,
            options: input.options,
            options_patch: ContextOptionsPatch::default(),
            report,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct PruningContextEngine<R = DefaultSourceRenderer> {
    renderer: R,
}

impl<R> PruningContextEngine<R> {
    pub const ID: &'static str = "pruning";

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

#[derive(Clone)]
pub struct SummaryContextEngine {
    pruning: PruningContextEngine,
    compaction_policy: ThresholdCompactionPolicy,
    hooks: Arc<dyn CompactionLifecycleHook>,
}

impl std::fmt::Debug for SummaryContextEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SummaryContextEngine")
            .field("compaction_policy", &self.compaction_policy)
            .finish_non_exhaustive()
    }
}

impl Default for SummaryContextEngine {
    fn default() -> Self {
        Self {
            pruning: PruningContextEngine::default(),
            compaction_policy: ThresholdCompactionPolicy::default(),
            hooks: Arc::new(NoopCompactionLifecycleHook),
        }
    }
}

impl SummaryContextEngine {
    pub const ID: &'static str = "summary";
}

#[async_trait]
impl ContextEngine for SummaryContextEngine {
    fn info(&self) -> ContextEngineInfo {
        ContextEngineInfo::new(Self::ID, "Summary Context Engine")
    }

    async fn assemble(&self, input: ContextAssembleInput) -> MacacaResult<ContextAssembleResult> {
        let estimated_tokens = estimate_messages_tokens(&input.base_messages);
        let decision = self.compaction_policy.decide(&CompactionTrigger {
            estimated_tokens,
            token_budget: input.budget.input_budget(),
            manual: false,
            focus_topic: None,
        });
        let mut result = self.pruning.assemble(input.clone()).await?;
        result.report.engine_id = Self::ID.into();
        let should_summarize = decision.should_compact && result.messages.len() > 3;
        if should_summarize {
            let sid = result
                .report
                .session_id
                .clone()
                .unwrap_or_else(|| "unknown".into());
            let lineage = SessionLineage::root(&sid);
            let hook_in = CompactionHookInput {
                lineage,
                source_segment_id: format!("segment-{sid}"),
                estimated_tokens,
            };
            self.hooks.before_compaction(hook_in.clone()).await?;
            let envelope = CompactionSummaryEnvelope {
                root_session_id: sid.clone(),
                source_segment_id: "current".into(),
                successor_segment_id: "next".into(),
                resolved: vec!["Earlier context was compacted for budget management.".into()],
                decisions: vec![format!(
                    "Generated by SummaryContextEngine ({}).",
                    decision.reason
                )],
                current_state: "Recent messages are preserved below.".into(),
                open_questions: Vec::new(),
                active_task: "Continue from the latest user request.".into(),
                important_ids_and_paths: Vec::new(),
            };
            let summary_body = envelope.render_reference_only();
            let summary = LlmMessage::system(summary_body.clone());
            let recent_start = result.messages.len().saturating_sub(2);
            let mut compacted = Vec::new();
            if let Some(first) = result.messages.first().cloned() {
                compacted.push(first);
            }
            compacted.push(summary);
            compacted.extend(result.messages[recent_start..].iter().cloned());
            result.messages = compacted;
            result.report.decisions.push(ContextDecisionReport::info(
                "context_summary_inserted",
                "Summary engine inserted a reference-only compaction summary.",
            ));
            let tok = envelope.estimated_tokens();
            result.report.sources.push(
                ContextSourceReport::included(
                    "compaction_summary/current",
                    ContextSourceKind::CompactionSummary,
                    "Reference-only compaction summary",
                    tok,
                    summary_body.len(),
                )
                .with_rendering(
                    "summary",
                    "untrusted",
                    Some("compaction/auto".into()),
                    0,
                ),
            );
            result.report.decisions.push(ContextDecisionReport::info(
                "compaction_hooks",
                decision.reason,
            ));
            self.hooks.after_compaction(hook_in).await?;
        }
        Ok(result)
    }
}

#[derive(Default)]
pub struct ContextEngineRegistry {
    engines: HashMap<String, Arc<dyn ContextEngine>>,
}

impl ContextEngineRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_legacy() -> Self {
        Self::new().register(Arc::new(LegacyContextEngine))
    }

    pub fn with_builtins() -> Self {
        Self::with_legacy()
            .register(Arc::new(WindowedContextEngine::default()))
            .register(Arc::new(
                PruningContextEngine::<DefaultSourceRenderer>::default(),
            ))
            .register(Arc::new(SummaryContextEngine::default()))
    }

    pub fn register(mut self, engine: Arc<dyn ContextEngine>) -> Self {
        let id = engine.info().id;
        self.engines.insert(id, engine);
        self
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn ContextEngine>> {
        self.engines.get(id).cloned()
    }

    pub fn resolve_or_legacy(&self, id: Option<&str>) -> Arc<dyn ContextEngine> {
        id.and_then(|id| self.get(id))
            .or_else(|| self.get(LegacyContextEngine::ID))
            .unwrap_or_else(|| Arc::new(LegacyContextEngine))
    }
}

#[derive(Clone)]
pub struct ContextRuntimeFacade {
    registry: Arc<ContextEngineRegistry>,
    selection: ContextEngineSelection,
}

impl ContextRuntimeFacade {
    pub fn new(registry: ContextEngineRegistry, selection: ContextEngineSelection) -> Self {
        Self {
            registry: Arc::new(registry),
            selection,
        }
    }

    pub fn builtins(selection: ContextEngineSelection) -> Self {
        Self::new(ContextEngineRegistry::with_builtins(), selection)
    }

    pub fn legacy() -> Self {
        Self::builtins(ContextEngineSelection::legacy())
    }

    pub async fn assemble(
        &self,
        input: ContextAssembleInput,
    ) -> MacacaResult<ContextAssembleResult> {
        let engine = self
            .registry
            .resolve_or_legacy(Some(&self.selection.engine_id));
        let requested_primary = self.selection.engine_id.clone();
        match engine.assemble(input.clone()).await {
            Ok(mut result) => {
                result.report.requested_engine_id = requested_primary.clone();
                result.report.engine_fallback_applied = false;
                Ok(result)
            }
            Err(error) => {
                let fallback_id = self.selection.fallback_engine_id.as_str();
                let fallback = self.registry.resolve_or_legacy(Some(fallback_id));
                let mut result = fallback.assemble(input).await?;
                result.report.requested_engine_id = requested_primary;
                result.report.engine_fallback_applied = true;
                result.report.decisions.push(ContextDecisionReport {
                    code: "context_engine_fallback".into(),
                    severity: ContextDecisionSeverity::Warning,
                    message: format!(
                        "Context engine '{}' failed and fallback '{}' was used: {}",
                        self.selection.engine_id, fallback_id, error
                    ),
                });
                Ok(result)
            }
        }
    }
}

#[derive(Clone)]
pub struct ContextManagerFacade {
    engine: Arc<dyn ContextEngine>,
}

impl ContextManagerFacade {
    pub fn new(engine: Arc<dyn ContextEngine>) -> Self {
        Self { engine }
    }

    pub fn legacy() -> Self {
        Self::new(Arc::new(LegacyContextEngine))
    }

    pub fn engine_info(&self) -> ContextEngineInfo {
        self.engine.info()
    }

    pub async fn assemble(
        &self,
        input: ContextAssembleInput,
    ) -> MacacaResult<ContextAssembleResult> {
        self.engine.assemble(input).await
    }

    pub async fn after_turn(&self, input: ContextAfterTurnInput) -> MacacaResult<()> {
        self.engine.after_turn(input).await
    }
}

fn build_legacy_report(input: &ContextAssembleInput) -> ContextReport {
    let mut report =
        build_report_for_messages(LegacyContextEngine::ID, input, &input.base_messages);
    report.decisions.push(ContextDecisionReport::info(
        "legacy_passthrough",
        "Legacy engine preserved incoming messages and options.",
    ));
    report
}

fn build_report_for_messages(
    engine_id: &str,
    input: &ContextAssembleInput,
    messages: &[LlmMessage],
) -> ContextReport {
    let mut builder = ContextReportBuilder::new(LegacyContextEngine::ID)
        .identity(
            input.app_id,
            input.session_id.clone(),
            input.agent_name.clone(),
            input.model.clone(),
        )
        .budget(input.budget)
        .engine(engine_id);

    let mut composer = PromptComposer::new();
    for (idx, message) in messages.iter().enumerate() {
        let kind = message_source_kind(message);
        let source = ContextSourceReport::included(
            format!("message/{idx}"),
            kind.clone(),
            format!("{:?}", message.role),
            estimate_text_tokens(&message.content),
            message.content.len(),
        );
        builder = builder.source(source);

        if matches!(
            kind,
            ContextSourceKind::SystemPrompt | ContextSourceKind::DynamicPrompt
        ) {
            let stability = if kind == ContextSourceKind::SystemPrompt {
                PromptStability::Stable
            } else {
                PromptStability::Dynamic
            };
            composer = composer.push_section(PromptSection {
                id: format!("message/{idx}"),
                kind,
                stability,
                trust_level: TrustLevel::Trusted,
                content: message.content.clone(),
            });
        }
    }

    if let Some(tools) = input.options.tools.as_ref() {
        let bytes = serde_json::to_vec(tools).map(|v| v.len()).unwrap_or(0);
        let token_estimate = serde_json::to_string(tools)
            .map(|text| estimate_text_tokens(&text))
            .unwrap_or(0);
        builder = builder.source(ContextSourceReport::included(
            "tool_schema",
            ContextSourceKind::ToolSchema,
            "LLM tool schema",
            token_estimate,
            bytes,
        ));
    }

    let compiled = composer.compile();
    builder
        .hashes(compiled.stable_hash, compiled.full_hash)
        .build()
}

fn estimate_messages_tokens(messages: &[LlmMessage]) -> u32 {
    messages
        .iter()
        .map(|message| estimate_text_tokens(&message.content))
        .sum()
}

fn trim_to_budget(
    messages: Vec<LlmMessage>,
    budget: ContextBudget,
    preserve_recent: usize,
) -> Vec<LlmMessage> {
    if estimate_messages_tokens(&messages) <= budget.input_budget() || messages.len() <= 3 {
        return messages;
    }
    let mut output = Vec::new();
    let start_idx = if messages
        .first()
        .map(|message| message.role == macaca_proto::LlmRole::System)
        .unwrap_or(false)
    {
        output.push(messages[0].clone());
        1
    } else {
        0
    };
    let recent = preserve_recent.min(messages.len().saturating_sub(start_idx));
    let recent_start = messages.len().saturating_sub(recent);
    if recent_start > start_idx {
        output.push(LlmMessage::system(format!(
            "[{} earlier messages trimmed to fit the context window.]",
            recent_start - start_idx
        )));
    }
    output.extend(messages[recent_start..].iter().cloned());
    output
}

fn message_source_kind(message: &LlmMessage) -> ContextSourceKind {
    match message.role {
        macaca_proto::LlmRole::System => ContextSourceKind::SystemPrompt,
        macaca_proto::LlmRole::Tool => ContextSourceKind::ToolResult,
        macaca_proto::LlmRole::User | macaca_proto::LlmRole::Assistant => {
            ContextSourceKind::History
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::{LlmMessage, LlmOptions, ToolDefinition};

    #[tokio::test]
    async fn legacy_engine_preserves_messages_and_options() {
        let messages = vec![LlmMessage::system("system"), LlmMessage::user("hello")];
        let options = LlmOptions {
            model: "test-model".into(),
            tools: Some(vec![ToolDefinition {
                name: "read".into(),
                description: "read file".into(),
                parameters: serde_json::json!({"type": "object"}),
            }]),
            ..Default::default()
        };

        let result = LegacyContextEngine
            .assemble(ContextAssembleInput::legacy(
                "agent",
                "test-model",
                messages.clone(),
                options.clone(),
            ))
            .await
            .unwrap();

        assert_eq!(result.messages.len(), messages.len());
        assert_eq!(result.options.model, options.model);
        assert_eq!(result.report.engine_id, "legacy");
        assert!(result.report.tool_schema_tokens > 0);
    }

    #[test]
    fn registry_resolves_legacy_by_default() {
        let registry = ContextEngineRegistry::with_legacy();
        assert_eq!(registry.resolve_or_legacy(None).info().id, "legacy");
        assert_eq!(
            registry.resolve_or_legacy(Some("missing")).info().id,
            "legacy"
        );
    }

    #[tokio::test]
    async fn facade_delegates_to_selected_engine() {
        let facade = ContextManagerFacade::legacy();
        let result = facade
            .assemble(ContextAssembleInput::legacy(
                "agent",
                "model",
                vec![LlmMessage::user("hello")],
                LlmOptions::default(),
            ))
            .await
            .unwrap();

        assert_eq!(facade.engine_info().id, "legacy");
        assert_eq!(result.messages.len(), 1);
    }

    #[tokio::test]
    async fn pruning_engine_summarizes_large_tool_result_without_changing_options() {
        let messages = vec![LlmMessage::tool_result("call", "x".repeat(20_000))];
        let options = LlmOptions {
            model: "test-model".into(),
            ..Default::default()
        };

        let result = PruningContextEngine::<DefaultSourceRenderer>::default()
            .assemble(ContextAssembleInput::legacy(
                "agent",
                "test-model",
                messages,
                options.clone(),
            ))
            .await
            .unwrap();

        assert_eq!(result.options.model, options.model);
        assert!(result.messages[0].content.len() < 20_000);
        assert!(result.report.pruned_tokens > 0);
        assert!(result
            .report
            .decisions
            .iter()
            .any(|decision| decision.code == "context_render_excerpt"));
    }

    #[tokio::test]
    async fn runtime_facade_selects_pruning_without_concrete_type() {
        let facade = ContextRuntimeFacade::builtins(ContextEngineSelection {
            engine_id: "pruning".into(),
            fallback_engine_id: "legacy".into(),
        });
        let result = facade
            .assemble(ContextAssembleInput::legacy(
                "agent",
                "model",
                vec![LlmMessage::tool_result("call", "x".repeat(20_000))],
                LlmOptions::default(),
            ))
            .await
            .unwrap();

        assert_eq!(result.report.engine_id, "pruning");
        assert!(result.report.pruned_tokens > 0);
        assert_eq!(result.report.requested_engine_id, "pruning");
        assert!(!result.report.engine_fallback_applied);
    }

    #[tokio::test]
    async fn runtime_facade_sets_requested_engine_when_legacy_succeeds() {
        let facade = ContextRuntimeFacade::builtins(ContextEngineSelection {
            engine_id: "legacy".into(),
            fallback_engine_id: "pruning".into(),
        });
        let result = facade
            .assemble(ContextAssembleInput::legacy(
                "agent",
                "model",
                vec![LlmMessage::user("hello")],
                LlmOptions::default(),
            ))
            .await
            .unwrap();
        assert_eq!(result.report.requested_engine_id, "legacy");
        assert_eq!(result.report.engine_id, "legacy");
        assert!(!result.report.engine_fallback_applied);
    }

    #[tokio::test]
    async fn builtins_include_windowed_and_summary_engines() {
        let registry = ContextEngineRegistry::with_builtins();
        assert_eq!(
            registry.resolve_or_legacy(Some("windowed")).info().id,
            "windowed"
        );
        assert_eq!(
            registry.resolve_or_legacy(Some("summary")).info().id,
            "summary"
        );
    }
}
