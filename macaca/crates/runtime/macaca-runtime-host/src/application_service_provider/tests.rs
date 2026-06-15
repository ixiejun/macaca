//! Contract tests for Application Service provider behavior.
//!
//! **Pattern:** Contract Test — validates GenUI surface replay and agent
//! delegation unavailable/backend paths through the public `SystemService` API.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    ApplicationAgentDelegateCommand, ApplicationAgentDelegateResult,
    ApplicationGenUiSurfaceCommand, ApplicationId, ApplicationServiceScope, ServiceCommand,
    ServiceCommandName, TraceContext, UiComponent, UiComponentKind, UiComponentTree, UiIntent,
    UiRenderSurface, APPLICATION_AGENT_DELEGATE_COMMAND, APPLICATION_GENUI_SURFACE_COMMAND,
};
use serde_json::json;

use super::{ApplicationOrchestrationBackend, ApplicationSystemServiceProvider as Provider};

struct FakeOrchestrationBackend;

#[async_trait]
impl ApplicationOrchestrationBackend for FakeOrchestrationBackend {
    async fn delegate_agent(
        &self,
        command: ApplicationAgentDelegateCommand,
    ) -> macaca_proto::ServiceResult<ApplicationAgentDelegateResult> {
        Ok(ApplicationAgentDelegateResult {
            application_id: command.scope.application_id.unwrap(),
            session_id: command.scope.session_id.unwrap(),
            target_agent: command.target_agent,
            task_id: Some("task-from-fake-backend".into()),
            success: true,
            output: json!({"status": "queued"}),
            status: "queued".into(),
            metadata: BTreeMap::from([("reason_code".into(), "delegate_queued".into())]),
        })
    }
}

fn card_intent(app_id: ApplicationId, session_id: &str, surface_id: &str) -> UiIntent {
    let trace = TraceContext::new("test-genui-render");
    UiIntent {
        app_id: app_id.to_string(),
        session_id: session_id.to_string(),
        surface_id: UiRenderSurface::new(surface_id),
        tree: UiComponentTree {
            root: UiComponent::new(
                "session-card",
                UiComponentKind::Card,
                json!({
                    "title": "Session Surface",
                    "body": "Schema-defined session card"
                }),
            ),
            trace_markers: Vec::new(),
            metadata: Default::default(),
        },
        permission_prompts: Vec::new(),
        approval_prompts: Vec::new(),
        trace: Some(trace),
        metadata: Default::default(),
    }
}

#[tokio::test]
async fn genui_surface_query_returns_stored_session_surface() {
    let provider = Provider::unavailable();
    let app_id = ApplicationId::new();
    let session_id = "session-genui-test";
    let surface_id = "session-surface";
    let intent = card_intent(app_id, session_id, surface_id);

    provider
        .store_genui_surface(intent.clone())
        .await
        .expect("test intent should store");

    let command = ApplicationGenUiSurfaceCommand {
        trace: TraceContext::new("test-genui-query"),
        scope: ApplicationServiceScope {
            application_id: Some(app_id),
            application_name: None,
            session_id: Some(session_id.to_string()),
            agent_name: None,
        },
        surface_id: Some(surface_id.to_string()),
    };
    let result = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new(APPLICATION_GENUI_SURFACE_COMMAND),
            serde_json::to_value(command).unwrap(),
            TraceContext::new("test-genui-query"),
        ))
        .await
        .unwrap();
    assert_eq!(result.trace.trace_id, "test-genui-query");
    let decoded: Option<UiIntent> = serde_json::from_value(result.output).unwrap();

    assert_eq!(decoded, Some(intent));
}

#[tokio::test]
async fn genui_surface_query_without_surface_id_returns_latest_session_surface() {
    let provider = Provider::unavailable();
    let app_id = ApplicationId::new();
    let session_id = "session-genui-default-test";
    let intent = card_intent(app_id, session_id, "non-default-surface");

    provider
        .store_genui_surface(intent.clone())
        .await
        .expect("test intent should store");

    let command = ApplicationGenUiSurfaceCommand {
        trace: TraceContext::new("test-genui-default-query"),
        scope: ApplicationServiceScope {
            application_id: Some(app_id),
            application_name: None,
            session_id: Some(session_id.to_string()),
            agent_name: None,
        },
        surface_id: None,
    };
    let result = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new(APPLICATION_GENUI_SURFACE_COMMAND),
            serde_json::to_value(command).unwrap(),
            TraceContext::new("test-genui-default-query"),
        ))
        .await
        .unwrap();
    let decoded: Option<UiIntent> = serde_json::from_value(result.output).unwrap();

    assert_eq!(decoded, Some(intent));
}

#[tokio::test]
async fn agent_delegate_without_backend_returns_structured_unavailable() {
    let provider = Provider::unavailable();
    let app_id = ApplicationId::new();
    let command = ApplicationAgentDelegateCommand {
        trace: TraceContext::new("test-agent-delegate-unavailable"),
        scope: ApplicationServiceScope {
            application_id: Some(app_id),
            application_name: None,
            session_id: Some("session-agent-unavailable".into()),
            agent_name: Some("wasm-guest".into()),
        },
        target_agent: "analyst".into(),
        prompt: "Analyze BTC".into(),
        context: json!({"symbol": "BTC"}),
        metadata: BTreeMap::new(),
    };

    let result = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new(APPLICATION_AGENT_DELEGATE_COMMAND),
            serde_json::to_value(command).unwrap(),
            TraceContext::new("test-agent-delegate-unavailable"),
        ))
        .await
        .unwrap();
    let decoded: ApplicationAgentDelegateResult = serde_json::from_value(result.output).unwrap();

    assert!(!decoded.success);
    assert_eq!(decoded.status, "unavailable");
    assert_eq!(
        decoded.metadata.get("reason_code").map(String::as_str),
        Some("orchestration_backend_unavailable")
    );
}

#[tokio::test]
async fn agent_delegate_uses_injected_backend() {
    let provider =
        Provider::unavailable().with_orchestration_backend(Arc::new(FakeOrchestrationBackend));
    let app_id = ApplicationId::new();
    let command = ApplicationAgentDelegateCommand {
        trace: TraceContext::new("test-agent-delegate-backend"),
        scope: ApplicationServiceScope {
            application_id: Some(app_id),
            application_name: None,
            session_id: Some("session-agent-backend".into()),
            agent_name: Some("wasm-guest".into()),
        },
        target_agent: "analyst".into(),
        prompt: "Analyze BTC".into(),
        context: json!({"symbol": "BTC"}),
        metadata: BTreeMap::new(),
    };

    let result = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new(APPLICATION_AGENT_DELEGATE_COMMAND),
            serde_json::to_value(command).unwrap(),
            TraceContext::new("test-agent-delegate-backend"),
        ))
        .await
        .unwrap();
    let decoded: ApplicationAgentDelegateResult = serde_json::from_value(result.output).unwrap();

    assert!(decoded.success);
    assert_eq!(decoded.task_id.as_deref(), Some("task-from-fake-backend"));
    assert_eq!(decoded.status, "queued");
}
