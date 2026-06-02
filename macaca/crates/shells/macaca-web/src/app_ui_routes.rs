//! Generic routes for application-owned UI bundles.
//!
//! These handlers implement the host side of the Application-Owned UI Runtime
//! contract.  They are intentionally application-agnostic: static files are
//! served only from manifest-declared package paths, and bridge calls are
//! admitted only through manifest-declared capabilities before being routed to
//! the existing Application Service host-dispatch boundary.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Path as AxumPath, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::Response;
use axum::Json;
use serde_json::Value;
use uuid::Uuid;

use macaca_app::{
    expand_service_capabilities, ui_runtime::AppUiRuntimeConfig, InMemoryDomainPackCatalog,
};
use macaca_proto::{
    ApplicationHostCommand, ApplicationHostCommandResult, ApplicationHostCommandStatus,
    ApplicationHostDispatchServiceCommand, ApplicationId, ApplicationImport,
    ApplicationMetadataQueryCommand, ApplicationServiceScope, ApplicationUiBridgeRequest,
    ApplicationUiBridgeResponse, TraceContext, WASM_HOST_IMPORT_CAPABILITY,
    WASM_HOST_IMPORT_OPERATION, WASM_HOST_IMPORT_SERVICE_ID,
};

use crate::app_ui_csp::app_ui_html_csp;
use crate::app_ui_llm_bridge::hydrate_llm_bridge_payload;
use crate::app_ui_session_projection::AppUiSessionProjection;
use crate::app_ui_workspace_scope::decorate_workspace_scoped_payload;
use crate::routes::{err, proto_err, ErrorResponse};
use crate::state::AppState;

type RouteError = (StatusCode, Json<ErrorResponse>);

/// Serve one file from an installed application-owned web bundle.
///
/// The route validates three separate boundaries before reading from disk:
/// application identity, manifest-declared UI paths, and canonical filesystem
/// containment under the installed app directory.  This keeps the route useful
/// for any application bundle while preventing arbitrary package traversal.
pub async fn get_app_ui_asset(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    AxumPath((app_id, asset_path)): AxumPath<(String, String)>,
) -> Result<Response, RouteError> {
    let app_id = parse_app_id(&app_id)?;
    let asset_path = normalize_package_path(&asset_path)?;
    let context = app_ui_context(&state, app_id).await?;
    let app_dir = context.app_dir;
    let ui = context.ui;
    ensure_declared_asset(&ui, &asset_path)?;
    let body = read_declared_asset(&app_dir, &asset_path).await?;
    let content_type = content_type_for(&asset_path);

    tracing::info!(
        app_id = %app_id,
        asset_path = %asset_path.display(),
        bytes = body.len(),
        content_type,
        "served application-owned UI asset"
    );

    let mut response = Response::new(Body::from(body));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(content_type_for(&asset_path)),
    );
    if asset_path.extension().and_then(|value| value.to_str()) == Some("html") {
        response.headers_mut().insert(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_str(&app_ui_html_csp(&headers)).map_err(|error| {
                err(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("failed to construct application UI CSP: {error}"),
                )
            })?,
        );
    }
    Ok(response)
}

