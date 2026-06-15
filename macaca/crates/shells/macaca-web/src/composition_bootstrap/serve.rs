//! Thin orchestrator: runs bootstrap phases in order, then binds axum (steps 11–12).
//!
//! This is the only module that knows the full phase sequence; individual phase modules own
//! one bootstrap slice and read/write the shared [`BootstrapCtx`] carrier.

use tracing::info;

use crate::bootstrap::WebRuntimeFacade;
use macaca_proto::MacacaResult;

use super::app_state_assembly;
use super::application_discovery;
use super::bootstrap_ctx::BootstrapCtx;
use super::config_and_kernel;
use super::post_bootstrap_hooks;
use super::service_runtime_wiring;
use super::tooling_and_persist;

/// Load configuration, wire services, assemble `AppState`, and serve HTTP until shutdown.
pub(crate) async fn serve_web_server(port: u16) -> MacacaResult<()> {
    let mut ctx = BootstrapCtx::default();
    config_and_kernel::run(&mut ctx).await?;
    application_discovery::run(&mut ctx).await?;
    tooling_and_persist::run(&mut ctx).await?;
    service_runtime_wiring::run(&mut ctx).await?;
    app_state_assembly::run(&mut ctx).await?;
    post_bootstrap_hooks::run(&mut ctx).await?;

    let state = ctx.app_state.expect("bootstrap: app_state");
    // 11. Build axum router.
    let app = WebRuntimeFacade::new(state).router();

    // 12. Start server.
    let addr = format!("0.0.0.0:{port}");
    info!(addr = %addr, "Macaca OS API server starting");
    println!("\n  Macaca OS API server: http://localhost:{port}\n");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| macaca_proto::MacacaError::Io(e))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| macaca_proto::MacacaError::Io(e))?;

    Ok(())
}
