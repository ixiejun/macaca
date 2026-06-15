//! Contract tests for context engines, registry resolution, and runtime facades.
//!
//! Uses neutral fixtures (generic agent/model ids) — no application-specific names.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{LlmMessage, LlmOptions, ToolDefinition};

use crate::source::DefaultSourceRenderer;

use super::facade::{ContextManagerFacade, ContextRuntimeFacade};
use super::helpers::build_passthrough_report;
use super::passthrough::PassthroughContextEngine;
use super::pruning::PruningContextEngine;
use super::registry::ContextEngineRegistry;
use super::types::{
    ContextAssembleInput, ContextAssembleResult, ContextEngine, ContextEngineInfo,
    ContextEngineSelection, ContextOptionsPatch,
};

struct CustomTestEngine;

#[async_trait]
impl ContextEngine for CustomTestEngine {
    fn info(&self) -> ContextEngineInfo {
        ContextEngineInfo::new("custom-test", "Custom Test Engine")
    }

    async fn assemble(
        &self,
        input: ContextAssembleInput,
    ) -> macaca_proto::MacacaResult<ContextAssembleResult> {
        let mut report = build_passthrough_report(&input);
        report.engine_id = "custom-test".into();
        Ok(ContextAssembleResult {
            messages: input.base_messages,
            options: input.options,
            options_patch: ContextOptionsPatch::default(),
            report,
        })
    }
}

#[tokio::test]
async fn passthrough_engine_preserves_messages_and_options() {
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

    let result = PassthroughContextEngine
        .assemble(ContextAssembleInput::unscoped(
            "agent",
            "test-model",
            messages.clone(),
            options.clone(),
        ))
        .await
        .unwrap();

    assert_eq!(result.messages.len(), messages.len());
    assert_eq!(result.options.model, options.model);
    assert_eq!(result.report.engine_id, "passthrough");
    assert!(result.report.tool_schema_tokens > 0);
}

#[test]
fn registry_resolves_passthrough_by_default() {
    let registry = ContextEngineRegistry::with_passthrough();
    assert_eq!(
        registry.resolve_or_passthrough(None).info().id,
        "passthrough"
    );
    assert_eq!(
        registry.resolve_or_passthrough(Some("missing")).info().id,
        "passthrough"
    );
}

#[tokio::test]
async fn facade_delegates_to_selected_engine() {
    let facade = ContextManagerFacade::passthrough();
    let result = facade
        .assemble(ContextAssembleInput::unscoped(
            "agent",
            "model",
            vec![LlmMessage::user("hello")],
            LlmOptions::default(),
        ))
        .await
        .unwrap();

    assert_eq!(facade.engine_info().id, "passthrough");
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
        .assemble(ContextAssembleInput::unscoped(
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
        fallback_engine_id: "passthrough".into(),
    });
    let result = facade
        .assemble(ContextAssembleInput::unscoped(
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
async fn runtime_facade_sets_requested_engine_when_passthrough_succeeds() {
    let facade = ContextRuntimeFacade::builtins(ContextEngineSelection {
        engine_id: "passthrough".into(),
        fallback_engine_id: "pruning".into(),
    });
    let result = facade
        .assemble(ContextAssembleInput::unscoped(
            "agent",
            "model",
            vec![LlmMessage::user("hello")],
            LlmOptions::default(),
        ))
        .await
        .unwrap();
    assert_eq!(result.report.requested_engine_id, "passthrough");
    assert_eq!(result.report.engine_id, "passthrough");
    assert!(!result.report.engine_fallback_applied);
}

#[tokio::test]
async fn builtins_include_windowed_and_summary_engines() {
    let registry = ContextEngineRegistry::with_builtins();
    assert_eq!(
        registry.resolve_or_passthrough(Some("windowed")).info().id,
        "windowed"
    );
    assert_eq!(
        registry.resolve_or_passthrough(Some("summary")).info().id,
        "summary"
    );
}

#[tokio::test]
async fn runtime_facade_can_select_overlay_custom_engine() {
    let overlay = ContextEngineRegistry::new().register(Arc::new(CustomTestEngine));
    let facade = ContextRuntimeFacade::builtins_with_overlay(
        ContextEngineSelection {
            engine_id: "custom-test".into(),
            fallback_engine_id: "passthrough".into(),
        },
        &overlay,
    );
    let result = facade
        .assemble(ContextAssembleInput::unscoped(
            "agent",
            "model",
            vec![LlmMessage::user("hello")],
            LlmOptions::default(),
        ))
        .await
        .unwrap();
    assert_eq!(result.report.engine_id, "custom-test");
    assert_eq!(result.report.requested_engine_id, "custom-test");
    assert!(!result.report.engine_fallback_applied);
}