/// Dispatch one generic UI bridge command through Application Service.
///
/// The host does not interpret application business intent.  It validates the
/// bridge envelope, checks that the capability was declared by the manifest,
/// maps the capability to the stable Application ABI import, and lets the
/// runtime/service router make the final provider and policy decisions.
pub async fn post_app_ui_bridge(
    State(state): State<Arc<AppState>>,
    AxumPath(app_id): AxumPath<String>,
    Json(request): Json<ApplicationUiBridgeRequest>,
) -> Result<Json<ApplicationUiBridgeResponse>, RouteError> {
    let app_id = parse_app_id(&app_id)?;
    let context = app_ui_context(&state, app_id).await?;
    let trace = bridge_trace(&request);
    tracing::info!(
        app_id = %app_id,
        trace_id = %trace.trace_id,
        command_id = %request.command_id,
        capability = %request.capability,
        "application-owned UI bridge command received"
    );

    if !context.ui.bridge.declares(&request.capability) {
        tracing::warn!(
            app_id = %app_id,
            trace_id = %trace.trace_id,
            command_id = %request.command_id,
            capability = %request.capability,
            "application-owned UI bridge rejected undeclared capability"
        );
        return Ok(Json(bridge_response(
            &request,
            false,
            rejected_bridge_result("ui bridge capability was not declared", trace),
        )));
    }

    // App-owned UI bundles can use the generic service bridge without the host
    // chat composer.  Once the bridge capability is admitted, persist a small
    // shell-visible session memento so refresh and the left session log can
    // recover the run.  The projection uses only generic bridge routing fields
    // and never inspects application-owned payload data.
    AppUiSessionProjection::new(Arc::clone(&state.persist.session_store))
        .record_bridge_received(&app_id, &request)
        .await
        .map_err(|error| {
            err(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to project application UI bridge session: {error}"),
            )
        })?;

    let result = match request.capability.as_str() {
        "service.call" => {
            dispatch_service_call(
                &state,
                app_id,
                &request,
                trace,
                &context.declared_services,
                context.workspace_root.as_deref(),
            )
            .await?
        }
        _ => rejected_bridge_result(
            "ui bridge capability is not implemented by this host",
            trace,
        ),
    };
    Ok(Json(bridge_response(&request, true, result)))
}

async fn dispatch_service_call(
    state: &Arc<AppState>,
    app_id: ApplicationId,
    request: &ApplicationUiBridgeRequest,
    trace: TraceContext,
    declared_services: &BTreeSet<String>,
    workspace_root: Option<&Path>,
) -> Result<ApplicationHostCommandResult, RouteError> {
    let Some(service_id) = request
        .service_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    else {
        return Ok(rejected_bridge_result(
            "service.call requires service_id",
            trace,
        ));
    };
    let Some(operation) = request
        .operation
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    else {
        return Ok(rejected_bridge_result(
            "service.call requires operation",
            trace,
        ));
    };
    if let Some(result) =
        rejected_undeclared_service_result(service_id.trim(), declared_services, trace.clone())
    {
        return Ok(result);
    }

    let payload = hydrate_llm_bridge_payload(
        app_id,
        request.session_id.as_deref(),
        service_id.trim(),
        operation.trim(),
        &request.payload,
        trace.clone(),
    )
    .map_err(|error| err(StatusCode::INTERNAL_SERVER_ERROR, error))?;
    let payload = decorate_workspace_scoped_payload(
        service_id.trim(),
        operation.trim(),
        payload,
        workspace_root,
    );

    let mut host_command =
        ApplicationHostCommand::with_trace(ApplicationImport::ServiceCall, payload, trace.clone());
    host_command.metadata.insert(
        WASM_HOST_IMPORT_SERVICE_ID.into(),
        service_id.trim().to_string(),
    );
    host_command.metadata.insert(
        WASM_HOST_IMPORT_OPERATION.into(),
        operation.trim().to_string(),
    );
    host_command.metadata.insert(
        WASM_HOST_IMPORT_CAPABILITY.into(),
        request.capability.trim().to_string(),
    );
    if let Some(session_id) = request
        .session_id
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        host_command
            .metadata
            .insert("session_id".into(), session_id.to_string());
    }
    host_command
        .metadata
        .insert("ui.bridge.command_id".into(), request.command_id.clone());
    if let Some(surface_id) = request
        .surface_id
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        host_command
            .metadata
            .insert("ui.surface_id".into(), surface_id.to_string());
    }

    let command = ApplicationHostDispatchServiceCommand {
        trace: trace.clone(),
        scope: ApplicationServiceScope::application(app_id),
        host_command,
    };
    let result = state
        .application_client
        .host_dispatch(command)
        .await
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;
    tracing::info!(
        app_id = %app_id,
        trace_id = %trace.trace_id,
        command_id = %request.command_id,
        service_id,
        operation,
        status = ?result.status,
        "application-owned UI bridge service.call dispatched"
    );
    Ok(result)
}

/// Manifest-derived context needed by application-owned UI routes.
///
/// The route layer keeps this context intentionally small: static asset serving
/// needs the package directory and UI declaration, while bridge calls need the
/// effective service allowlist expanded from the manifest service contract.
/// The expansion uses the same data-only domain pack catalog as runtime policy
/// sync so Web does not invent service semantics or branch on application ids.
struct AppUiRouteContext {
    app_dir: PathBuf,
    ui: AppUiRuntimeConfig,
    declared_services: BTreeSet<String>,
    workspace_root: Option<PathBuf>,
}

