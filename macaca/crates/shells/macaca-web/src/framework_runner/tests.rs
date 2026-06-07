

#[cfg(test)]
mod tests {
    use crate::framework_runner::{
        contract_source, is_framework_tool_wrapper_trace, should_forward_driver_trace,
        tool_response_text, truncate_tool_output, ExecutionControlMiddleware, FrameworkRunner,
    };
    use macaca_app::model::AppContextConfig;
    use macaca_framework::message::{ContentBlock, TextBlock};
    use macaca_framework::tool::ToolResponse;
    use macaca_proto::config::{AgentProfileContextConfig, ContextConfig};
    use macaca_sdk::tools::TraceEvent;

    #[test]
    fn truncate_tool_output_respects_utf8_boundaries() {
        let text = "─".repeat(800);

        let truncated = truncate_tool_output(&text, 2000);

        assert!(truncated.ends_with("[truncated, 2400 bytes]"));
        assert!(truncated.is_char_boundary(truncated.len()));
    }

    #[test]
    fn truncate_tool_output_keeps_short_text_unchanged() {
        let text = "北京 weather";

        assert_eq!(truncate_tool_output(text, 2000), text);
    }

    #[test]
    fn tool_response_text_joins_multiple_text_blocks() {
        let response = ToolResponse {
            content: vec![
                ContentBlock::Text(TextBlock {
                    text: "hello".into(),
                }),
                ContentBlock::Text(TextBlock {
                    text: " world".into(),
                }),
            ],
            metadata: None,
            is_stream: false,
            is_last: true,
            is_interrupted: false,
        };

        assert_eq!(tool_response_text(&response), "hello world");
    }

    #[test]
    fn tool_response_text_returns_empty_string_for_empty_response() {
        let response = ToolResponse {
            content: Vec::new(),
            metadata: None,
            is_stream: false,
            is_last: true,
            is_interrupted: false,
        };

        assert_eq!(tool_response_text(&response), "");
    }

    #[test]
    fn production_code_does_not_call_deprecated_build_system_prompt_shim() {
        let source = contract_source::framework_runner_module_sources();
        let forbidden = concat!("Self::", "build_system_prompt(");
        assert!(
            !source.contains(forbidden),
            "production path should call build_context_system_prompt directly"
        );
    }

    #[test]
    fn agent_context_snapshot_records_replayable_skill_and_policy_evidence() {
        let source = contract_source::framework_runner_module_sources();

        assert!(source.contains("snapshot.visible_skills"));
        assert!(source.contains("snapshot.filtered_skills"));
        assert!(source.contains("skill_snapshot_unavailable"));
        assert!(source.contains("snapshot.tool_policy"));
        assert!(source.contains("agent_context_built"));
    }

    #[test]
    fn framework_tool_wrapper_trace_is_suppressed_without_driver_identity() {
        let call_trace = TraceEvent {
            event_type: "tool_call".into(),
            tool_name: Some("browser_run_code".into()),
            ..Default::default()
        };
        let result_trace = TraceEvent {
            event_type: "tool_result".into(),
            tool_name: Some("browser_run_code".into()),
            ..Default::default()
        };

        assert!(is_framework_tool_wrapper_trace(&call_trace));
        assert!(is_framework_tool_wrapper_trace(&result_trace));
        assert!(!should_forward_driver_trace(&call_trace));
        assert!(!should_forward_driver_trace(&result_trace));
    }

    #[test]
    fn non_wrapper_driver_traces_are_preserved_for_diagnostics() {
        let no_driver_diagnostic = TraceEvent {
            event_type: "thinking".into(),
            tool_name: Some("browser_run_code".into()),
            ..Default::default()
        };
        let concrete_driver_call = TraceEvent {
            event_type: "tool_call".into(),
            driver_id: Some("browser-driver".into()),
            tool_name: Some("browser_run_code".into()),
            ..Default::default()
        };
        let concrete_driver_result = TraceEvent {
            event_type: "tool_result".into(),
            driver_id: Some("browser-driver".into()),
            tool_name: Some("browser_run_code".into()),
            ..Default::default()
        };

        assert!(!is_framework_tool_wrapper_trace(&no_driver_diagnostic));
        assert!(should_forward_driver_trace(&no_driver_diagnostic));
        assert!(!is_framework_tool_wrapper_trace(&concrete_driver_call));
        assert!(should_forward_driver_trace(&concrete_driver_call));
        assert!(!is_framework_tool_wrapper_trace(&concrete_driver_result));
        assert!(should_forward_driver_trace(&concrete_driver_result));
    }

