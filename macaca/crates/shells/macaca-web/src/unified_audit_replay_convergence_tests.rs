//! P1 exit validation — audit replay single-chain convergence (task 2.7.1).
//!
//! These contract tests encode the post-convergence execution topology documented
//! in `audit-replay-baseline.md`.  They use static source inspection (no LLM, no
//! live server) so CI can prove the pre-convergence triple chains (YAML 3 / WASM 3)
//! collapsed to one canonical `service.agent_execution` provider path per session type.
//!
//! Design pattern: **Specification by Example** — each test maps one baseline chain
//! row to concrete source markers that must exist (canonical) or must not exist
//! (retired parallel paths).

#[cfg(test)]
mod tests {
    /// Canonical service id every agent run must converge on.
    const AGENT_EXECUTION_SERVICE: &str = "service.agent_execution";

    /// Retired coordination-patch tokens that must not appear in production entry surfaces.
    const RETIRED_COORDINATION_PATCH_TOKENS: &[&str] = &[
        "legacy_unmarked",
        "non_authoritative",
        "suppress_executor_lifecycle",
        "legacy_chat_main_thread_goal_pause",
    ];

    /// Retired parallel-path call sites from pre-convergence inventory (task 0.3).
    ///
    /// Comments may still mention retired symbols for migration context; these tokens
    /// target live call patterns that would recreate a second execution backend.
    const RETIRED_YAML_PARALLEL_CALL_SITES: &[&str] = &[
        "Kernel::execute_agent(",
        ".delegate_task(",
        "FrameworkRunner::build_runtime_agent(",
    ];

    /// Retired WASM coordination/classification markers (task 2.6).
    const RETIRED_WASM_CLASSIFICATION_MARKERS: &[&str] = &[
        "non_authoritative",
        "legacy_unmarked",
        "execution.graph_owner",
        "authoritative_results",
    ];

    /// Count how many distinct execution entry surfaces still dispatch agent runs in a source file.
    ///
    /// Each marker represents a *separate* backend that would produce a distinct audit replay
    /// chain if it were still active.  Post-convergence we expect at most one marker per concern.
    fn count_active_entry_markers(source: &str, markers: &[&str]) -> usize {
        markers
            .iter()
            .filter(|marker| source.contains(**marker))
            .count()
    }

    /// Assert a production source surface does not contain retired coordination patches.
    fn assert_no_coordination_patches(surface_name: &str, source: &str) {
        for token in RETIRED_COORDINATION_PATCH_TOKENS {
            assert!(
                !source.contains(token),
                "{surface_name} must not contain retired coordination patch token `{token}`"
            );
        }
    }

    /// YAML `/api/chat/v2` main-thread chat converges on one agent execution chain.
    #[test]
    fn yaml_chat_session_audit_replay_single_chain() {
        let chat = crate::chat_orchestrator::contract_source::chat_orchestrator_module_sources();
        let bootstrap = include_str!("bootstrap.rs");

        // Canonical chain: chat_orchestrator → service.agent_execution (ComposedAgentExecutionBackend).
        assert!(chat.contains("run_chat_main_thread_via_agent_service"));
        assert!(chat.contains(AGENT_EXECUTION_SERVICE));
        assert!(chat.contains("AgentExecutionIntent::ChatMainThread"));
        assert!(bootstrap.contains("/api/chat/v2"));

        assert_no_coordination_patches("chat_orchestrator", &chat);

        // Retired yaml-A parallel pause patch must not remain.
        assert!(
            !chat.contains("legacy_chat_main_thread_goal_pause"),
            "chat main thread must use manifest execution_control, not shell pause patches"
        );
    }

    /// YAML workflow and delegate paths share Application ABI then one execution provider.
    #[test]
    fn yaml_workflow_and_delegate_share_single_execution_chain() {
        let runner = include_str!("agent_runner.rs");
        let delegate_bridge = include_str!("application_agent_delegate_bridge.rs");
        let dispatcher = include_str!("delegated_task_dispatcher.rs");

        // Canonical chain: application.agent.delegate → service.agent_execution.
        assert!(runner.contains("execute_via_application_delegate"));
        assert!(delegate_bridge.contains(AGENT_EXECUTION_SERVICE));
        assert!(delegate_bridge.contains("dispatch_agent_execution_via_service"));

        // Retired yaml-B: direct kernel / shell-owned execution bypass.
        assert!(
            !runner.contains("execute_via_agent_service"),
            "workflow steps must not bypass Application Service delegate ABI"
        );
        assert!(
            count_active_entry_markers(runner, RETIRED_YAML_PARALLEL_CALL_SITES) == 0,
            "yaml workflow must not retain parallel kernel/framework execution backends"
        );

        // Retired yaml-C: executor fast-path delegation.
        assert!(dispatcher.contains(AGENT_EXECUTION_SERVICE));
        assert!(
            !dispatcher.contains(".delegate_task("),
            "delegate_task must route through service.agent_execution, not kernel executor"
        );

        assert_no_coordination_patches("agent_runner", runner);
        assert_no_coordination_patches("delegated_task_dispatcher", dispatcher);
    }

