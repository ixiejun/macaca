//! Application UI route context loader.
//!
//! Resolves manifest UI config, declared service allowlist, and optional
//! workspace root through the Application Service client and shell registry.

use std::path::PathBuf;
use std::sync::Arc;

use axum::http::StatusCode;
use macaca_sdk::app::expand_service_capabilities;
use macaca_proto::{
    ApplicationId, ApplicationMetadataQueryCommand, TraceContext,
};

use crate::app_ui_routes::types::{AppUiRouteContext, RouteError};
use crate::routes::{err, proto_err};
use crate::state::AppState;

/// Load manifest-derived UI routing context for one application.
pub(crate) async fn app_ui_context(
    state: &Arc<AppState>,
    app_id: ApplicationId,
) -> Result<AppUiRouteContext, RouteError> {
    let metadata = ApplicationMetadataQueryCommand::application(
        TraceContext::new("web-route-app-ui-metadata"),
        app_id,
    )
    .map_err(|error| proto_err(StatusCode::BAD_REQUEST, &error))?;
    let view = state
        .application_client
        .metadata(metadata)
        .await
        .map_err(|error| proto_err(StatusCode::NOT_FOUND, &error))?;
    let app_dir = view.application.runtime.app_dir.as_deref().ok_or_else(|| {
        err(
            StatusCode::FAILED_DEPENDENCY,
            "application directory is unavailable".into(),
        )
    })?;

    #[allow(deprecated)]
    let (ui, declared_services) = {
        let registry = crate::application_shell_adapter::registry_read_guard(state).await;
        let app = registry.get_app(&app_id).ok_or_else(|| {
            err(
                StatusCode::NOT_FOUND,
                "application is not registered".into(),
            )
        })?;
        let ui = app.manifest.ui.clone().ok_or_else(|| {
            err(
                StatusCode::NOT_FOUND,
                "application UI runtime is not declared".into(),
            )
        })?;
        let declared_services = expand_service_capabilities(
            app.manifest.service_contract.as_ref(),
            state.domain_pack_catalog.as_ref(),
        )
        .services;
        (ui, declared_services)
    };
    let workspace_root = {
        let workspaces = state.config.app_workspaces.read().await;
        workspaces
            .get(&app_id)
            .map(|workspace| workspace.root.clone())
    };

    tracing::debug!(
        app_id = %app_id,
        declared_service_count = declared_services.len(),
        has_workspace = workspace_root.is_some(),
        "loaded application-owned UI route context"
    );

    Ok(AppUiRouteContext {
        app_dir: PathBuf::from(app_dir),
        ui,
        declared_services,
        workspace_root,
    })
}
