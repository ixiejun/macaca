//! Agent execution backend test module — static source wiring and audit trail scans.
//!
//! Part of the `agent_execution_backend/tests/` Facade module tree (P3 §4.5.1).
//! Validates execution control, heartbeat evidence, envelope contracts, and
//! architecture governance boundaries without application-specific business logic.


use super::support::*;
#[test]
fn execution_backend_registers_enabled_policy_before_adapter_install() {
    let host_adapter = include_str!("../../web_agent_execution_adapters.rs");
    let composed = include_str!(
        "../../../../../runtime/macaca-runtime-host/src/composed_agent_execution_backend.rs"
    );

    assert!(composed.contains("register_execution_control(&command, control_policy)"));
    assert!(host_adapter.contains("ExecutionControlRegisterExecutionCommand"));
    assert!(composed.contains("install_execution_control(&command, policy, execution_id)"));
}

#[test]
fn execution_backend_consumes_context_snapshot_without_rebuilding_context() {
    let composed = include_str!(
        "../../../../../runtime/macaca-runtime-host/src/composed_agent_execution_backend.rs"
    );
    let construction_adapter = include_str!("../../framework_agent_construction_shell_adapter.rs");
    let service_context_call = "build_agent_context_snapshot_via_service";
    let snapshot_runtime_builder = "build_runtime_agent_from_context_snapshot";
    let legacy_runtime_builder = ["FrameworkRunner::build_runtime", "_agent("].concat();

    assert!(composed.contains(service_context_call));
    assert!(construction_adapter.contains(snapshot_runtime_builder));
    assert!(!composed.contains(&legacy_runtime_builder));
}

#[test]
fn execution_backend_returns_context_snapshot_for_audit_replay() {
    let host_adapter = include_str!("../../web_agent_execution_adapters.rs");
    let composed = include_str!(
        "../../../../../runtime/macaca-runtime-host/src/composed_agent_execution_backend.rs"
    );
    let orchestration_source = include_str!(
        "../../../../../runtime/macaca-runtime-host/src/agent_execution_orchestration.rs"
    );

    assert!(composed.contains("completed_execution_result"));
    assert!(composed.contains("failed_execution_result"));
    assert!(host_adapter.contains("ServiceBusSource::new(WEB_AGENT_EXECUTION_BUS_SOURCE)"));
    assert!(orchestration_source.contains("AgentExecutionStatus::Failed"));
}

#[test]
fn agent_execution_service_registers_skill_self_evolution_decorator() {
    let lib_source =
        crate::composition_bootstrap::contract_source::composition_bootstrap_module_sources();
    let decorator_source = include_str!("../../skill_self_evolution_execution_observer.rs");

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
