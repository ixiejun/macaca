//! Application shell adapter — service-first application/registry access.
//!
//! Routes and framework adapters must not read `AppRuntime` or `AppRegistry` through
//! `AppState` directly. This module centralizes the **Adapter** pattern for:
//! - primary reads through `SystemApplicationClient`
//! - bounded legacy fallback through `WebShellCompositionBundle` when service metadata is incomplete

use std::sync::Arc;

use macaca_sdk::app::{model::AppStatus, AppRegistry};
use macaca_sdk::app::AppLlmConfig;
use macaca_proto::{ApplicationId, ApplicationStatusCommand, ApplicationServiceScope, TraceContext};
use tokio::sync::{RwLockReadGuard, RwLockWriteGuard};

use crate::state::AppState;

/// Borrow the legacy registry read lock from the composition bundle.
///
/// This is the only approved production path for raw manifest reads during P3 migration.
/// Callers must prefer `SystemApplicationClient::metadata` when sanitized views suffice.
pub async fn registry_read_guard(
    state: &Arc<AppState>,
) -> RwLockReadGuard<'_, AppRegistry> {
    tracing::trace!("application shell adapter acquiring legacy registry read guard");
    state.composition.registry.read().await
}

/// Borrow the legacy registry write lock from the composition bundle.
///
/// Write access is restricted to approved reload/migration paths such as
/// application discovery fallback when the Application Service is unavailable.
pub async fn registry_write_guard(
    state: &Arc<AppState>,
) -> RwLockWriteGuard<'_, AppRegistry> {
    tracing::trace!("application shell adapter acquiring legacy registry write guard");
    state.composition.registry.write().await
}

/// Reload the legacy on-disk application registry during service outage fallback.
pub async fn reload_legacy_registry(state: &Arc<AppState>) -> Result<usize, String> {
    tracing::info!("application shell adapter reloading legacy registry via composition bundle");
    let mut registry = registry_write_guard(state).await;
    registry.reload().map_err(|error| error.to_string()).map(|apps| apps.len())
}

/// List applications from the legacy runtime anchor held in the composition bundle.
///
/// Routes that can serve from Application Service status should do so directly; this helper
/// exists only for approved fallback paths that still need `AppRuntime::list_apps` semantics.
pub async fn list_runtime_apps(
    state: &Arc<AppState>,
) -> Vec<(ApplicationId, String, AppStatus)> {
    tracing::trace!("application shell adapter listing apps via legacy runtime anchor");
    state.composition.runtime.list_apps().await
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
                "application shell adapter status failed; using legacy runtime count"
            );
            state.composition.runtime.list_apps().await.len()
        }
    }
}

/// Resolve agent ids for one application via the legacy runtime anchor.
pub async fn app_agent_ids(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
) -> Result<Vec<macaca_proto::AgentId>, String> {
    tracing::trace!(
        app_id = %app_id,
        "application shell adapter resolving agent ids via legacy runtime anchor"
    );
    state
        .composition
        .runtime
        .app_agents(app_id)
        .await
        .map_err(|error| error.to_string())
}

/// Load manifest-declared LLM defaults from the legacy registry when metadata lacks them.
pub async fn manifest_llm_config(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
) -> Option<AppLlmConfig> {
    let registry = registry_read_guard(state).await;
    registry
        .get_app(app_id)
        .and_then(|app| app.manifest.llm_config.clone())
}

/// Compatibility fallback for wasm layer detection when metadata runtime-kind is absent.
pub async fn is_registry_wasm_layer_app(state: &Arc<AppState>, app_id: &ApplicationId) -> bool {
    let registry = registry_read_guard(state).await;
    registry
        .get_app(app_id)
        .map(|app| app.manifest.layer == macaca_sdk::app::AppLayer::L2Wasm)
        .unwrap_or(false)
}
