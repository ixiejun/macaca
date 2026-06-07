//! Path normalization, asset policy, and filesystem **Adapter** helpers.
//!
//! All filesystem reads go through canonical containment checks so declared
//! manifest paths cannot escape the installed application package directory.

use std::path::{Component, Path, PathBuf};

use axum::http::StatusCode;
use macaca_app::ui_runtime::AppUiRuntimeConfig;
use macaca_proto::ApplicationId;
use uuid::Uuid;

use crate::app_ui_routes::types::RouteError;
use crate::routes::err;

/// Parse a UUID application id from the URL path segment.
pub(crate) fn parse_app_id(value: &str) -> Result<ApplicationId, RouteError> {
    Uuid::parse_str(value).map(ApplicationId).map_err(|error| {
        err(
            StatusCode::BAD_REQUEST,
            format!("invalid application id: {error}"),
        )
    })
}

/// Normalize a package-relative asset path and reject traversal escapes.
pub(crate) fn normalize_package_path(value: &str) -> Result<PathBuf, RouteError> {
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

/// Verify the requested asset is declared by the application UI manifest.
pub(crate) fn ensure_declared_asset(ui: &AppUiRuntimeConfig, asset_path: &Path) -> Result<(), RouteError> {
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

/// Read a declared asset after canonical containment checks under `app_dir`.
pub(crate) async fn read_declared_asset(app_dir: &Path, asset_path: &Path) -> Result<Vec<u8>, RouteError> {
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

/// Convert a path to forward-slash form for manifest pattern matching.
pub(crate) fn path_to_slash(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(value) => value.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// Map a file extension to a static MIME type for HTTP responses.
pub(crate) fn content_type_for(path: &Path) -> &'static str {
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
