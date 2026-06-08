//! Shared adapter helpers for Application Service command handlers.
//!
//! **Pattern:** Adapter — translates between provider-neutral proto DTOs and
//! `macaca-app` manifest/runtime projections without embedding application
//! business semantics in the infrastructure layer.

use std::path::Path;

use macaca_app::{app_manifest_to_service_app_view_with_catalog, AppStatus, DiscoveredApp};
use macaca_proto::{
    ApplicationId, ApplicationServiceAppView, ApplicationServiceRuntimeView,
    ApplicationServiceSessionView, PackageRuntimeKind, ServiceError, ServiceResult,
};

/// Deserialize a typed command payload from the generic service envelope.
pub(super) fn decode<T: serde::de::DeserializeOwned>(
    payload: serde_json::Value,
) -> ServiceResult<T> {
    serde_json::from_value(payload)
        .map_err(|error| ServiceError::AdapterFailure(error.to_string()))
}

/// Serialize a typed handler result back into the generic service envelope.
pub(super) fn to_value<T: serde::Serialize>(value: T) -> ServiceResult<serde_json::Value> {
    serde_json::to_value(value).map_err(|error| ServiceError::AdapterFailure(error.to_string()))
}

/// Map lower-level adapter failures into the provider-neutral `ServiceError`.
pub(super) fn service_adapter_error(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::AdapterFailure(error.to_string())
}

/// Build a discovered-app view using the shared domain-pack catalog projection.
pub(super) fn discovered_view(
    app: &DiscoveredApp,
    catalog: &dyn macaca_app::DomainPackCatalog,
) -> ApplicationServiceAppView {
    app_manifest_to_service_app_view_with_catalog(
        &app.manifest,
        Some(&app.path),
        AppStatus::Loaded,
        catalog,
    )
}

/// Build a manifest-backed view for bootstrap/start responses.
pub(super) fn manifest_view(
    manifest: &macaca_app::AppManifest,
    app_dir: Option<&Path>,
    running: bool,
    catalog: &dyn macaca_app::DomainPackCatalog,
) -> ApplicationServiceAppView {
    let status = if running {
        AppStatus::Running
    } else {
        AppStatus::Loaded
    };
    app_manifest_to_service_app_view_with_catalog(manifest, app_dir, status, catalog)
}

/// Fallback view when runtime knows an app id/name but registry lookup is absent.
pub(super) fn minimal_running_view(
    id: ApplicationId,
    name: String,
    status: AppStatus,
) -> ApplicationServiceAppView {
    ApplicationServiceAppView {
        id,
        name,
        version: "unknown".into(),
        description: None,
        entry_agent: None,
        agents: Vec::new(),
        runtime: ApplicationServiceRuntimeView {
            runtime_kind: Some(PackageRuntimeKind::Yaml),
            lifecycle_state: macaca_app::lifecycle_from_app_status(status),
            compatibility_status: format!("{status:?}"),
            app_dir: None,
            skills_dir: None,
        },
        ui: None,
        diagnostics: Vec::new(),
    }
}

/// Resolve the current runtime status for one application id, if known.
pub(super) async fn running_status_for(
    runtime: Option<&std::sync::Arc<macaca_app::AppRuntime>>,
    app_id: &ApplicationId,
) -> AppStatus {
    if let Some(runtime) = runtime {
        for (id, _, status) in runtime.list_apps().await {
            if id == *app_id {
                return status;
            }
        }
    }
    AppStatus::Loaded
}

/// Build a replayable session view from a typed application scope.
pub(super) fn session_view(
    scope: &macaca_proto::ApplicationServiceScope,
    status: &str,
) -> ServiceResult<ApplicationServiceSessionView> {
    let application_id = scope.application_id.ok_or_else(|| {
        ServiceError::AdapterFailure("application session command requires application_id".into())
    })?;
    let session_id = scope.session_id.clone().ok_or_else(|| {
        ServiceError::AdapterFailure("application session command requires session_id".into())
    })?;
    Ok(ApplicationServiceSessionView::new(
        application_id,
        session_id,
        scope.agent_name.clone(),
        status,
    ))
}
