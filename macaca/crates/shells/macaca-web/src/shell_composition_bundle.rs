//! Web shell composition bundle for bootstrap-time provider anchors.
//!
//! The thin-shell boundary keeps direct provider fields out of route handlers by
//! placing bootstrap-owned anchors in this focused bundle. The bundle is
//! constructed once at web bootstrap in `composition_bootstrap::app_state_assembly`
//! and must only be accessed by approved shell adapters — never by route handlers directly.
//!
//! Design pattern: **Composition Root** + **Facade** boundary. `AppState` keeps SDK clients
//! and HTTP/session state; this bundle keeps in-process anchors that are accessed
//! only by approved adapters.

use std::sync::Arc;

use macaca_host_composition::app::{AppRegistry, AppRuntime};
use macaca_host_composition::llm::{LlmProvider, LlmRouter};
use macaca_host_composition::mcp_runtime::McpRuntimeFacade;

/// Bootstrap-owned provider anchors retained at the Web composition root.
///
/// New production code must not read these fields outside approved adapter
/// modules because provider construction belongs to composition roots, not
/// route handlers.
#[derive(Clone)]
pub struct WebShellCompositionBundle {
    /// Application lifecycle runtime shared with the Application Service provider.
    pub runtime: Arc<AppRuntime>,
    /// Manifest registry shared with the Application Service provider.
    pub registry: Arc<tokio::sync::RwLock<AppRegistry>>,
    /// Primary LLM provider handle used only for bootstrap and adapter fallback.
    pub llm: Arc<dyn LlmProvider>,
    /// In-process LLM router retained until all route resolution uses `SystemLlmClient`.
    pub llm_router: Arc<LlmRouter>,
    /// MCP runtime facade for skill toolkit registration fallback.
    pub mcp_runtime: Arc<McpRuntimeFacade>,
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
        mcp_runtime: Arc<McpRuntimeFacade>,
    ) -> Self {
        tracing::info!("web shell composition bundle assembled at bootstrap");
        Self {
            runtime,
            registry,
            llm,
            llm_router,
            mcp_runtime,
        }
    }
}
