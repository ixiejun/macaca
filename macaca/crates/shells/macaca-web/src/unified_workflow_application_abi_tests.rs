//! Contract tests for unified YAML/WASM Application ABI path (task 2.4.3).
//!
//! These tests assert that YAML workflow steps and WASM host imports both enter
//! through `service.application` / `application.agent.delegate` before the
//! shared orchestration bridge dispatches `service.agent_execution`.

#[cfg(test)]
mod tests {
    #[test]
    fn yaml_workflow_enters_application_service_not_direct_agent_execution() {
        let runner = include_str!("agent_runner.rs");

        assert!(runner.contains("execute_via_application_delegate"));
        assert!(runner.contains("APPLICATION_SERVICE_ID"));
        assert!(runner.contains("ApplicationAgentDelegateCommand"));
        assert!(runner.contains("AGENT_EXECUTION_INTENT_METADATA_KEY"));
        assert!(runner.contains("AgentExecutionIntent::YamlWorkflowStep"));
        assert!(runner.contains("metadata_value()"));
        assert!(!runner.contains("execute_via_agent_service"));
        assert!(!runner.contains("KernelServiceId::new(AGENT_EXECUTION_SERVICE_ID)"));
    }

    fn host_import_bridge_module_sources() -> String {
        [
            include_str!(
                "../../../runtime/macaca-runtime-host/src/wasm_runtime_provider/host_import_bridge/mod.rs"
            ),
            include_str!(
                "../../../runtime/macaca-runtime-host/src/wasm_runtime_provider/host_import_bridge/routing_support.rs"
            ),
            include_str!(
                "../../../runtime/macaca-runtime-host/src/wasm_runtime_provider/host_import_bridge/dispatch.rs"
            ),
            include_str!(
                "../../../runtime/macaca-runtime-host/src/wasm_runtime_provider/host_import_bridge/task_lifecycle.rs"
            ),
        ]
        .join("\n")
    }

    #[test]
    fn wasm_host_import_enters_application_service_delegate_command() {
        let bridge = host_import_bridge_module_sources();

        assert!(bridge.contains("APPLICATION_SERVICE_ID"));
        assert!(bridge.contains("APPLICATION_AGENT_DELEGATE_COMMAND"));
        assert!(bridge.contains("ApplicationImport::AgentDelegate"));
    }

    #[test]
    fn orchestration_backend_forwards_both_intents_through_shared_bridge() {
        let backend = include_str!("wasm_orchestration_backend.rs");
        let delegate_bridge = include_str!("application_agent_delegate_bridge.rs");

        assert!(backend.contains("build_execution_command_from_delegate"));
        assert!(backend.contains("dispatch_agent_execution_via_service"));
        assert!(delegate_bridge.contains("AgentExecutionIntent::from_delegate_metadata"));
        assert!(delegate_bridge.contains("AGENT_EXECUTION_SERVICE_ID"));
    }

    #[test]
    fn yaml_and_wasm_share_application_abi_then_agent_execution_provider() {
        let yaml = include_str!("agent_runner.rs");
        let wasm = host_import_bridge_module_sources();
        let bridge = include_str!("application_agent_delegate_bridge.rs");
        let application_provider = include_str!(
            "../../../runtime/macaca-runtime-host/src/application_service_provider/command_dispatch.rs"
        );
        let execution_provider = include_str!(
            "../../../runtime/macaca-runtime-host/src/agent_execution_service_provider.rs"
        );

        assert!(yaml.contains("APPLICATION_SERVICE_ID"));
        assert!(wasm.contains("APPLICATION_SERVICE_ID"));
        assert!(application_provider.contains("APPLICATION_AGENT_DELEGATE_COMMAND"));
        assert!(bridge.contains("ORCHESTRATION_AGENT_EXECUTION_SOURCE"));
        assert!(execution_provider.contains("trace.system_service.agent_execution.v1"));
    }

    #[test]
    fn application_delegate_command_exposes_into_service_command_helper() {
        let proto = include_str!(
            "../../../foundation/macaca-proto/src/application_service/delegate.rs"
        );

        assert!(proto.contains("impl ApplicationAgentDelegateCommand"));
        assert!(proto.contains("into_service_command"));
        assert!(proto.contains("APPLICATION_AGENT_DELEGATE_COMMAND"));
    }

    #[test]
    fn unified_workflow_path_does_not_hardcode_application_role_names() {
        let sources = [
            include_str!("agent_runner.rs"),
            include_str!("application_agent_delegate_bridge.rs"),
            include_str!("wasm_orchestration_backend.rs"),
        ];

        for source in sources {
            assert!(!source.contains("\"coordinator\""));
            assert!(!source.contains("\"backend\""));
            assert!(!source.contains("\"frontend\""));
            assert!(!source.contains("\"planner\""));
        }
    }
}
