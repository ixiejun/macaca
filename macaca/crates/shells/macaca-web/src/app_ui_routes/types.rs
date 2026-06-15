//! Shared types for application-owned UI route adapters.
//!
//! [`AppUiRouteContext`] bundles manifest-derived data needed by both static
//! asset serving and bridge dispatch without re-querying the application registry.

use std::collections::BTreeSet;
use std::path::PathBuf;

use axum::http::StatusCode;
use axum::Json;

use crate::routes::ErrorResponse;

/// Axum error tuple used by all app-ui route handlers.
pub(crate) type RouteError = (StatusCode, Json<ErrorResponse>);

/// Route-safe projection of manifest UI data needed by Web handlers.
///
/// The full application UI runtime declaration belongs to the application
/// framework. Web routes only need asset admission and bridge capability checks,
/// so the context loader projects the manifest into this small data object
/// before handlers use it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct AppUiRouteRuntime {
    /// Package-relative HTML entry point declared by the application.
    pub entry: Option<String>,
    /// Package-relative asset patterns declared by the application.
    pub assets: Vec<String>,
    /// Required host bridge capabilities declared by the application UI.
    pub bridge_required: Vec<String>,
    /// Optional host bridge capabilities declared by the application UI.
    pub bridge_optional: Vec<String>,
}

impl AppUiRouteRuntime {
    /// Return whether the manifest projection declares a bridge capability.
    pub(crate) fn declares_bridge_capability(&self, capability: &str) -> bool {
        self.bridge_required
            .iter()
            .chain(self.bridge_optional.iter())
            .any(|declared| declared == capability)
    }
}

/// Manifest-derived context needed by application-owned UI routes.
///
/// The route layer keeps this context intentionally small: static asset serving
/// needs the package directory and UI declaration, while bridge calls need the
/// effective service allowlist expanded from the manifest service contract.
/// The expansion uses the same data-only domain pack catalog as runtime policy
/// sync so Web does not invent service semantics or branch on application ids.
pub(crate) struct AppUiRouteContext {
    pub app_dir: PathBuf,
    pub ui: AppUiRouteRuntime,
    pub declared_services: BTreeSet<String>,
    pub workspace_root: Option<PathBuf>,
}
