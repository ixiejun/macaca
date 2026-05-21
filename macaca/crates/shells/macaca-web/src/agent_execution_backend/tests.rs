use super::*;
use std::path::{Path, PathBuf};

use macaca_proto::{
    AgentContextBuildCommand, AgentContextSnapshot, AgentExecutionIntent,
    ExecutionControlCheckpointMode, ExecutionControlPolicyOverride,
    ExecutionControlResolutionStatus, ExecutionControlResumeSource, ExecutionControlTrigger,
};

#[test]
fn chat_main_thread_uses_deprecated_compat_execution_control_policy() {
    let command = AgentExecutionCommand::new(
        macaca_proto::ApplicationId::from_name("demo"),
        "session-a",
        "coordinator",
        macaca_proto::AgentExecutionIntent::ChatMainThread,
        "create a project goal",
        macaca_proto::TraceContext::new("trace-chat-main-thread"),
    )
    .unwrap();

    let resolution = WebAgentExecutionBackend::resolve_execution_control_policy(&command);

    assert_eq!(resolution.status, ExecutionControlResolutionStatus::Enabled);
    assert_eq!(
        resolution.metadata.get("compatibility"),
        Some(&"legacy_chat_main_thread_goal_pause".to_string())
    );
}

#[test]
fn delegated_runtime_execution_without_policy_does_not_install_execution_control() {
    let command = AgentExecutionCommand::new(
        macaca_proto::ApplicationId::from_name("demo"),
        "session-a",
        "backend",
        macaca_proto::AgentExecutionIntent::TaskWorker,
        "implement the assigned task",
        macaca_proto::TraceContext::new("trace-task-worker"),
    )
    .unwrap();

    let resolution = WebAgentExecutionBackend::resolve_execution_control_policy(&command);

    assert_eq!(
        resolution.status,
        ExecutionControlResolutionStatus::Disabled
    );
}

#[test]
fn delegated_runtime_execution_with_override_installs_execution_control() {
    let command = AgentExecutionCommand::new(
        macaca_proto::ApplicationId::from_name("demo"),
        "session-a",
        "backend",
        macaca_proto::AgentExecutionIntent::TaskWorker,
        "implement the assigned task",
        macaca_proto::TraceContext::new("trace-task-worker-control"),
    )
    .unwrap()
    .with_execution_control_override(ExecutionControlPolicyOverride::enable_for_run(
        vec![ExecutionControlTrigger::tool_call_barrier("create_goal")],
        vec![ExecutionControlResumeSource::goal_lifecycle()],
        ExecutionControlCheckpointMode::ReferenceOnly,
    ));

    let resolution = WebAgentExecutionBackend::resolve_execution_control_policy(&command);

    assert_eq!(resolution.status, ExecutionControlResolutionStatus::Enabled);
}

#[test]
fn heartbeat_intent_requires_heartbeat_source_evidence() {
    let command = AgentExecutionCommand::new(
        macaca_proto::ApplicationId::from_name("demo"),
        "session-a",
        "coordinator",
        AgentExecutionIntent::Heartbeat,
        "run heartbeat work",
        macaca_proto::TraceContext::new("trace-heartbeat-missing-profile"),
    )
    .unwrap();
    let context_command = AgentContextBuildCommand::from_execution(&command);
    let snapshot = AgentContextSnapshot::minimal(&context_command, "trusted context");

    assert!(WebAgentExecutionBackend::should_skip_heartbeat_without_source(&command, &snapshot));
}

#[test]
fn heartbeat_intent_runs_when_heartbeat_source_evidence_exists() {
    let command = AgentExecutionCommand::new(
        macaca_proto::ApplicationId::from_name("demo"),
        "session-a",
        "coordinator",
        AgentExecutionIntent::Heartbeat,
        "run heartbeat work",
        macaca_proto::TraceContext::new("trace-heartbeat-profile-present"),
    )
    .unwrap();
    let context_command = AgentContextBuildCommand::from_execution(&command);
    let mut snapshot = AgentContextSnapshot::minimal(&context_command, "trusted context");
    snapshot.sources.push(macaca_proto::AgentContextSource {
        kind: "profile_file".into(),
        name: "HEARTBEAT.md".into(),
        location: Some("personas/coordinator/HEARTBEAT.md".into()),
        metadata: Default::default(),
    });

    assert!(!WebAgentExecutionBackend::should_skip_heartbeat_without_source(&command, &snapshot));
}