async fn app_ui_context(
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
        let registry = state.registry.read().await;
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
        let catalog = InMemoryDomainPackCatalog::with_builtin_defaults();
        let declared_services =
            expand_service_capabilities(app.manifest.service_contract.as_ref(), &catalog).services;
        (ui, declared_services)
    };
    let workspace_root = {
        let workspaces = state.config.app_workspaces.read().await;
        workspaces
            .get(&app_id)
            .map(|workspace| workspace.root.clone())
    };

    Ok(AppUiRouteContext {
        app_dir: PathBuf::from(app_dir),
        ui,
        declared_services,
        workspace_root,
    })
}

fn parse_app_id(value: &str) -> Result<ApplicationId, RouteError> {
    Uuid::parse_str(value).map(ApplicationId).map_err(|error| {
        err(
            StatusCode::BAD_REQUEST,
            format!("invalid application id: {error}"),
        )
    })
}

fn normalize_package_path(value: &str) -> Result<PathBuf, RouteError> {
    let path = Path::new(value.trim_start_matches('/'));
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "asset path must be package-relative".into(),
        ));
    }
    for component in path.components() {
        if matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        ) {
            return Err(err(
                StatusCode::FORBIDDEN,
                "asset path must not escape the application package".into(),
            ));
        }
    }
    Ok(path.to_path_buf())
}

fn ensure_declared_asset(ui: &AppUiRuntimeConfig, asset_path: &Path) -> Result<(), RouteError> {
    let requested = path_to_slash(asset_path);
    if ui.entry.as_deref() == Some(requested.as_str()) {
        return Ok(());
    }
    let allowed = ui.assets.iter().any(|pattern| {
        let prefix = pattern.trim_end_matches("/**").trim_end_matches('/');
        requested == prefix || requested.starts_with(&format!("{prefix}/"))
    });
    if allowed {
        Ok(())
    } else {
        Err(err(
            StatusCode::FORBIDDEN,
            "asset path is not declared by the application UI manifest".into(),
        ))
    }
}

fn rejected_undeclared_service_result(
    service_id: &str,
    declared_services: &BTreeSet<String>,
    trace: TraceContext,
) -> Option<ApplicationHostCommandResult> {
    if declared_services.contains(service_id) {
        return None;
    }
    tracing::warn!(
        service_id,
        trace_id = %trace.trace_id,
        declared_service_count = declared_services.len(),
        "application-owned UI bridge rejected undeclared service call"
    );
    let mut result = rejected_bridge_result(
        "service.call target service is not declared by the application manifest",
        trace,
    );
    result
        .metadata
        .insert("service_id".into(), service_id.to_string());
    Some(result)
}

async fn read_declared_asset(app_dir: &Path, asset_path: &Path) -> Result<Vec<u8>, RouteError> {
    let canonical_root = app_dir.canonicalize().map_err(|error| {
        err(
            StatusCode::NOT_FOUND,
            format!("application directory is unavailable: {error}"),
        )
    })?;
    let candidate = canonical_root.join(asset_path);
    let canonical_candidate = candidate.canonicalize().map_err(|error| {
        err(
            StatusCode::NOT_FOUND,
            format!("application UI asset is unavailable: {error}"),
        )
    })?;
    if !canonical_candidate.starts_with(&canonical_root) {
        return Err(err(
            StatusCode::FORBIDDEN,
            "asset path escaped the application package".into(),
        ));
    }
    tokio::fs::read(canonical_candidate).await.map_err(|error| {
        err(
            StatusCode::NOT_FOUND,
            format!("failed to read application UI asset: {error}"),
        )
    })
}

