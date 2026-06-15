//! Application shell adapter — service-first application/registry access.
//!
//! Routes and framework adapters must not read `AppRuntime` or `AppRegistry` through
//! `AppState` directly. This module centralizes the **Adapter** pattern for:
//! - primary reads through `SystemApplicationClient`
//! - bounded manifest reads that still require package-local declarations

use std::sync::Arc;

use macaca_host_composition::app::AppLlmConfig;
use macaca_host_composition::app::AppRegistry;
use macaca_proto::{
    ApplicationId, ApplicationServiceScope, ApplicationStatusCommand, TraceContext,
};
use tokio::sync::RwLockReadGuard;

use crate::state::AppState;

/// Borrow the application registry read lock from the composition bundle.
///
/// Callers must prefer `SystemApplicationClient::metadata` when sanitized views
/// contain the required declaration data. This helper is limited to package-
/// local declarations that are not yet projected by Application Service.
pub async fn registry_read_guard(state: &Arc<AppState>) -> RwLockReadGuard<'_, AppRegistry> {
    tracing::trace!("application shell adapter acquiring registry read guard");
    state.composition.registry.read().await
}

/// Count running applications for status surfaces.
pub async fn running_app_count(state: &Arc<AppState>) -> usize {
    let command = ApplicationStatusCommand {
        trace: TraceContext::new("web-shell-adapter-app-count"),
        scope: ApplicationServiceScope::default(),
    };
    match state.application_client.status(command).await {
        Ok(views) => views.len(),
        Err(error) => {
            tracing::warn!(
                error = %error,
                "application shell adapter status failed; returning zero running applications"
            );
            0
        }
    }
}

/// Load manifest-declared LLM defaults from package-local declarations.
pub async fn manifest_llm_config(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
) -> Option<AppLlmConfig> {
    let registry = registry_read_guard(state).await;
    registry
        .get_app(app_id)
        .and_then(|app| app.manifest.llm_config.clone())
}

/// Detect WASM layer from package-local declarations when metadata is absent.
pub async fn is_registry_wasm_layer_app(state: &Arc<AppState>, app_id: &ApplicationId) -> bool {
    let registry = registry_read_guard(state).await;
    registry
        .get_app(app_id)
        .map(|app| app.manifest.layer == macaca_host_composition::app::AppLayer::L2Wasm)
        .unwrap_or(false)
}
