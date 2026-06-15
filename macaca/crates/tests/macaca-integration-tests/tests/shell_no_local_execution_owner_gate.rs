//! Shell local execution-owner boundary gate.
//!
//! Presentation shells may spawn host-local controller tasks as adapters, but
//! they must not own the PlanLoop/WorkerLoop handle registries. Runtime-host is
//! the only allowed owner for in-process shutdown flags and wake handles.

use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    for ancestor in Path::new(env!("CARGO_MANIFEST_DIR")).ancestors() {
        let cargo_toml = ancestor.join("Cargo.toml");
        if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
            if content.contains("[workspace]") {
                return ancestor.to_path_buf();
            }
        }
    }
    panic!("failed to locate Macaca workspace root from CARGO_MANIFEST_DIR")
}

fn read(root: &Path, relative: &str) -> String {
    let path = root.join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

fn exists(root: &Path, relative: &str) -> bool {
    root.join(relative).exists()
}

#[test]
fn web_shell_state_does_not_store_loop_handle_maps() {
    let root = workspace_root();
    let state = read(&root, "crates/shells/macaca-web/src/state.rs");

    for forbidden in [
        "plan_loop_handles",
        "worker_loop_handles",
        "plan_loop_wakers",
        "worker_loop_wakers",
        "PlanLoopWaker",
        "WorkerLoopWaker",
    ] {
        assert!(
            !state.contains(forbidden),
            "web AppState/LoopState must not own local execution loop handle map: {forbidden}"
        );
    }

    assert!(
        state.contains("SessionLoopLocalRuntime"),
        "web shell may keep only the runtime-host local session-loop owner handle"
    );
}

#[test]
fn web_shell_state_does_not_store_execution_control_notification_maps() {
    let root = workspace_root();
    let state = read(&root, "crates/shells/macaca-web/src/state.rs");

    for forbidden in [
        "pub struct ExecutionControlLocalNotification",
        "execution_control_notifications",
        "HashMap<String, ExecutionControlLocalNotification>",
        "resume_tx: mpsc::Sender",
    ] {
        assert!(
            !state.contains(forbidden),
            "web AppState/SessionState must not own execution-control local notification state: {forbidden}"
        );
    }

    assert!(
        state.contains("ExecutionControlLocalNotificationRuntime"),
        "web shell may keep only the runtime-host execution-control local notification owner handle"
    );
}

#[test]
fn runtime_host_owns_execution_control_local_notifications() {
    let root = workspace_root();
    let owner = read(
        &root,
        "crates/runtime/macaca-runtime-host/src/execution_control_local_notification.rs",
    );
    let public_api = read(
        &root,
        "crates/runtime/macaca-runtime-host/src/runtime_host_public_api.rs",
    );

    for required in [
        "pub struct ExecutionControlLocalNotification",
        "pub struct ExecutionControlLocalNotificationRuntime",
        "notifications: RwLock<HashMap",
        "install(",
        "remove_many",
        "notify_resume",
    ] {
        assert!(
            owner.contains(required),
            "runtime-host execution-control local notification owner is missing required surface: {required}"
        );
    }
    assert!(
        public_api.contains("ExecutionControlLocalNotificationRuntime"),
        "runtime-host public API must expose the execution-control local notification owner through the facade"
    );
}

#[test]
fn runtime_host_owns_local_session_loop_handles() {
    let root = workspace_root();
    let owner = read(
        &root,
        "crates/runtime/macaca-runtime-host/src/session_loop_local_runtime.rs",
    );
    let public_api = read(
        &root,
        "crates/runtime/macaca-runtime-host/src/runtime_host_public_api.rs",
    );

    for required in [
        "pub struct SessionLoopLocalRuntime",
        "plan_loop_shutdowns",
        "worker_loop_shutdowns",
        "plan_loop_wakers",
        "worker_loop_wakers",
        "shutdown_application_loops",
        "wake_plan_loop",
        "wake_worker_loops",
    ] {
        assert!(
            owner.contains(required),
            "runtime-host local session-loop owner is missing required surface: {required}"
        );
    }
    assert!(
        public_api.contains("SessionLoopLocalRuntime"),
        "runtime-host public API must expose the local session-loop owner through the facade"
    );
}

#[test]
fn web_cleanup_uses_runtime_host_local_runtime_owner() {
    let root = workspace_root();
    let cleanup = read(
        &root,
        "crates/shells/macaca-web/src/chat_orchestrator/executor_adapter.rs",
    );

    assert!(
        cleanup.contains("shutdown_application_loops"),
        "web cleanup must delegate local loop shutdown to runtime-host"
    );
    for forbidden in [
        "plan_loop_handles.write",
        "worker_loop_handles.write",
        "plan_loop_wakers.write",
        "worker_loop_wakers.write",
    ] {
        assert!(
            !cleanup.contains(forbidden),
            "web cleanup must not mutate local execution loop maps directly: {forbidden}"
        );
    }
}

#[test]
fn web_shell_does_not_own_workspace_memory_runtime() {
    let root = workspace_root();
    let state = read(&root, "crates/shells/macaca-web/src/state.rs");
    let bootstrap_ctx = read(
        &root,
        "crates/shells/macaca-web/src/composition_bootstrap/bootstrap_ctx.rs",
    );
    let service_wiring = read(
        &root,
        "crates/shells/macaca-web/src/composition_bootstrap/service_runtime_wiring.rs",
    );

    assert!(
        !exists(&root, "crates/shells/macaca-web/src/memory_runtime.rs"),
        "web shell must not keep a local memory runtime implementation"
    );
    for source in [state.as_str(), bootstrap_ctx.as_str()] {
        assert!(
            !source.contains("workspace_memory:"),
            "web state/bootstrap must not own workspace memory manager fields"
        );
        assert!(
            !source.contains("TestMemoryManager"),
            "web state/bootstrap must not depend on concrete memory manager ownership"
        );
    }
    assert!(
        service_wiring.contains("FabricMemoryRuntime::from_configured_memory"),
        "web bootstrap must use the service-layer memory runtime adapter"
    );
    assert!(
        service_wiring.contains("ServiceBackedMemoryClient::new"),
        "context recall must use the service-backed SDK memory client"
    );
}
