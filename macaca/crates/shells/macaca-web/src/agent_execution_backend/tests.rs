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
fn evidence_metadata_is_rendered_as_structured_execution_context() {
    let mut command = AgentExecutionCommand::new(
        macaca_proto::ApplicationId::from_name("demo"),
        "session-a",
        "coordinator",
        AgentExecutionIntent::Heartbeat,
        "run heartbeat work",
        macaca_proto::TraceContext::new("trace-heartbeat-evidence-context"),
    )
    .unwrap();
    command.delegated_context = serde_json::json!({
        "heartbeat": {
            "run_id": "run-a"
        }
    });
    command.metadata.insert(
        "evidence.expected_artifact_path".into(),
        "/workspace/agents/a/sentinel.md".into(),
    );

    let prompt = WebAgentExecutionBackend::user_prompt_with_context(&command);

    assert!(prompt.contains("Structured evidence context"));
    assert!(prompt.contains("\"delegated_context\""));
    assert!(prompt.contains("\"evidence_requirements\""));
    assert!(prompt.contains("\"expected_artifact_path\""));
    assert!(prompt.contains("/workspace/agents/a/sentinel.md"));
}

#[test]
fn execution_envelope_is_rendered_as_highest_priority_contract() {
    let mut command = AgentExecutionCommand::new(
        macaca_proto::ApplicationId::from_name("demo"),
        "session-a",
        "coordinator",
        AgentExecutionIntent::Heartbeat,
        "run heartbeat work",
        macaca_proto::TraceContext::new("trace-envelope-rendering"),
    )
    .unwrap();
    command.execution_envelope = Some(
        macaca_proto::AutonomousExecutionEnvelope::compile(
            macaca_proto::AutonomousExecutionSourceKind::HeartbeatProfile,
            "run heartbeat work",
            &std::collections::BTreeMap::from([(
                "evidence.expected_artifact_path".into(),
                "/workspace/agents/a/sentinel.md".into(),
            )]),
        )
        .unwrap(),
    );

    let prompt = WebAgentExecutionBackend::user_prompt_with_context(&command);

    assert!(prompt.contains("Highest-priority delegated execution contract"));
    assert!(prompt.contains("\"source_kind\": \"heartbeat_profile\""));
    assert!(prompt.contains("\"kind\": \"require_artifact\""));
    assert!(prompt.contains("/workspace/agents/a/sentinel.md"));
}

#[test]
fn artifact_completion_policy_uses_short_runtime_loop_budget() {
    let mut command = AgentExecutionCommand::new(
        macaca_proto::ApplicationId::from_name("demo"),
        "session-a",
        "coordinator",
        AgentExecutionIntent::Heartbeat,
        "run heartbeat work",
        macaca_proto::TraceContext::new("trace-artifact-budget"),
    )
    .unwrap();
    command.execution_envelope = Some(
        macaca_proto::AutonomousExecutionEnvelope::compile(
            macaca_proto::AutonomousExecutionSourceKind::HeartbeatProfile,
            "run heartbeat work",
            &std::collections::BTreeMap::from([(
                "evidence.expected_artifact_path".into(),
                "/workspace/agents/a/sentinel.md".into(),
            )]),
        )
        .unwrap(),
    );

    assert_eq!(
        WebAgentExecutionBackend::runtime_agent_max_iters(&command),
        12
    );
}

#[test]
fn artifact_completion_policy_requires_authorized_tool_use() {
    let mut command = AgentExecutionCommand::new(
        macaca_proto::ApplicationId::from_name("demo"),
        "session-a",
        "coordinator",
        AgentExecutionIntent::Heartbeat,
        "run artifact work",
        macaca_proto::TraceContext::new("trace-artifact-tool-choice"),
    )
    .unwrap();
    command.execution_envelope = Some(
        macaca_proto::AutonomousExecutionEnvelope::compile(
            macaca_proto::AutonomousExecutionSourceKind::HeartbeatProfile,
            "run artifact work",
            &std::collections::BTreeMap::from([(
                "evidence.expected_artifact_path".into(),
                "/workspace/agents/a/sentinel.md".into(),
            )]),
        )
        .unwrap(),
    );

    assert_eq!(
        WebAgentExecutionBackend::runtime_agent_tool_choice(&command),
        Some(macaca_framework::model::ToolChoice::Required)
    );
}

#[test]
fn agent_result_completion_policy_keeps_automatic_tool_choice() {
    let command = AgentExecutionCommand::new(
        macaca_proto::ApplicationId::from_name("demo"),
        "session-a",
        "coordinator",
        AgentExecutionIntent::TaskWorker,
        "summarize work",
        macaca_proto::TraceContext::new("trace-default-tool-choice"),
    )
    .unwrap();

    assert_eq!(
        WebAgentExecutionBackend::runtime_agent_tool_choice(&command),
        None
    );
}

