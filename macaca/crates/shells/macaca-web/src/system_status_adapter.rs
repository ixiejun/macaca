//! Runtime-backed SDK status client used by the Web process composition root.
//!
//! The HTTP route depends only on [`macaca_sdk::SystemStatusClient`]. This
//! adapter is the temporary Bridge from the current host-typed [`AppState`] to
//! that provider-neutral SDK contract. Keeping the Bridge behind an injected
//! trait object lets the later host-composition extraction move this file
//! without changing route handlers or response DTOs.

use std::sync::Weak;

use async_trait::async_trait;
use macaca_proto::MacacaResult;
use macaca_sdk::{SystemStatusClient, SystemStatusSnapshot};

use crate::state::AppState;

/// Weak state-backed status Strategy for the running Web process.
///
/// A weak reference avoids creating a self-owning `Arc` cycle because the
/// adapter is installed while `AppState` is assembled with `Arc::new_cyclic`.
/// If state is unavailable during shutdown, the adapter returns a bounded
/// unavailable snapshot instead of panicking or fabricating provider health.
pub(crate) struct WebRuntimeStatusClient {
    state: Weak<AppState>,
}

impl WebRuntimeStatusClient {
    /// Create the status Bridge from the composition root's weak state handle.
    pub(crate) fn new(state: Weak<AppState>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl SystemStatusClient for WebRuntimeStatusClient {
    async fn status_snapshot(&self) -> MacacaResult<SystemStatusSnapshot> {
        let Some(state) = self.state.upgrade() else {
            tracing::warn!(
                operation = "status_snapshot",
                reason_code = "web_state_unavailable",
                "web runtime status adapter could not upgrade process state"
            );
            return Ok(SystemStatusSnapshot {
                version: env!("CARGO_PKG_VERSION").into(),
                agent_count: 0,
                loaded_apps: 0,
                max_agents: 0,
                llm_provider: "service.llm.unavailable".into(),
                app_runtime: "service.application.unavailable".into(),
                gateway_enabled: false,
            });
        };

        tracing::info!(
            operation = "status_snapshot",
            "web runtime status adapter collecting provider-neutral status"
        );
        let agent_count = state.kernel.agent_count().await;
        let loaded_apps = crate::application_shell_adapter::running_app_count(&state).await;
        let llm_provider = crate::llm_route_shell_adapter::status_provider_label(&state).await;

        Ok(SystemStatusSnapshot {
            version: env!("CARGO_PKG_VERSION").into(),
            agent_count,
            loaded_apps,
            // The Web status contract does not expose capacity. Keep this
            // provider-neutral field unset until a service snapshot owns it.
            max_agents: 0,
            llm_provider,
            app_runtime: "service.application".into(),
            gateway_enabled: false,
        })
    }
}