fn path_to_slash(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(value) => value.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn content_type_for(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
    {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    }
}

fn bridge_trace(request: &ApplicationUiBridgeRequest) -> TraceContext {
    TraceContext::new(
        request
            .trace_id
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(request.command_id.as_str()),
    )
}

fn bridge_response(
    request: &ApplicationUiBridgeRequest,
    accepted: bool,
    result: ApplicationHostCommandResult,
) -> ApplicationUiBridgeResponse {
    ApplicationUiBridgeResponse {
        bridge_version: "macaca.ui.bridge.v1".into(),
        command_id: request.command_id.clone(),
        accepted,
        result,
    }
}

fn rejected_bridge_result(
    reason: impl Into<String>,
    trace: TraceContext,
) -> ApplicationHostCommandResult {
    let reason = reason.into();
    let mut result = ApplicationHostCommandResult {
        status: ApplicationHostCommandStatus::DisabledByPolicy {
            reason: reason.clone(),
        },
        output: Value::Null,
        trace: Some(trace),
        policy: None,
        metadata: Default::default(),
    };
    result
        .metadata
        .insert("reason_code".into(), "ui_bridge_rejected".into());
    result.metadata.insert("reason".into(), reason);
    result
}

#[cfg(test)]
mod tests {
    use macaca_app::ui_runtime::{
        AppUiBridgeConfig, AppUiPresentationConfig, AppUiRuntimeConfig, AppUiRuntimeKind,
        AppUiSandboxConfig, AppUiSurfaceConfig, AppUiThemeConfig,
    };

    use super::*;

    fn ui_config() -> AppUiRuntimeConfig {
        AppUiRuntimeConfig {
            runtime: AppUiRuntimeKind::WebBundle,
            surface: AppUiSurfaceConfig::default(),
            framework: None,
            entry: Some("dist/ui/index.html".into()),
            assets: vec!["dist/ui/assets/**".into()],
            sandbox: AppUiSandboxConfig::default(),
            bridge: AppUiBridgeConfig {
                required: vec!["service.call".into()],
                optional: vec!["trace.emit".into()],
            },
            presentation: AppUiPresentationConfig::default(),
            theme: AppUiThemeConfig::default(),
        }
    }

    #[test]
    fn app_ui_asset_policy_allows_entry_and_declared_asset_prefix() {
        let ui = ui_config();
        assert!(ensure_declared_asset(&ui, Path::new("dist/ui/index.html")).is_ok());
        assert!(ensure_declared_asset(&ui, Path::new("dist/ui/assets/app.js")).is_ok());
    }

    #[test]
    fn app_ui_asset_policy_rejects_undeclared_package_file() {
        let error = ensure_declared_asset(&ui_config(), Path::new("dist/secret.txt"))
            .expect_err("undeclared package files must be rejected");
        assert_eq!(error.0, StatusCode::FORBIDDEN);
    }

    #[test]
    fn app_ui_asset_path_normalization_rejects_escape() {
        let error = normalize_package_path("../secret.txt")
            .expect_err("path traversal must be rejected before filesystem access");
        assert_eq!(error.0, StatusCode::FORBIDDEN);
    }

    #[test]
    fn app_ui_bridge_service_policy_rejects_undeclared_service() {
        let declared = BTreeSet::from(["service.file".to_string()]);
        let result = rejected_undeclared_service_result(
            "service.llm",
            &declared,
            TraceContext::new("test-ui-bridge-service-policy"),
        )
        .expect("undeclared service calls must fail before host dispatch");
        assert!(matches!(
            result.status,
            ApplicationHostCommandStatus::DisabledByPolicy { .. }
        ));
        assert_eq!(
            result.metadata.get("service_id").map(String::as_str),
            Some("service.llm")
        );
    }

    #[test]
    fn app_ui_bridge_service_policy_allows_declared_service() {
        let declared = BTreeSet::from(["service.llm".to_string()]);
        assert!(rejected_undeclared_service_result(
            "service.llm",
            &declared,
            TraceContext::new("test-ui-bridge-service-policy-ok"),
        )
        .is_none());
    }

    #[test]
    fn app_ui_bridge_rejection_is_structured_and_traced() {
        let result =
            rejected_bridge_result("capability denied", TraceContext::new("test-ui-bridge"));
        assert!(matches!(
            result.status,
            ApplicationHostCommandStatus::DisabledByPolicy { .. }
        ));
        assert_eq!(
            result.metadata.get("reason_code").map(String::as_str),
            Some("ui_bridge_rejected")
        );
    }
}
