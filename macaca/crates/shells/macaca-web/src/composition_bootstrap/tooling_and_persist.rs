//! Bootstrap phase 6–9: skills, tools, orchestration, persistence, and audit primitives.
//!
//! Builds the composite tool surface, opens the session store, and registers core execution
//! protocol services before the service bus wires optional providers.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tracing::{error, info};

use macaca_sdk::skill::{ExecutableSkillToolSet, SkillCatalog};
use macaca_sdk::tools::Tool;
use macaca_proto::MacacaResult;
use macaca_sdk::runtime_host::persist::RedbStore;
use crate::orchestration_tools::build_web_tools;

use super::bootstrap_ctx::BootstrapCtx;

/// Run the `tooling-and-persist` bootstrap slice.
pub(crate) async fn run(ctx: &mut BootstrapCtx) -> MacacaResult<()> {
    let config = ctx.config.clone().expect("bootstrap: config");
    let kernel = Arc::clone(ctx.kernel.as_ref().expect("bootstrap: kernel"));
    let service_runtime = Arc::clone(ctx.service_runtime.as_ref().expect("bootstrap: service_runtime"));
    let skills_dirs = ctx.skills_dirs.clone().expect("bootstrap: skills_dirs");

    // 6. Load skill catalog (SKILL.md knowledge skills).
    let mut catalog = SkillCatalog::new();
    for dir in &skills_dirs {
        if dir.exists() {
            match catalog.load_from_directory(&dir).await {
                Ok(n) => info!(count = n, "Knowledge skills loaded into catalog"),
                Err(e) => tracing::warn!("Failed to load knowledge skills: {e}"),
            }
        }
    }


    // 7. Build composite toolset: built-in tools + executable skill tools.
    let mut all_tools: Vec<Box<dyn Tool>> = vec![
        Box::new(macaca_sdk::tools::FileReadTool),
        Box::new(macaca_sdk::tools::FileWriteTool),
        Box::new(macaca_sdk::tools::ShellTool::default()),
    ];

    // Load executable skill tools from all app skills directories.
    for dir in &skills_dirs {
        if dir.exists() {
            let mut skill_tools = ExecutableSkillToolSet::new();
            match skill_tools.load_from_directory(dir).await {
                Ok(n) => {
                    let skill_tools = skill_tools.into_tools();
                    info!(count = n, "Executable skill tools loaded");
                    all_tools.extend(skill_tools);
                }
                Err(e) => tracing::warn!("Failed to load executable skills: {e}"),
            }
        }
    }

    // Load external driver plugins from configured directory.
    // Driver tools are NOT added to the static CompositeToolSet; instead they
    // are aggregated dynamically from `DriverRegistry` in `build_toolkit` so
    // that `/api/drivers/reload` picks up new driver tools at runtime.
    let drivers_dir =
        std::env::var("MACACA_DRIVERS_DIR").unwrap_or_else(|_| config.drivers.directory.clone());
    let driver_registry = Arc::new(macaca_sdk::driver::DriverRegistry::new());
    let driver_runtime = Arc::new(macaca_sdk::driver::DriverRuntime::new(
        drivers_dir.clone(),
        Arc::clone(&driver_registry),
    ));
    if config.drivers.auto_load {
        let report = driver_runtime.load_all().await;
        for entry in &report.entries {
            match entry.status {
                macaca_sdk::driver::DriverLoadStatus::Loaded => {
                    info!(
                        name = %entry.name,
                        tools = entry.tool_count.unwrap_or_default(),
                        "External driver loaded"
                    );
                }
                macaca_sdk::driver::DriverLoadStatus::Failed => {
                    error!(
                        name = %entry.name,
                        error = %entry.error.as_deref().unwrap_or("unknown error"),
                        "Failed to load external driver"
                    );
                }
            }
        }
    }


    // 8. Initialize orchestration tools.
    let fork_to_session: Arc<
        tokio::sync::RwLock<std::collections::HashMap<macaca_proto::ForkId, crate::state::ForkSessionMapping>>,
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
    let session_store_shared: Arc<dyn macaca_sdk::runtime_host::persist::PersistBackend> = session_store_impl.clone();
    let todo_store = Arc::new(macaca_sdk::task::TodoStore::new(Arc::clone(
        &session_store_shared,
    )));
    let event_log = Arc::new(macaca_sdk::runtime_host::persist::EventLog::new(Arc::clone(
        &session_store_impl,
    )));
    // Register the application execution protocol service at the shared host
    // composition root.  Web supplies only generic infrastructure (EventLog and
    // ServiceRuntime); provider strategies remain runtime-host owned and can be
    // registered later without moving execution loops into the presentation
    // shell.
    macaca_sdk::runtime_host::bootstrap_default_application_execution_service(
        Arc::clone(&service_runtime),
        Arc::clone(&event_log),
        "web-startup-application-execution-service",
    )
    .await?;
    macaca_sdk::runtime_host::bootstrap_interaction_service(
        Arc::clone(&service_runtime),
        Arc::clone(&session_store_shared),
        Some(Arc::clone(&event_log)),
        "web-startup-interaction-service",
    )
    .await?;
    macaca_sdk::runtime_host::bootstrap_local_task_service(
        Arc::clone(&service_runtime),
        Arc::clone(&todo_store),
        "web-startup-task-service",
    )
    .await?;
    let entitlement_store: Arc<dyn macaca_sdk::runtime_host::persist::EntitlementStore> =
        Arc::new(macaca_sdk::runtime_host::persist::InMemoryEntitlementStore::new());
    let payment_store: Arc<dyn macaca_sdk::runtime_host::persist::PaymentStore> =
        Arc::new(macaca_sdk::runtime_host::persist::InMemoryPaymentStore::new());
    let entitlement_facade = Arc::new(
        macaca_sdk::runtime_host::EntitlementRuntimeFacade::with_event_log(
            Arc::clone(&entitlement_store),
            Arc::clone(&event_log),
        ),
    );
    let run_tracer = Arc::new(crate::run_trace::RunTracer::new(Arc::clone(&event_log)));
    info!(path = %session_db_path.display(), "Session store initialized");

    ctx.catalog = Some(catalog);
    ctx.tools = Some(tools);
    ctx.driver_registry = Some(driver_registry);
    ctx.driver_runtime = Some(driver_runtime);
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
    ctx.entitlement_facade = Some(entitlement_facade);
    ctx.run_tracer = Some(run_tracer);
    Ok(())
}
