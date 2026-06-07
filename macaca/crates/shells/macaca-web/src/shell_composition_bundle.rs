//! Web shell composition bundle for bootstrap-time provider anchors.
//!
//! P3 thin-shell migration moves direct provider fields out of [`crate::state::AppState`]
//! into this focused bundle. The bundle is constructed once at web bootstrap in
//! `composition_bootstrap::app_state_assembly`
//! and must only be accessed by approved shell adapters — never by route handlers directly.
//!
//! Design pattern: **Composition Root** + **Facade** boundary. `AppState` keeps SDK clients
//! and HTTP/session state; this bundle keeps the last in-process anchors required until every
//! consumer crosses a service client.

use std::sync::Arc;

use macaca_app::{AppRegistry, AppRuntime};
use macaca_sdk::driver::{DriverRegistry, DriverRuntime};
use macaca_sdk::llm::{LlmProvider, LlmRouter};
use macaca_runtime_host::McpRuntimeFacade;

use crate::memory_runtime::WebMemoryRuntime;

/// Bootstrap-owned provider anchors retained during P3 serviceization migration.
///
/// Each field maps to a deprecated `AppState` field removed in task 4.1.3. New production
/// code must not read these fields outside adapter modules listed in the serviceization
/// escape-hatch allowlist.
#[derive(Clone)]
pub struct WebShellCompositionBundle {
    /// Legacy application lifecycle runtime shared with the Application Service provider.
    pub runtime: Arc<AppRuntime>,
    /// Legacy manifest registry shared with the Application Service provider.
    pub registry: Arc<tokio::sync::RwLock<AppRegistry>>,
    /// Primary LLM provider handle used only for bootstrap and adapter fallback.
    pub llm: Arc<dyn LlmProvider>,
    /// In-process LLM router retained until all route resolution uses `SystemLlmClient`.
    pub llm_router: Arc<LlmRouter>,
    /// Optional web-local memory runtime facade for legacy tool wiring.
    pub memory_runtime: Option<Arc<WebMemoryRuntime>>,
    /// MCP runtime facade for skill toolkit registration fallback.
    pub mcp_runtime: Arc<McpRuntimeFacade>,
    /// Driver registry constructed at bootstrap.
    pub driver_registry: Arc<DriverRegistry>,
    /// Driver runtime facade constructed at bootstrap.
    pub driver_runtime: Arc<DriverRuntime>,
}

impl WebShellCompositionBundle {
    /// Construct the composition bundle from bootstrap-local provider handles.
    ///
    /// This constructor does not start services; it only groups handles that already exist
    /// in the web composition root so `AppState` can stay a thin HTTP/session surface.
    pub fn new(
        runtime: Arc<AppRuntime>,
        registry: Arc<tokio::sync::RwLock<AppRegistry>>,
        llm: Arc<dyn LlmProvider>,
        llm_router: Arc<LlmRouter>,
        memory_runtime: Option<Arc<WebMemoryRuntime>>,
        mcp_runtime: Arc<McpRuntimeFacade>,
        driver_registry: Arc<DriverRegistry>,
        driver_runtime: Arc<DriverRuntime>,
    ) -> Self {
        tracing::info!("web shell composition bundle assembled at bootstrap");
        Self {
            runtime,
            registry,
            llm,
            llm_router,
            memory_runtime,
            mcp_runtime,
            driver_registry,
            driver_runtime,
        }
    }
}
