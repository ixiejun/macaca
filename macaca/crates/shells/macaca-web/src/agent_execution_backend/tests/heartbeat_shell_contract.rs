//! Agent execution backend test module — heartbeat shell fence extraction and exact contract.
//!
//! Part of the `agent_execution_backend/tests/` Facade module tree (P3 §4.5.1).
//! Validates execution control, heartbeat evidence, envelope contracts, and
//! architecture governance boundaries without application-specific business logic.


use super::support::*;
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

    let contract = heartbeat_exact_shell_contract(&command, &snapshot).expect("exact contract");

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

    assert!(!should_skip_heartbeat_without_source(&command, &snapshot));
}
