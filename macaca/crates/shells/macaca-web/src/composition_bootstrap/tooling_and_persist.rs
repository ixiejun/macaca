//! Bootstrap phase 6–9: skills, tools, orchestration, persistence, and audit primitives.
//!
//! Builds the composite tool surface, opens the session store, and registers core execution
//! protocol services before the service bus wires optional providers.

use std::path::PathBuf;
use std::sync::Arc;

use tracing::info;

use crate::orchestration_tools::build_web_tools;
use macaca_host_composition::persist::RedbStore;
use macaca_proto::MacacaResult;

use super::bootstrap_ctx::BootstrapCtx;

/// Run the `tooling-and-persist` bootstrap slice.
pub(crate) async fn run(ctx: &mut BootstrapCtx) -> MacacaResult<()> {
    let config = ctx.config.clone().expect("bootstrap: config");
    let kernel = Arc::clone(ctx.kernel.as_ref().expect("bootstrap: kernel"));
    let service_runtime = Arc::clone(
        ctx.service_runtime
            .as_ref()
            .expect("bootstrap: service_runtime"),
    );
    let skills_dirs = ctx.skills_dirs.clone().expect("bootstrap: skills_dirs");

    // 6. Load local Skill assets through runtime-host so the Web shell does not
    // construct Skill catalogs or executable Skill registries directly.
    let skill_assets = macaca_host_composition::service_bootstrap::bootstrap_local_skill_assets(
        &skills_dirs,
        "web-startup-skill-assets",
    )
    .await?;
    info!(
        knowledge_loaded = skill_assets.knowledge_loaded,
        executable_loaded = skill_assets.executable_loaded,
        "Local Skill assets initialized through runtime-host"
    );

    // 7. Build composite toolset: built-in tools + runtime-host supplied skill tools.
    let mut all_tools = macaca_host_composition::tool_bootstrap::bootstrap_local_base_tools(
        "web-startup-base-tools",
    );
    all_tools.extend(skill_assets.executable_tools);

    // Install the external driver service through runtime-host so the Web shell
    // never owns driver registries, runtimes, or provider lifecycle state.
    let drivers_dir =
        std::env::var("MACACA_DRIVERS_DIR").unwrap_or_else(|_| config.drivers.directory.clone());
    let driver_report = macaca_host_composition::service_bootstrap::bootstrap_driver_service(
        Arc::clone(&service_runtime),
        drivers_dir.clone(),
        config.drivers.auto_load,
        "web-startup-driver-service",
    )
    .await?;
    info!(
        drivers_dir = %driver_report.drivers_dir,
        auto_load_attempted = driver_report.auto_load_attempted,
        loaded = driver_report.loaded,
        failed = driver_report.failed,
        "Driver service initialized through runtime-host"
    );

    // 8. Initialize orchestration tools.
    let fork_to_session: Arc<
        tokio::sync::RwLock<
            std::collections::HashMap<macaca_proto::ForkId, crate::state::ForkSessionMapping>,
        >,
    > = Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    let tool_assembly = build_web_tools(
        Arc::clone(&kernel),
        Arc::clone(&service_runtime),
        Arc::clone(&fork_to_session),
        all_tools,
    );
    let tools = tool_assembly.tools;
    let executor_registry_ref = tool_assembly.executor_registry_ref;
    let delegate_session_id = tool_assembly.delegate_session_id;

    // 9. Initialize persistent session store.
    let data_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("macaca");
    std::fs::create_dir_all(&data_dir).ok();
    let session_db_path = data_dir.join("sessions.db");
    let session_store_impl = Arc::new(RedbStore::open(&session_db_path)?);
    let session_store_shared: Arc<dyn macaca_host_composition::persist::PersistBackend> =
        session_store_impl.clone();
    let todo_store = Arc::new(macaca_host_composition::tools::TodoStore::new(Arc::clone(
        &session_store_shared,
    )));
    let event_log = Arc::new(macaca_host_composition::persist::EventLog::new(Arc::clone(
        &session_store_impl,
    )));
    // Register the application execution protocol service at the shared host
    // composition root.  Web supplies only generic infrastructure (EventLog and
    // ServiceRuntime); provider strategies remain runtime-host owned and can be
    // registered later without moving execution loops into the presentation
    // shell.
    macaca_host_composition::service_bootstrap::bootstrap_default_application_execution_service(
        Arc::clone(&service_runtime),
        Arc::clone(&event_log),
        "web-startup-application-execution-service",
    )
    .await?;
    macaca_host_composition::service_bootstrap::bootstrap_interaction_service(
        Arc::clone(&service_runtime),
        Arc::clone(&session_store_shared),
        Some(Arc::clone(&event_log)),
        "web-startup-interaction-service",
    )
    .await?;
    macaca_host_composition::service_bootstrap::bootstrap_local_task_service(
        Arc::clone(&service_runtime),
        Arc::clone(&todo_store),
        "web-startup-task-service",
    )
    .await?;
    let entitlement_store: Arc<dyn macaca_host_composition::persist::EntitlementStore> =
        Arc::new(macaca_host_composition::persist::InMemoryEntitlementStore::new());
    let payment_store: Arc<dyn macaca_host_composition::persist::PaymentStore> =
        Arc::new(macaca_host_composition::persist::InMemoryPaymentStore::new());
    let run_trace_sink = Arc::new(crate::state::StateRunTraceSink::new(Arc::clone(&event_log)));
    let run_tracer = Arc::new(crate::run_trace::RunTracer::new(run_trace_sink));
    info!(path = %session_db_path.display(), "Session store initialized");

    ctx.catalog_entries = Some(skill_assets.catalog_entries);
    ctx.tools = Some(tools);
    ctx.drivers_dir = Some(drivers_dir);
    ctx.fork_to_session = Some(fork_to_session);
    ctx.executor_registry_ref = Some(executor_registry_ref);
    ctx.delegate_session_id = Some(delegate_session_id);
    ctx.data_dir = Some(data_dir);
    ctx.session_db_path = Some(session_db_path);
    ctx.session_store_impl = Some(session_store_impl);
    ctx.session_store_shared = Some(session_store_shared);
    ctx.todo_store = Some(todo_store);
    ctx.event_log = Some(event_log);
    ctx.entitlement_store = Some(entitlement_store);
    ctx.payment_store = Some(payment_store);
    ctx.run_tracer = Some(run_tracer);
    Ok(())
}