#[test]
fn non_heartbeat_intents_do_not_require_heartbeat_source_evidence() {
    let command = AgentExecutionCommand::new(
        macaca_proto::ApplicationId::from_name("demo"),
        "session-a",
        "coordinator",
        AgentExecutionIntent::TaskWorker,
        "run task work",
        macaca_proto::TraceContext::new("trace-task-no-heartbeat-profile"),
    )
    .unwrap();
    let context_command = AgentContextBuildCommand::from_execution(&command);
    let snapshot = AgentContextSnapshot::minimal(&context_command, "trusted context");

    assert!(!WebAgentExecutionBackend::should_skip_heartbeat_without_source(&command, &snapshot));
}

#[test]
fn execution_control_scope_carries_required_trace_and_run_identity() {
    let command = AgentExecutionCommand::new(
        macaca_proto::ApplicationId::from_name("demo"),
        "session-a",
        "backend",
        macaca_proto::AgentExecutionIntent::TaskWorker,
        "implement the assigned task",
        macaca_proto::TraceContext::new("trace-task-worker-control"),
    )
    .unwrap();
    let execution_id = WebAgentExecutionBackend::execution_control_execution_id(&command);

    let scope =
        WebAgentExecutionBackend::execution_control_scope(&command, execution_id.clone()).unwrap();

    assert_eq!(scope.execution_id, execution_id);
    assert_eq!(scope.session_id, "session-a");
    assert_eq!(scope.trace.trace_id, "trace-task-worker-control");
    assert_eq!(scope.source, "macaca.web.agent_execution");
    assert_eq!(scope.metadata.get("target_agent"), Some(&"backend".into()));
}

#[test]
fn execution_backend_registers_enabled_policy_before_adapter_install() {
    let source = include_str!("../agent_execution_backend.rs");

    assert!(source.contains("register_execution_control(&command, control_policy)"));
    assert!(source.contains("ExecutionControlRegisterExecutionCommand"));
    assert!(source.contains("install_execution_control(&command, policy, execution_id)"));
}

#[test]
fn execution_backend_consumes_context_snapshot_without_rebuilding_context() {
    let source = include_str!("../agent_execution_backend.rs");
    let service_context_call = "build_context_snapshot";
    let snapshot_runtime_builder = "build_runtime_agent_from_context_snapshot";
    let legacy_runtime_builder = ["FrameworkRunner::build_runtime", "_agent("].concat();

    assert!(source.contains(service_context_call));
    assert!(source.contains(snapshot_runtime_builder));
    assert!(!source.contains(&legacy_runtime_builder));
}

#[test]
fn execution_backend_returns_context_snapshot_for_audit_replay() {
    let source = include_str!("../agent_execution_backend.rs");

    assert!(source.contains("result.context_snapshot = Some(context_snapshot)"));
    assert!(source.contains("AgentExecutionResult::completed"));
    assert!(source.contains("AgentExecutionStatus::Failed"));
    assert!(source.contains("ServiceBusSource::new(\"macaca.web.agent_execution\")"));
}

#[test]
fn execution_control_selection_does_not_branch_on_application_or_provider_names() {
    let backend_source = include_str!("../agent_execution_backend.rs");
    let runtime_policy_source =
        include_str!("../../../../runtime/macaca-runtime-host/src/execution_control.rs");
    let service_provider_source = include_str!(
        "../../../../runtime/macaca-runtime-host/src/execution_control_service_provider.rs"
    );
    let sources = [
        ("agent_execution_backend", backend_source),
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
        "agent_execution_backend.rs",
        "chat_orchestrator.rs",
        "framework_runner.rs",
        "hook_consumer.rs",
        "loop_manager.rs",
        "state.rs",
    ];
    let mut violations = Vec::new();

    for file in rust_files(&src_root) {
        if file.ends_with("tests.rs") {
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
        let relative = file
            .strip_prefix(&src_root)
            .unwrap_or(&file)
            .to_string_lossy()
            .replace('\\', "/");
        if !approved_files.iter().any(|allowed| relative == *allowed) {
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
    let state_source = include_str!("../state.rs");
    let hook_consumer_source = include_str!("../hook_consumer.rs");
    let loop_manager_source = include_str!("../loop_manager.rs");

    assert!(state_source.contains("Deprecated compatibility boundary"));
    assert!(state_source.contains("new execution-control ownership should"));
    assert!(hook_consumer_source.contains("service.execution_control"));
    assert!(hook_consumer_source.contains("Deprecated compatibility boundary"));
    assert!(loop_manager_source.contains("service-backed execution-control path"));
    assert!(loop_manager_source.contains("Deprecated compatibility boundary"));
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
