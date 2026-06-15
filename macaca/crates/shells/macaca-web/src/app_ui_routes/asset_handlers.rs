//! Static asset handler for application-owned UI bundles.
//!
//! Validates application identity, manifest-declared UI paths, and canonical
//! filesystem containment before reading from the installed package directory.

use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Path as AxumPath, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::Response;

use crate::app_ui_csp::app_ui_html_csp;
use crate::app_ui_routes::adapter::{
    content_type_for, ensure_declared_asset, normalize_package_path, parse_app_id,
    read_declared_asset,
};
use crate::app_ui_routes::context::app_ui_context;
use crate::app_ui_routes::types::RouteError;
use crate::routes::err;
use crate::state::AppState;

/// Serve one file from an installed application-owned web bundle.
///
/// The route validates three separate boundaries before reading from disk:
/// application identity, manifest-declared UI paths, and canonical filesystem
/// containment under the installed app directory. This keeps the route useful
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
        command = "app.ui.asset.serve",
        "served application-owned UI asset"
    );

    let mut response = Response::new(Body::from(body));
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
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
