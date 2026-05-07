//! Server bootstrap primitives for `macaca-web`.

use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::{Any, CorsLayer};

use macaca_proto::MacacaResult;

use crate::state::AppState;
use crate::{chat_orchestrator, genui_routes, loop_manager, metrics, routes, session};

/// Canonical builder for starting the web server.
#[derive(Debug, Clone)]
pub struct WebServerBuilder {
    port: u16,
}

impl Default for WebServerBuilder {
    fn default() -> Self {
        Self { port: 3001 }
    }
}

impl WebServerBuilder {
    /// Create a builder with the default web port.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the port for the API server listener.
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Build all runtime state, bind the listener, and serve requests.
    pub async fn serve(self) -> MacacaResult<()> {
        crate::serve_web_server(self.port).await
    }
}

/// Facade for binding shared runtime state to the HTTP route surface.
pub struct WebRuntimeFacade {
    state: Arc<AppState>,
}

impl WebRuntimeFacade {
    /// Create a facade around fully initialized web state.
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    /// Build the Axum router without changing route paths or handlers.
    pub fn router(self) -> Router {
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        Router::new()
            .route("/metrics", get(metrics::metrics_handler))
            .route("/", get(routes::root_not_found))
            .route("/api/status", get(routes::get_status))
            .route("/api/apps", get(routes::get_apps))
            .route("/api/apps/{id}", get(routes::get_app))
            .route("/api/apps/{id}/agents", get(routes::get_app_agents))
            .route("/api/apps/{id}/skills", get(routes::get_app_skills))
            .route(
                "/api/apps/{id}/agents/stream",
                get(routes::stream_agent_status),
            )
            .route("/api/apps/{id}/sessions", get(session::list_app_sessions))
            .route("/api/apps/reload", post(routes::reload_apps))
            .route("/api/mcp", get(routes::get_mcp_status))
            .route(
                "/api/context/provider-runtime",
                get(routes::get_context_provider_runtime),
            )
            .route("/api/sessions", get(session::list_sessions))
            .route("/api/sessions/{app_id}", get(session::get_session))
            .route(
                "/api/sessions/detail/{session_id}",
                get(session::get_session_by_id),
            )
            .route(
                "/api/sessions/stream/{session_id}",
                get(session::stream_session_events),
            )
            .route("/api/skills", get(routes::get_skills))
            .route("/api/chat/v2", post(chat_orchestrator::post_chat_v2))
            .route("/api/chat/stop", post(chat_orchestrator::post_chat_stop))
            .route("/api/apps/{app_id}/todos", get(routes::list_todos))
            .route(
                "/api/apps/{app_id}/todos/progress",
                get(routes::get_todo_progress),
            )
            .route(
                "/api/apps/{app_id}/todos/claim-diagnostics",
                get(routes::get_todo_claim_diagnostics),
            )
            .route(
                "/api/apps/{app_id}/todos/{agent_name}",
                get(routes::list_agent_todos),
            )
            .route(
                "/api/apps/{app_id}/goals",
                get(routes::list_goals).post(loop_manager::create_goal),
            )
            .route(
                "/api/apps/{app_id}/genui/surface",
                get(genui_routes::get_genui_surface),
            )
            .route(
                "/api/apps/{app_id}/genui/events",
                post(genui_routes::post_genui_event),
            )
            .route(
                "/api/apps/{app_id}/schedules",
                get(routes::list_schedules).post(routes::create_schedule),
            )
            .route(
                "/api/apps/{app_id}/schedules/{id}",
                get(routes::get_schedule).delete(routes::delete_schedule),
            )
            .route(
                "/api/apps/{app_id}/schedules/{id}/toggle",
                axum::routing::put(routes::toggle_schedule),
            )
            .route("/api/sessions/{id}/events", get(routes::get_session_events))
            .route(
                "/api/sessions/{id}/source-artifact",
                get(routes::get_session_source_artifact),
            )
            .route(
                "/api/sessions/{id}/run-trace",
                get(routes::get_session_run_trace),
            )
            .route(
                "/api/sessions/{id}/context-reports",
                get(routes::get_session_context_reports),
            )
            .route(
                "/api/sessions/{id}/compact",
                post(routes::manual_compact_session),
            )
            .route(
                "/api/sessions/{id}/lineage",
                get(routes::get_session_lineage),
            )
            .route("/api/drivers", get(routes::get_drivers))
            .route("/api/drivers/reload", post(routes::reload_drivers))
            .layer(cors)
            .with_state(self.state)
    }
}
