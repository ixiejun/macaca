//! LLM route shell adapter — service-first model routing for the web shell.
//!
//! Web routes and framework runners must not call `LlmRouter` or `LlmProvider` through
//! `AppState`. This adapter implements the **Adapter** + **Facade** pattern:
//! - primary routing via `SystemLlmClient::resolve_route`
//! - provider label via `SystemLlmClient::snapshot`
//! - legacy fallback via `WebShellCompositionBundle` when the service is unavailable

use std::collections::BTreeMap;
use std::sync::Arc;

use macaca_app::AppLlmConfig;
use macaca_llm::{
    LlmPolicyHints, LlmRouteResolveCommand, LlmServiceScope, LlmServiceSnapshotCommand,
    ModelSelection, ModelSelectionRequest, ModelTarget,
};
use macaca_proto::{ApplicationId, TraceContext};
use tracing::{info, warn};

use crate::state::AppState;

/// Resolve the operator-facing primary provider label for status routes.
///
/// The label is derived from the LLM service snapshot when healthy; otherwise the adapter
/// falls back to the bootstrap provider name without inventing routing semantics in routes.
pub async fn status_provider_label(state: &Arc<AppState>) -> String {
    let scope = LlmServiceScope::new(
        ApplicationId::default(),
        "web-status",
        "web-shell",
    )
    .unwrap_or_else(|_| {
        LlmServiceScope::new(
            ApplicationId::default(),
            "web-status",
            "status",
        )
        .expect("static web status LLM scope must be valid")
    });
    let command = LlmServiceSnapshotCommand {
        scope,
        trace: TraceContext::new("web-shell-adapter-llm-status"),
        include_inventory: false,
    };
    match state.llm_client.snapshot(command).await {
        Ok(snapshot) if snapshot.healthy => {
            let label = snapshot
                .providers
                .first()
                .map(|provider| provider.provider_id.clone())
                .or_else(|| snapshot.default_model.clone())
                .unwrap_or_else(|| "service.llm".to_string());
            info!(
                provider_label = %label,
                "llm route shell adapter resolved status label from service snapshot"
            );
            label
        }
        Ok(_) => {
            warn!("llm service snapshot unhealthy; using legacy provider label fallback");
            state.composition.llm.name().into()
        }
        Err(error) => {
            warn!(
                error = %error,
                "llm service snapshot failed; using legacy provider label fallback"
            );
            state.composition.llm.name().into()
        }
    }
}

/// Resolve a framework `ModelSelection` through the LLM service, with router fallback.
pub async fn resolve_model_selection(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    agent_name: &str,
    session_id: Option<&str>,
    request: ModelSelectionRequest,
) -> Result<ModelSelection, String> {
    let scope = LlmServiceScope::new(
        *app_id,
        session_id.unwrap_or("framework-sessionless"),
        agent_name,
    )
    .map_err(|error| error.to_string())?;

    let command = LlmRouteResolveCommand {
        scope,
        trace: TraceContext::new("web-shell-adapter-resolve-model"),
        request_model: request.request_model.clone(),
        agent_model: request.agent_model.clone(),
        app_model: request.app_model.clone(),
        app_provider: request.app_provider.clone(),
        system_model: request.system_model.clone(),
        fallbacks: request.fallbacks.clone(),
        policy: LlmPolicyHints::default(),
    };

    match state.llm_client.resolve_route(command).await {
        Ok(result) => {
            let selection = route_summary_to_model_selection(&result.selected);
            info!(
                app_id = %app_id,
                agent = agent_name,
                session_id = session_id.unwrap_or("framework-sessionless"),
                route_source = selection.source,
                provider = %selection.primary.provider,
                model = %selection.primary.model,
                "llm route shell adapter resolved model selection via service"
            );
            Ok(selection)
        }
        Err(error) => {
            warn!(
                app_id = %app_id,
                agent = agent_name,
                error = %error,
                "llm route shell adapter falling back to legacy router selection"
            );
            state
                .composition
                .llm_router
                .resolve_selection(&request)
                .map_err(|router_error| router_error.to_string())
        }
    }
}

/// Build replayable route metadata for one chat request without capturing prompt content.
pub async fn resolve_request_route_metadata(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: &str,
    agent_name: &str,
    request_model: Option<&str>,
    app_defaults: Option<AppLlmConfig>,
    system_model: Option<String>,
) -> BTreeMap<String, String> {
    let request = ModelSelectionRequest {
        request_model: request_model.map(str::to_owned),
        app_model: app_defaults.as_ref().map(|cfg| cfg.model.clone()),
        app_provider: app_defaults.as_ref().map(|cfg| cfg.provider.clone()),
        system_model,
        ..Default::default()
    };

    let mut metadata = BTreeMap::new();
    if let Some(model) = request_model.filter(|value| !value.trim().is_empty()) {
        metadata.insert("requested_model".into(), model.to_string());
    }

    match resolve_model_selection(state, app_id, agent_name, Some(session_id), request).await {
        Ok(selection) => {
            metadata.insert("provider_id".into(), selection.primary.provider);
            metadata.insert("model".into(), selection.primary.model);
            metadata.insert("source".into(), selection.source.into());
            metadata.insert(
                "fallback_count".into(),
                selection.fallbacks.len().to_string(),
            );
            info!(
                app_id = %app_id,
                session_id,
                source = selection.source,
                fallback_count = selection.fallbacks.len(),
                request_model_present = request_model.is_some(),
                "llm route shell adapter stored request-level route metadata"
            );
        }
        Err(error) => {
            metadata.insert("diagnostic".into(), "route_unavailable".into());
            metadata.insert("diagnostic_detail".into(), error);
            warn!(
                app_id = %app_id,
                session_id,
                request_model_present = request_model.is_some(),
                "llm route shell adapter failed to resolve request route metadata"
            );
        }
    }

    metadata
}

/// Convert a service-owned route summary into the legacy `ModelSelection` shape.
fn route_summary_to_model_selection(
    summary: &macaca_llm::LlmRouteSummary,
) -> ModelSelection {
    ModelSelection {
        primary: ModelTarget {
            provider: summary.provider_id.clone(),
            model: summary.model.clone(),
        },
        fallbacks: summary
            .fallbacks
            .iter()
            .map(|reference| parse_model_target(reference))
            .collect(),
        source: "service.llm",
    }
}

/// Parse `provider:model` fallback references from the service snapshot.
fn parse_model_target(reference: &str) -> ModelTarget {
    let mut parts = reference.splitn(2, ':');
    let provider = parts.next().unwrap_or_default().trim();
    let model = parts.next().unwrap_or_default().trim();
    ModelTarget {
        provider: provider.to_string(),
        model: if model.is_empty() {
            reference.to_string()
        } else {
            model.to_string()
        },
    }
}
