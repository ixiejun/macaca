//! Shared types for application-owned UI route adapters.
//!
//! [`AppUiRouteContext`] bundles manifest-derived data needed by both static
//! asset serving and bridge dispatch without re-querying the application registry.

use std::collections::BTreeSet;
use std::path::PathBuf;

use axum::http::StatusCode;
use axum::Json;
use macaca_sdk::app::ui_runtime::AppUiRuntimeConfig;

use crate::routes::ErrorResponse;

/// Axum error tuple used by all app-ui route handlers.
pub(crate) type RouteError = (StatusCode, Json<ErrorResponse>);

/// Manifest-derived context needed by application-owned UI routes.
///
/// The route layer keeps this context intentionally small: static asset serving
/// needs the package directory and UI declaration, while bridge calls need the
/// effective service allowlist expanded from the manifest service contract.
/// The expansion uses the same data-only domain pack catalog as runtime policy
/// sync so Web does not invent service semantics or branch on application ids.
pub(crate) struct AppUiRouteContext {
    pub app_dir: PathBuf,
    pub ui: AppUiRuntimeConfig,
    pub declared_services: BTreeSet<String>,
    pub workspace_root: Option<PathBuf>,
}
