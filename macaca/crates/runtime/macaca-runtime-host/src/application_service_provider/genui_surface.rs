//! GenUI surface Repository boundary for Application Service.
//!
//! **Pattern:** Repository/Memento — WASM guests and future application
//! runtimes emit declarative UI intent data, while Application Service owns
//! the replayable lookup surface consumed by Web, Desktop, CLI inspectors, or
//! audit tooling.

use macaca_proto::{ApplicationGenUiSurfaceCommand, ServiceError, ServiceResult, UiIntent};

use super::ApplicationSystemServiceProvider;

impl ApplicationSystemServiceProvider {
    /// Store the latest validated GenUI intent for one app/session/surface.
    ///
    /// Exposed to unit tests only so contract tests can seed replayable state
    /// without inventing shell-specific storage maps.
    #[cfg(test)]
    pub(crate) async fn store_genui_surface(&self, intent: UiIntent) -> ServiceResult<()> {
        tracing::info!(
            app_id = %intent.app_id,
            session_id = %intent.session_id,
            surface_id = %intent.surface_id,
            trace_id = intent.trace.as_ref().map(|trace| trace.trace_id.as_str()).unwrap_or("none"),
            root_component = %intent.tree.root.id,
            "application service stored GenUI session surface"
        );
        self.genui_surfaces.store(intent).await
    }

    /// Query the latest GenUI intent for one app/session/surface.
    ///
    /// A missing value is a first-class `None`, not an error.  That preserves
    /// the existing chat shell fallback for applications that have not emitted
    /// a declarative surface yet.
    pub(super) async fn get_genui_surface(
        &self,
        command: &ApplicationGenUiSurfaceCommand,
    ) -> ServiceResult<Option<UiIntent>> {
        let app_id = command.scope.application_id.ok_or_else(|| {
            ServiceError::AdapterFailure("application.genui.surface requires application_id".into())
        })?;
        let session_id = command
            .scope
            .session_id
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                ServiceError::AdapterFailure("application.genui.surface requires session_id".into())
            })?;
        let surface_id = command
            .surface_id
            .as_deref()
            .filter(|value| !value.trim().is_empty());
        let surface = self
            .genui_surfaces
            .get(&app_id.to_string(), session_id, surface_id)
            .await?;
        tracing::info!(
            trace_id = %command.trace.trace_id,
            app_id = %app_id,
            session_id,
            surface_id = surface_id.unwrap_or("default"),
            found = surface.is_some(),
            "application service queried GenUI session surface"
        );
        Ok(surface)
    }
}
