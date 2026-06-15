//! Catalog projection helpers for discovered and running application views.
//!
//! **Pattern:** Adapter — reads registry/runtime state owned by `macaca-app` and
//! projects provider-neutral `ApplicationServiceAppView` rows for snapshot and
//! status commands without duplicating manifest semantics in the host layer.

use std::sync::Arc;

use macaca_app::{
    app_manifest_to_service_app_view_with_catalog, AppRegistry, AppRuntime, SharedDomainPackCatalog,
};
use macaca_proto::{ApplicationServiceAppView, ServiceResult};
use tokio::sync::RwLock;

use super::support::{discovered_view, minimal_running_view};
use super::ApplicationSystemServiceProvider;

impl ApplicationSystemServiceProvider {
    /// List discovered applications from the registry with catalog expansion.
    pub(super) async fn discovered_views(
        registry: &Arc<RwLock<AppRegistry>>,
        catalog: &SharedDomainPackCatalog,
    ) -> ServiceResult<Vec<ApplicationServiceAppView>> {
        let registry = registry.read().await;
        Ok(registry
            .list_apps()
            .into_iter()
            .map(|app| discovered_view(&app, catalog.as_ref()))
            .collect())
    }

    /// List running applications from runtime, enriching with registry metadata when present.
    pub(super) async fn running_views(
        runtime: &Arc<AppRuntime>,
        registry: Option<&Arc<RwLock<AppRegistry>>>,
        catalog: &SharedDomainPackCatalog,
    ) -> ServiceResult<Vec<ApplicationServiceAppView>> {
        let apps = runtime.list_apps().await;
        let mut views = Vec::new();
        for (id, name, status) in apps {
            let discovered = if let Some(registry) = registry {
                let guard = registry.read().await;
                guard.get_app(&id).cloned()
            } else {
                None
            };
            let view = discovered
                .as_ref()
                .map(|app| {
                    app_manifest_to_service_app_view_with_catalog(
                        &app.manifest,
                        Some(&app.path),
                        status,
                        catalog.as_ref(),
                    )
                })
                .unwrap_or_else(|| minimal_running_view(id, name, status));
            views.push(view);
        }
        Ok(views)
    }
}
