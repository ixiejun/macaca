//! Context engine contracts and the legacy compatibility implementation.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{ApplicationId, LlmMessage, LlmOptions, MacacaResult, ToolDefinition};
use serde::{Deserialize, Serialize};

use crate::budget::ContextBudget;
use crate::estimate::estimate_text_tokens;
use crate::prompt::{PromptComposer, PromptSection, PromptStability, TrustLevel};
use crate::report::{
    ContextDecisionReport, ContextReport, ContextReportBuilder, ContextSourceKind,
    ContextSourceReport,
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
    let mut builder = ContextReportBuilder::new(LegacyContextEngine::ID)
        .identity(
            input.app_id,
            input.session_id.clone(),
            input.agent_name.clone(),
            input.model.clone(),
        )
        .budget(input.budget)
        .decision(ContextDecisionReport::info(
            "legacy_passthrough",
            "Legacy engine preserved incoming messages and options.",
        ));

    let mut composer = PromptComposer::new();
    for (idx, message) in input.base_messages.iter().enumerate() {
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

fn message_source_kind(message: &LlmMessage) -> ContextSourceKind {
    match message.role {
        macaca_proto::LlmRole::System => ContextSourceKind::SystemPrompt,
        macaca_proto::LlmRole::Tool => ContextSourceKind::Trace,
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
}