#[test]
fn agent_result_completion_policy_keeps_default_runtime_loop_budget() {
    let command = AgentExecutionCommand::new(
        macaca_proto::ApplicationId::from_name("demo"),
        "session-a",
        "coordinator",
        AgentExecutionIntent::TaskWorker,
        "summarize work",
        macaca_proto::TraceContext::new("trace-default-budget"),
    )
    .unwrap();

    assert_eq!(
        WebAgentExecutionBackend::runtime_agent_max_iters(&command),
        25
    );
}

#[test]
fn extracts_single_shell_fence_from_heartbeat_contract() {
    let body = r#"
# Heartbeat

```sh
mkdir -p /tmp/demo
printf ok > /tmp/demo/sentinel.md
```
"#;

    let command = extract_single_shell_fence(body).expect("single shell block");

    assert!(command.contains("mkdir -p /tmp/demo"));
    assert!(command.contains("sentinel.md"));
}

#[test]
fn ambiguous_heartbeat_shell_fences_are_not_exact_contracts() {
    let body = r#"
```sh
echo one
```
```sh
echo two
```
"#;

    assert!(extract_single_shell_fence(body).is_none());
}

#[test]
fn heartbeat_exact_shell_contract_requires_trusted_source_and_artifact_policy() {
    let dir = tempfile::tempdir().expect("tempdir");
    let heartbeat = dir.path().join("HEARTBEAT.md");
    std::fs::write(
        &heartbeat,
        r#"
```sh
printf ok
```
"#,
    )
    .expect("write heartbeat");
    let mut command = AgentExecutionCommand::new(
        macaca_proto::ApplicationId::from_name("demo"),
        "session-a",
        "coordinator",
        AgentExecutionIntent::Heartbeat,
        "run heartbeat work",
        macaca_proto::TraceContext::new("trace-exact-contract"),
    )
    .unwrap();
    command.execution_envelope = Some(
        macaca_proto::AutonomousExecutionEnvelope::compile(
            macaca_proto::AutonomousExecutionSourceKind::HeartbeatProfile,
            "run heartbeat work",
            &std::collections::BTreeMap::from([(
                "evidence.expected_artifact_path".into(),
                "/tmp/demo/sentinel.md".into(),
            )]),
        )
        .unwrap(),
    );
    let context_command = AgentContextBuildCommand::from_execution(&command);
    let mut snapshot = AgentContextSnapshot::minimal(&context_command, "trusted context");
    snapshot.sources.push(macaca_proto::AgentContextSource {
        kind: "profile_file".into(),
        name: "HEARTBEAT.md".into(),
        location: Some(heartbeat.display().to_string()),
        metadata: Default::default(),
    });

    let contract = WebAgentExecutionBackend::heartbeat_exact_shell_contract(&command, &snapshot)
        .expect("exact contract");

    assert_eq!(contract, "printf ok");
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
fn agent_execution_service_registers_skill_self_evolution_decorator() {
    let lib_source = include_str!("../lib.rs");
    let decorator_source = include_str!("../skill_self_evolution_execution_observer.rs");

    assert!(lib_source.contains("SkillSelfEvolutionObservedAgentExecutionBackend"));
    assert!(lib_source.contains("SkillSelfEvolutionObservedAgentExecutionBackend::new"));
    assert!(decorator_source.contains("impl AgentExecutionBackend"));
    assert!(decorator_source.contains("emit_runtime_event"));
    assert!(decorator_source.contains("\"service.agent_execution\""));
    assert!(decorator_source.contains("\"agent_execution_completed_seen\""));
    assert!(decorator_source.contains("\"agent_execution_service_error\""));
    assert!(decorator_source.contains("observe_agent_execution_result_for_skill_self_evolution"));
    assert!(
        !decorator_source.contains("result.output"),
        "decorator checkpoints must not copy raw agent output into EventLog"
    );
}

#[test]
fn skill_self_evolution_observation_is_centralized_at_agent_execution_boundary() {
    let backend_source = include_str!("../agent_execution_backend.rs");
    let chat_source = include_str!("../chat_orchestrator.rs");
    let event_persistence_source = include_str!("../event_persistence.rs");
    let observer_source = include_str!("../skill_self_evolution_observer.rs");

    assert!(
        !backend_source.contains("spawn_skill_self_evolution_observation"),
        "agent execution backend should return results; the decorator owns observation"
    );
    assert!(
        !chat_source.contains("observe_agent_execution_result_for_skill_self_evolution"),
        "chat orchestration must not duplicate Skill self-evolution observation"
    );
    assert!(
        !event_persistence_source.contains("observe_executor_event_for_skill_self_evolution"),
        "event persistence must remain durable logging, not Skill proposal ownership"
    );
    assert!(
        !observer_source.contains("observe_executor_event_for_skill_self_evolution"),
        "observer helpers must expose the service.agent_execution result boundary only"
    );
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
