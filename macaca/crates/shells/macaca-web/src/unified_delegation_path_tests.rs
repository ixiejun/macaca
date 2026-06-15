//! Contract tests for unified delegation paths (task 2.3.5).
//!
//! These tests validate that fork-join (`delegate_task`) and goal-task
//! (`create_goal` → PlanLoop) paths route pause/resume through
//! `service.execution_control` and `service.agent_execution` rather than
//! driving kernel executor semantics directly from loop_manager.

#[cfg(test)]
mod tests {
    #[test]
    fn delegate_task_path_uses_service_agent_execution_dispatcher() {
        let orchestration = include_str!("orchestration_tools.rs");
        let dispatcher =
            include_str!("../../../runtime/macaca-runtime-host/src/delegated_task_dispatcher.rs");
        let tool_bootstrap =
            include_str!("../../../runtime/macaca-runtime-host/src/tool_bootstrap.rs");

        assert!(orchestration.contains("bootstrap_delegate_task_tool"));
        assert!(!orchestration.contains("DelegateTaskTool::"));
        assert!(dispatcher.contains("ServiceDelegatedTaskDispatcher"));
        assert!(tool_bootstrap.contains("DelegateTaskTool::"));
        assert!(tool_bootstrap.contains("ExecutionControlForkJoinCoordinator"));
        assert!(tool_bootstrap.contains(".dispatch("));
        assert!(dispatcher.contains("AGENT_EXECUTION_SERVICE_ID"));
        assert!(dispatcher.contains("service.agent_execution"));
        assert!(dispatcher.contains("begin_service_backed_delegation"));
    }

    #[test]
    fn fork_join_resume_uses_execution_control_before_local_channel() {
        let hook_consumer = include_str!("hook_consumer.rs");
        let fork_adapter = include_str!("fork_join_shell_adapter.rs");

        assert!(hook_consumer.contains("deliver_fork_join_resume_and_notify_parent"));
        assert!(fork_adapter.contains("deliver_parent_fork_resume"));
        assert!(fork_adapter.contains("EXECUTION_CONTROL_SERVICE_ID"));
    }

    #[test]
    fn goal_task_path_registers_and_resumes_via_execution_control() {
        let loop_manager = crate::loop_manager::contract_source::loop_manager_module_sources();
        let goal_adapter = include_str!("goal_lifecycle_shell_adapter.rs");
        let toolkit = crate::framework_toolkit::contract_source::framework_toolkit_module_sources();

        assert!(loop_manager.contains("ExecutionControlGoalLifecycleCoordinator"));
        assert!(loop_manager.contains("deliver_goal_resume_and_notify_parent"));
        assert!(goal_adapter.contains("register_goal_wait_via_execution_control"));
        assert!(goal_adapter.contains("deliver_parent_goal_resume"));
        assert!(toolkit.contains("register_goal_wait_via_execution_control"));
    }

    #[test]
    fn worker_loop_executes_tasks_via_agent_execution_service() {
        let loop_manager = crate::loop_manager::contract_source::loop_manager_module_sources();

        assert!(loop_manager.contains("execute_worker_task_via_agent_service"));
        assert!(loop_manager.contains("run_loop_agent_execution_service_call"));
        assert!(loop_manager.contains("AgentExecutionIntent::TaskWorker"));
        assert!(loop_manager.contains("macaca.web.worker_loop"));
    }

    #[test]
    fn framework_runtime_agent_port_owned_by_runtime_host_service() {
        let web_adapters = include_str!("web_agent_execution_adapters.rs");
        let construction_adapter = include_str!("framework_agent_construction_shell_adapter.rs");
        let runtime_host_service = include_str!(
            "../../../runtime/macaca-runtime-host/src/framework_runtime_agent_service.rs"
        );

        assert!(
            !web_adapters
                .contains("impl FrameworkRuntimeAgentPort for WebFrameworkRuntimeAgentPort"),
            "web shell must not implement FrameworkRuntimeAgentPort directly"
        );
        assert!(web_adapters.contains("ServiceBackedFrameworkRuntimeAgentPort"));
        assert!(web_adapters.contains("RuntimeHostFrameworkAgentConstructionService"));
        assert!(web_adapters.contains("WebFrameworkAgentMaterializationPort"));
        assert!(construction_adapter.contains("impl FrameworkAgentMaterializationPort"));
        assert!(!construction_adapter.contains("impl FrameworkAgentConstructionPort"));
        assert!(construction_adapter.contains(
            "materialize_runtime_react_agent_from_context_snapshot_with_execution_policy"
        ));
        assert!(!construction_adapter.contains("build_runtime_agent_from_context_snapshot"));
        assert!(runtime_host_service.contains(
            "impl FrameworkAgentConstructionPort for RuntimeHostFrameworkAgentConstructionService"
        ));
        assert!(runtime_host_service
            .contains("impl FrameworkRuntimeAgentPort for ServiceBackedFrameworkRuntimeAgentPort"));
    }

    #[test]
    fn session_loop_wake_register_shutdown_via_execution_control() {
        let loop_manager = crate::loop_manager::contract_source::loop_manager_module_sources();
        let session_adapter = include_str!("session_loop_shell_adapter.rs");
        let runtime_coordinator = include_str!(
            "../../../runtime/macaca-runtime-host/src/execution_control_session_loop.rs"
        );
        let chat_orchestrator =
            crate::chat_orchestrator::contract_source::chat_orchestrator_module_sources();

        assert!(loop_manager.contains("ExecutionControlSessionLoopCoordinator"));
        assert!(loop_manager.contains("register_plan_loop_via_execution_control"));
        assert!(loop_manager.contains("register_worker_loops_via_execution_control"));
        assert!(loop_manager.contains("wake_plan_loop_and_notify_local"));
        assert!(loop_manager.contains("wake_worker_loops_and_notify_local"));
        assert!(session_adapter.contains("EXECUTION_CONTROL_SERVICE_ID"));
        assert!(session_adapter.contains("SESSION_LOOP_TASK_CAPABILITY_ID"));
        assert!(runtime_coordinator.contains("register_session_loop"));
        assert!(runtime_coordinator.contains("record_loop_wake"));
        assert!(runtime_coordinator.contains("record_session_loop_shutdown"));
        assert!(chat_orchestrator.contains("shutdown_session_loops_via_execution_control"));
        assert!(chat_orchestrator.contains("REASON_SESSION_LOOP_APPLICATION_CLEANUP"));
    }

    #[test]
    fn unified_paths_do_not_hardcode_application_role_names() {
        let sources = [
            include_str!("orchestration_tools.rs"),
            include_str!("../../../runtime/macaca-runtime-host/src/delegated_task_dispatcher.rs"),
            include_str!("goal_lifecycle_shell_adapter.rs"),
            include_str!("fork_join_shell_adapter.rs"),
            include_str!("session_loop_shell_adapter.rs"),
        ];

        for source in sources {
            assert!(!source.contains("\"coordinator\""));
            assert!(!source.contains("\"backend\""));
            assert!(!source.contains("\"frontend\""));
            assert!(!source.contains("\"planner\""));
        }
    }
}