    #[test]
    fn runtime_driver_trace_route_uses_shared_wrapper_suppression() {
        let source = contract_source::framework_runner_module_sources();
        let attach_start = source
            .find("async fn attach_driver_trace_route")
            .expect("driver trace route attachment should exist");
        let attach_end = source[attach_start..]
            .find("DriverTraceRoute::Coordinator")
            .map(|offset| attach_start + offset)
            .and_then(|coordinator_start| {
                source[coordinator_start..]
                    .find("\n    }\n")
                    .map(|offset| coordinator_start + offset + 6)
            })
            .expect("driver trace route attachment should include coordinator branch");
        let attach_source = &source[attach_start..attach_end];
        let guard_position = attach_source
            .find("if !should_forward_driver_trace(&trace)")
            .expect("driver trace routing should use shared suppression guard");
        let runtime_position = attach_source
            .find("DriverTraceRoute::Runtime")
            .expect("runtime driver trace route should exist");

        assert!(
            guard_position < runtime_position,
            "runtime route must pass through the shared wrapper suppression guard before forwarding"
        );
        assert!(
            !attach_source.contains("framework_tool_wrapper"),
            "route-specific wrapper predicates would let Runtime drift from Executor again"
        );
    }

    #[test]
    fn context_config_precedence_agent_overrides_app_and_system_engine() {
        let mut base = ContextConfig::default();
        base.default_engine = "system-windowed".into();
        base.fallback_engine = "system-legacy".into();

        let merged = FrameworkRunner::merge_context_config_overrides(
            base,
            Some(&AppContextConfig {
                engine: Some("app-pruning".into()),
                fallback_engine: Some("app-summary".into()),
                workspace_guides: None,
                agent_profile: Some(AgentProfileContextConfig {
                    enabled: true,
                    ..Default::default()
                }),
            }),
            Some("agent-custom"),
        );

        assert_eq!(merged.default_engine, "agent-custom");
        assert_eq!(merged.fallback_engine, "app-summary");
        assert!(merged.agent_profile.enabled);
    }

    #[test]
    fn context_config_precedence_keeps_system_defaults_when_overrides_absent() {
        let mut base = ContextConfig::default();
        base.default_engine = "system-windowed".into();
        base.fallback_engine = "system-legacy".into();

        let merged = FrameworkRunner::merge_context_config_overrides(base.clone(), None, None);
        assert_eq!(merged.default_engine, base.default_engine);
        assert_eq!(merged.fallback_engine, base.fallback_engine);
    }

    #[test]
    fn execution_control_middleware_matches_configured_tool_barrier() {
        let policy = macaca_proto::ExecutionControlPolicy::enabled(
            vec![macaca_proto::ExecutionControlTrigger::tool_call_barrier(
                "create_goal",
            )],
            vec![macaca_proto::ExecutionControlResumeSource::goal_lifecycle()],
            macaca_proto::ExecutionControlCheckpointMode::ReferenceOnly,
        );

        assert!(ExecutionControlMiddleware::policy_pauses_after_tool(
            &policy,
            "create_goal"
        ));
        assert!(!ExecutionControlMiddleware::policy_pauses_after_tool(
            &policy,
            "claude_code_execute"
        ));
    }

    #[test]
    fn coordinator_execution_control_uses_supplied_policy() {
        let source = contract_source::framework_runner_module_sources();
        let coordinator_start = source
            .find("async fn build_coordinator")
            .expect("coordinator builder should exist");
        let coordinator_end = source[coordinator_start..]
            .find("let merged_ctx")
            .map(|offset| coordinator_start + offset)
            .expect("coordinator context section should exist");
        let coordinator_source = &source[coordinator_start..coordinator_end];

        assert!(coordinator_source.contains("policy: execution_control.policy.clone()"));
        assert!(!coordinator_source.contains("ExecutionControlPolicy::enabled("));
        assert!(!coordinator_source.contains("tool_call_barrier(\"create_goal\")"));
    }
}
