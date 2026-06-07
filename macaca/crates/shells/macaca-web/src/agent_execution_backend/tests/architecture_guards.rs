//! Agent execution backend test module — architecture governance guards.
//!
//! Part of the `agent_execution_backend/tests/` Facade module tree (P3 §4.5.1).
//! Validates execution control, heartbeat evidence, envelope contracts, and
//! architecture governance boundaries without application-specific business logic.


use super::support::*;
#[test]
fn execution_control_selection_does_not_branch_on_application_or_provider_names() {
    let host_adapter_source = include_str!("../../web_agent_execution_adapters.rs");
    let orchestration_source = include_str!(
        "../../../../../runtime/macaca-runtime-host/src/agent_execution_orchestration.rs"
    );
    let runtime_policy_source =
        include_str!("../../../../../runtime/macaca-runtime-host/src/execution_control.rs");
    let service_provider_source = include_str!(
        "../../../../../runtime/macaca-runtime-host/src/execution_control_service_provider.rs"
    );
    let sources = [
        ("agent_execution_orchestration", orchestration_source),
        ("execution_control_policy", runtime_policy_source),
        (
            "execution_control_service_provider",
            service_provider_source,
        ),
    ];
    let forbidden_fragments = [
        "application_id ==",
        "application_id.as_str() ==",
        "target_agent ==",
        "target_agent.as_str() ==",
        "workflow_name ==",
        "workflow_name.as_str() ==",
        "provider_name ==",
        "provider_name.as_str() ==",
        "driver_name ==",
        "driver_name.as_str() ==",
    ];

    for (source_name, source) in sources {
        for fragment in forbidden_fragments {
            assert!(
                !source.contains(fragment),
                "{source_name} must not select execution-control behavior by matching {fragment}"
            );
        }
    }
}

#[test]
fn direct_session_pause_resume_channels_stay_inside_approved_adapters() {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src_root = crate_root.join("src");
    let approved_files = [
        "web_agent_execution_adapters.rs",
        "chat_orchestrator/route_chat_v2.rs",
        "framework_runner/mod.rs",
        "fork_join_shell_adapter.rs",
        "goal_lifecycle_shell_adapter.rs",
        "hook_consumer.rs",
        "loop_manager.rs",
        "state.rs",
    ];
    let mut violations = Vec::new();

    for file in rust_files(&src_root) {
        let relative = file
            .strip_prefix(&src_root)
            .unwrap_or(&file)
            .to_string_lossy()
            .replace('\\', "/");
        // Contract-test modules may reference channel identifiers as scan literals.
        if relative.ends_with("tests.rs") || relative.contains("/tests/") {
            continue;
        }
        let source = std::fs::read_to_string(&file)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", file.display()));
        let owns_session_channel = source.contains("pause_signal")
            || source.contains("resume_tx")
            || source.contains("resume_rx");
        if !owns_session_channel {
            continue;
        }
        let approved = approved_files.iter().any(|allowed| relative == *allowed)
            || relative.starts_with("framework_runner/")
            || relative.starts_with("loop_manager/")
            || relative.starts_with("chat_orchestrator/");
        if !approved {
            violations.push(relative);
        }
    }

    assert!(
        violations.is_empty(),
        "pause/resume session-channel ownership must stay behind service execution adapters: {violations:?}"
    );
}

#[test]
fn legacy_session_pause_resume_paths_are_marked_deprecated() {
    let state_source = include_str!("../../state.rs");
    let hook_consumer_source = include_str!("../../hook_consumer.rs");
    let loop_manager_source = crate::loop_manager::contract_source::loop_manager_module_sources();
    let fork_join_adapter = include_str!("../../fork_join_shell_adapter.rs");
    let goal_lifecycle_adapter = include_str!("../../goal_lifecycle_shell_adapter.rs");

    assert!(state_source.contains("Deprecated compatibility boundary"));
    assert!(state_source.contains("new execution-control ownership should"));
    assert!(hook_consumer_source.contains("service.execution_control"));
    assert!(fork_join_adapter.contains("Deprecated compatibility boundary"));
    assert!(goal_lifecycle_adapter.contains("Deprecated compatibility boundary"));
    assert!(loop_manager_source.contains("ExecutionControlGoalLifecycleCoordinator"));
    assert!(loop_manager_source.contains("goal_lifecycle_shell_adapter"));
}

fn rust_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let entries = std::fs::read_dir(root)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", root.display()));

    for entry in entries {
        let entry = entry.expect("directory entry should be readable");
        let path = entry.path();
        if path.is_dir() {
            files.extend(rust_files(&path));
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(path);
        }
    }
    files
}
