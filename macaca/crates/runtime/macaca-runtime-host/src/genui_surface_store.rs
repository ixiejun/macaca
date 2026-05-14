//! Shared GenUI surface repository for Application Service and WASM host imports.
//!
//! The store is intentionally small and application-agnostic.  It implements a
//! Repository-style boundary for schema-defined UI intents so execution hosts can
//! record the latest declarative surface, while shells query that surface through
//! the Application Service without learning anything about concrete apps.

use std::collections::HashMap;
use std::sync::Arc;

use macaca_proto::{ServiceError, ServiceResult, UiIntent, UiRenderError};
use tokio::sync::RwLock;
use tracing::info;

type GenUiSurfaceKey = (String, String, String);

/// Thread-safe in-memory repository for the latest GenUI surface per app/session.
#[derive(Debug, Clone, Default)]
pub struct ApplicationGenUiSurfaceStore {
    surfaces: Arc<RwLock<HashMap<GenUiSurfaceKey, UiIntent>>>,
}

impl ApplicationGenUiSurfaceStore {
    /// Store a validated UI intent under its app/session/surface identity.
    ///
    /// The key is derived only from protocol fields.  This keeps the repository
    /// generic and makes every write traceable without app-name or workflow-name
    /// branching in the platform.
    pub async fn store(&self, intent: UiIntent) -> ServiceResult<()> {
        validate_scope(&intent)?;
        let key = surface_key(
            &intent.app_id,
            &intent.session_id,
            Some(intent.surface_id.as_str()),
        );
        let default_key = surface_key(&intent.app_id, &intent.session_id, None);
        info!(
            app_id = %intent.app_id,
            session_id = %intent.session_id,
            surface_id = %intent.surface_id,
            trace_id = intent.trace.as_ref().map(|value| value.trace_id.as_str()).unwrap_or("none"),
            root_component = %intent.tree.root.id,
            "Stored GenUI surface intent"
        );
        let mut surfaces = self.surfaces.write().await;
        surfaces.insert(key.clone(), intent.clone());
        if key != default_key {
            // A session usually has a single active presentation surface.  The
            // explicit key preserves precise replay/inspection, while the
            // default key lets generic shells fetch the latest session surface
            // without learning app-specific surface identifiers.
            surfaces.insert(default_key, intent);
        }
        Ok(())
    }

    /// Return the current UI intent for an app/session/surface key.
    ///
    /// Missing entries are represented as `Ok(None)` because an empty surface is
    /// a valid runtime state, not an infrastructure error.
    pub async fn get(
        &self,
        app_id: &str,
        session_id: &str,
        surface_id: Option<&str>,
    ) -> ServiceResult<Option<UiIntent>> {
        require_non_empty(app_id, UiRenderError::MissingApplicationId)?;
        require_non_empty(session_id, UiRenderError::MissingSessionId)?;
        let key = surface_key(app_id, session_id, surface_id);
        let surface = self.surfaces.read().await.get(&key).cloned();
        info!(
            app_id,
            session_id,
            surface_id = key.2.as_str(),
            found = surface.is_some(),
            "Queried GenUI surface intent"
        );
        Ok(surface)
    }

    /// Clear all stored surfaces during provider cleanup.
    pub async fn clear(&self) {
        self.surfaces.write().await.clear();
        info!("Cleared GenUI surface store");
    }
}

fn validate_scope(intent: &UiIntent) -> ServiceResult<()> {
    require_non_empty(&intent.app_id, UiRenderError::MissingApplicationId)?;
    require_non_empty(&intent.session_id, UiRenderError::MissingSessionId)?;
    if intent.trace.is_none() {
        return Err(ServiceError::AdapterFailure(
            UiRenderError::MissingTraceContext.to_string(),
        ));
    }
    Ok(())
}

fn require_non_empty(value: &str, error: UiRenderError) -> ServiceResult<()> {
    if value.trim().is_empty() {
        return Err(ServiceError::AdapterFailure(error.to_string()));
    }
    Ok(())
}

fn surface_key(app_id: &str, session_id: &str, surface_id: Option<&str>) -> GenUiSurfaceKey {
    (
        app_id.to_string(),
        session_id.to_string(),
        surface_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("default")
            .to_string(),
    )
}