    /// Concatenate hosted execution module sources for architectural contract scanning.
    fn hosted_execution_module_sources() -> String {
        [
            include_str!(
                "../../../runtime/macaca-runtime-host/src/application_execution_hosted/mod.rs"
            ),
            include_str!(
                "../../../runtime/macaca-runtime-host/src/application_execution_hosted/types.rs"
            ),
            include_str!(
                "../../../runtime/macaca-runtime-host/src/application_execution_hosted/abi_adapter.rs"
            ),
            include_str!(
                "../../../runtime/macaca-runtime-host/src/application_execution_hosted/host_signal_translator.rs"
            ),
            include_str!(
                "../../../runtime/macaca-runtime-host/src/application_execution_hosted/provider.rs"
            ),
            include_str!(
                "../../../runtime/macaca-runtime-host/src/application_execution_hosted/provider_impl.rs"
            ),
        ]
        .join("\n")
    }

    /// WASM host import and orchestration backend converge on the shared delegate bridge.
    #[test]
    fn wasm_session_audit_replay_single_chain() {
        let hosted = hosted_execution_module_sources();
        let bridge = include_str!(
            "../../../runtime/macaca-runtime-host/src/wasm_runtime_provider/host_import_bridge.rs"
        );
        let wasm_backend = include_str!("wasm_orchestration_backend.rs");
        let delegate_bridge = include_str!("application_agent_delegate_bridge.rs");

        // Canonical chain: application.agent.delegate → service.agent_execution.
        assert!(bridge.contains("APPLICATION_AGENT_DELEGATE_COMMAND"));
        assert!(wasm_backend.contains("dispatch_agent_execution_via_service"));
        assert!(delegate_bridge.contains(AGENT_EXECUTION_SERVICE));

        // Retired wasm-A: non_authoritative/legacy_unmarked terminal reconciliation branches.
        for token in RETIRED_WASM_CLASSIFICATION_MARKERS {
            assert!(
                !hosted.contains(token),
                "application_execution_hosted must not classify host commands with `{token}`"
            );
        }

        // Retired wasm-B: execution.graph_owner compatibility labels in result metadata.
        assert!(
            !bridge.contains("execution.graph_owner"),
            "host import bridge must not emit graph_owner compatibility metadata"
        );

        assert_no_coordination_patches("application_execution_hosted", &hosted);
        assert_no_coordination_patches("host_import_bridge", bridge);
    }

    /// All YAML and WASM entry surfaces register exactly one composed execution backend.
    #[test]
    fn production_registers_single_composed_agent_execution_backend() {
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
            "multiple composed backends would recreate parallel audit replay chains"
        );
        assert!(
            lib_source.contains("ComposedAgentExecutionBackend"),
            "composed backend must remain the sole coarse lifecycle + execution owner"
        );
    }

    /// Pre-convergence triple chains (3 YAML + 3 WASM) are documented as converged to 1 each.
    #[test]
    fn audit_replay_baseline_documents_post_convergence_single_chain() {
        let baseline = include_str!(
            "../../../../openspec/changes/refactor-unified-call-path-microkernel/audit-replay-baseline.md"
        );

        assert!(
            baseline.contains("Post-convergence"),
            "baseline must record post-2.6 convergence report"
        );
        assert!(
            baseline.contains("Distinct chain count: 1"),
            "baseline must assert single-chain target for YAML and WASM"
        );
        assert!(
            baseline.contains("unified_audit_replay_convergence_tests"),
            "baseline must reference the CI contract test module"
        );
    }

    /// RC-CHAT-001/002 proxy: `/api/chat/v2` route remains registered after path convergence.
    #[test]
    fn chat_v2_route_remains_registered_for_route_c_regression() {
        let bootstrap = include_str!("bootstrap.rs");
        let chat = crate::chat_orchestrator::contract_source::chat_orchestrator_module_sources();

        assert!(bootstrap.contains("/api/chat/v2"));
        assert!(bootstrap.contains("post_chat_v2"));
        assert!(chat.contains("run_chat_main_thread_via_agent_service"));
    }
}
