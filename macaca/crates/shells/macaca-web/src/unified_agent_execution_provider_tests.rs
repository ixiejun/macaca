//! Integration contract tests for unified Agent Execution provider (task 2.2.5).
//!
//! These tests assert that YAML chat/workflow sessions and WASM delegate sessions
//! converge on the single `service.agent_execution` provider backed by
//! `ComposedAgentExecutionBackend`.  They use static source inspection (no LLM)
//! so CI can enforce the unified call path without provider credentials.

#[cfg(test)]
mod tests {
    /// YAML chat (`/api/chat/v2`) must call the shared agent execution service.
    #[test]
    fn yaml_chat_session_uses_unified_agent_execution_service() {
        let chat = crate::chat_orchestrator::contract_source::chat_orchestrator_module_sources();
        let legacy_builder = ["FrameworkRunner::build_runtime", "_agent("].concat();

        assert!(chat.contains("run_chat_main_thread_via_agent_service"));
        assert!(chat.contains("AGENT_EXECUTION_SERVICE_ID"));
        assert!(chat.contains("AgentExecutionIntent::ChatMainThread"));
        assert!(chat.contains("into_service_command()"));
        assert!(chat.contains("ServiceBusSource::new(\"macaca.web.chat_orchestrator\")"));
        assert!(!chat.contains(&legacy_builder));
    }

    /// YAML workflow steps must enter through Application Service before agent execution.
    #[test]
    fn yaml_workflow_session_uses_application_service_delegate_path() {
        let runner = include_str!("agent_runner.rs");
        let bridge = include_str!("application_agent_delegate_bridge.rs");
        let legacy_builder = ["FrameworkRunner::build_runtime", "_agent("].concat();

        assert!(runner.contains("execute_via_application_delegate"));
        assert!(runner.contains("APPLICATION_SERVICE_ID"));
        assert!(runner.contains("AgentExecutionIntent::YamlWorkflowStep"));
        assert!(bridge.contains("AGENT_EXECUTION_SERVICE_ID"));
        assert!(runner.contains("ServiceBusSource::new(\"macaca.web.agent_runner\")"));
        assert!(!runner.contains("KernelServiceId::new(AGENT_EXECUTION_SERVICE_ID)"));
        assert!(!runner.contains(&legacy_builder));
    }

    /// WASM application delegation must call the shared bridge after Application Service.
    #[test]
    fn wasm_session_uses_unified_agent_execution_service() {
        let wasm = include_str!("wasm_orchestration_backend.rs");
        let bridge = include_str!("application_agent_delegate_bridge.rs");
        let executor_fast_path = [".delegate", "_task("].concat();

        assert!(wasm.contains("dispatch_agent_execution_via_service"));
        assert!(bridge.contains("AGENT_EXECUTION_SERVICE_ID"));
        assert!(bridge.contains("ORCHESTRATION_AGENT_EXECUTION_SOURCE"));
        assert!(!wasm.contains(&executor_fast_path));
    }

    /// Web startup registers exactly one composed backend for the service provider.
    #[test]
    fn web_registers_single_composed_agent_execution_backend() {
        let lib_source = crate::composition_bootstrap::contract_source::composition_bootstrap_module_sources();
        let adapters = include_str!("web_agent_execution_adapters.rs");
        let composed = include_str!(
            "../../../runtime/macaca-runtime-host/src/composed_agent_execution_backend.rs"
        );

        assert!(lib_source.contains("build_composed_web_agent_execution_backend"));
        assert!(lib_source.contains("AgentExecutionSystemServiceProvider::new"));
        assert!(lib_source.contains("ComposedAgentExecutionBackend"));
        assert!(adapters.contains("ServiceBackedFrameworkRuntimeAgentPort"));
        assert!(composed.contains("impl AgentExecutionBackend for ComposedAgentExecutionBackend"));
    }

    /// YAML and WASM entry surfaces must share the same service command + trace schema.
    #[test]
    fn yaml_and_wasm_share_agent_execute_command_and_trace_schema() {
        let yaml_chat = crate::chat_orchestrator::contract_source::chat_orchestrator_module_sources();
        let yaml_workflow = include_str!("agent_runner.rs");
        let wasm_bridge = include_str!("application_agent_delegate_bridge.rs");
        let provider = include_str!(
            "../../../runtime/macaca-runtime-host/src/agent_execution_service_provider.rs"
        );

        assert!(yaml_chat.contains("AGENT_EXECUTION_SERVICE_ID"));
        assert!(yaml_chat.contains("AgentExecutionCommand::new"));
        assert!(yaml_workflow.contains("APPLICATION_SERVICE_ID"));
        assert!(wasm_bridge.contains("AGENT_EXECUTION_SERVICE_ID"));
        assert!(wasm_bridge.contains("into_service_command()"));

        assert!(provider.contains("trace.system_service.agent_execution.v1"));
        assert!(provider.contains("AGENT_EXECUTE_COMMAND"));
    }

    /// Composed backend must emit audit-replay friendly execution results.
    #[test]
    fn composed_backend_returns_context_snapshot_for_audit_replay() {
        let composed = include_str!(
            "../../../runtime/macaca-runtime-host/src/composed_agent_execution_backend.rs"
        );
        let orchestration = include_str!(
            "../../../runtime/macaca-runtime-host/src/agent_execution_orchestration.rs"
        );
        let ports = include_str!(
            "../../../runtime/macaca-runtime-host/src/agent_execution_ports.rs"
        );

        assert!(composed.contains("build_agent_context_snapshot_via_service"));
        assert!(composed.contains("completed_execution_result"));
        assert!(composed.contains("failed_execution_result"));
        assert!(orchestration.contains("context_snapshot"));
        assert!(ports.contains("AgentExecutionEvidenceCollector"));
    }

    /// Session entrypoints must not bypass service.agent_execution with parallel backends.
    #[test]
    fn session_entrypoints_do_not_register_parallel_agent_execution_backends() {
        let lib_source = crate::composition_bootstrap::contract_source::composition_bootstrap_module_sources();
        let registration_calls = lib_source
            .lines()
            .filter(|line| {
                line.contains("build_composed_web_agent_execution_backend(")
                    && !line.trim_start().starts_with("use ")
            })
            .count();

        assert_eq!(
            registration_calls, 1,
            "only one composed backend factory registration is allowed"
        );
        assert!(
            !lib_source.contains("WebAgentExecutionBackend"),
            "legacy shell-owned backend must not be reintroduced"
        );
    }
}
