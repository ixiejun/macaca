//! Forbidden token catalogue for serviceization escape-hatch gates.
//!
//! The catalogue is declarative policy data: each token has a rule family and a
//! rationale so CI output remains traceable to the architecture boundary it
//! protects.

/// A source token that must not appear in production Rust code unless the file
/// is an approved test surface, fixture, or service provider bridge.
pub struct ForbiddenToken {
    pub family: &'static str,
    pub token: &'static str,
    pub rationale: &'static str,
}

pub fn forbidden_tokens() -> Vec<ForbiddenToken> {
    vec![
        ForbiddenToken { family: "application-runtime-direct-start", token: "AppRuntime::start_app", rationale: "application lifecycle must enter through Application Service commands" },
        ForbiddenToken { family: "application-runtime-direct-start", token: "start_app_from_file", rationale: "file-backed application starts must pass through traced service commands" },
        ForbiddenToken { family: "web-direct-runtime-field", token: "state.driver_runtime", rationale: "Web must use the driver service client instead of runtime internals" },
        ForbiddenToken { family: "web-direct-runtime-field", token: "state.mcp_runtime", rationale: "Web must use the MCP service catalog/snapshot command" },
        ForbiddenToken { family: "web-direct-runtime-field", token: "state.runtime", rationale: "Web must use application service clients instead of AppRuntime anchors" },
        ForbiddenToken { family: "web-direct-runtime-field", token: "state.registry", rationale: "Web must use service-backed application metadata views" },
        ForbiddenToken { family: "hardcoded-agent-role", token: "\"coordinator\"", rationale: "production OS layers must receive agent names from manifests or service descriptors" },
        ForbiddenToken { family: "hardcoded-agent-role", token: "\"planner\"", rationale: "production OS layers must receive agent names from manifests or service descriptors" },
        ForbiddenToken { family: "hardcoded-agent-role", token: "\"worker\"", rationale: "production OS layers must receive agent names from manifests or service descriptors" },
        ForbiddenToken { family: "hardcoded-agent-role", token: "\"backend\"", rationale: "production OS layers must receive agent names from manifests or service descriptors" },
        ForbiddenToken { family: "hardcoded-agent-role", token: "\"frontend\"", rationale: "production OS layers must receive agent names from manifests or service descriptors" },
        ForbiddenToken { family: "hardcoded-agent-role", token: "\"architect\"", rationale: "production OS layers must receive agent names from manifests or service descriptors" },
        ForbiddenToken { family: "provider-model-routing-name", token: "\"openai\"", rationale: "provider/model routing names must stay inside LLM service descriptors" },
        ForbiddenToken { family: "provider-model-routing-name", token: "\"anthropic\"", rationale: "provider/model routing names must stay inside LLM service descriptors" },
        ForbiddenToken { family: "provider-model-routing-name", token: "\"dashscope\"", rationale: "provider/model routing names must stay inside LLM service descriptors" },
        ForbiddenToken { family: "provider-model-routing-name", token: "\"deepseek\"", rationale: "provider/model routing names must stay inside LLM service descriptors" },
        ForbiddenToken { family: "provider-model-routing-name", token: "\"minimax\"", rationale: "provider/model routing names must stay inside LLM service descriptors" },
        ForbiddenToken { family: "provider-model-routing-name", token: "\"openrouter\"", rationale: "provider/model routing names must stay inside LLM service descriptors" },
        ForbiddenToken { family: "provider-model-routing-name", token: "\"gpt-", rationale: "model-family routing prefixes must be descriptor data inside the LLM service" },
        ForbiddenToken { family: "provider-model-routing-name", token: "\"claude-", rationale: "model-family routing prefixes must be descriptor data inside the LLM service" },
        ForbiddenToken { family: "provider-model-routing-name", token: "\"qwen", rationale: "model-family routing prefixes must be descriptor data inside the LLM service" },
        ForbiddenToken { family: "provider-model-routing-name", token: "\"deepseek-", rationale: "model-family routing prefixes must be descriptor data inside the LLM service" },
        ForbiddenToken { family: "provider-model-routing-name", token: "\"minimax-", rationale: "model-family routing prefixes must be descriptor data inside the LLM service" },
        ForbiddenToken { family: "autonomy-service-boundary", token: "SchedulerSystemServiceProvider", rationale: "scheduler providers must be constructed only by runtime-host autonomy composition" },
        ForbiddenToken { family: "autonomy-service-boundary", token: "HeartbeatSystemServiceProvider", rationale: "heartbeat providers must be constructed only by runtime-host autonomy composition" },
        ForbiddenToken { family: "autonomy-service-boundary", token: "LocalSchedulerProvider", rationale: "local scheduler engines must remain replaceable service providers (retired name; use InProcessSchedulerProvider in approved surfaces)" },
        ForbiddenToken { family: "autonomy-service-boundary", token: "LocalHeartbeatProvider", rationale: "local heartbeat engines must remain replaceable service providers (retired name; use InProcessHeartbeatProvider in approved surfaces)" },
        ForbiddenToken { family: "autonomy-service-boundary", token: "AutonomySupervisor", rationale: "autonomy loops must remain lifecycle-managed runtime-host infrastructure (retired name; use AutonomyLifecycleCoordinator)" },
        ForbiddenToken { family: "autonomy-loop-boundary", token: "run_scheduler_tick_once", rationale: "scheduler ticks must be owned by runtime-host autonomy supervisor" },
        ForbiddenToken { family: "autonomy-loop-boundary", token: "run_heartbeat_tick_once", rationale: "heartbeat ticks must be owned by runtime-host autonomy supervisor" },
        ForbiddenToken { family: "autonomy-loop-boundary", token: "run_recovery_wake_once", rationale: "recovery wake loops must be owned by runtime-host autonomy supervisor" },
        ForbiddenToken { family: concat!("provider-", "com", "pat", "-construction"), token: "KernelProviderCompat", rationale: "kernel provider bundles must be replaced by service-client AgentExecutionPort wiring" },
        ForbiddenToken { family: concat!("provider-", "com", "pat", "-construction"), token: "LegacyLlmProvider", rationale: "LLM access must flow through service.agent_execution or LLM service clients" },
        ForbiddenToken { family: concat!("provider-", "com", "pat", "-construction"), token: "LegacyToolCatalog", rationale: "tool catalogs must flow through driver/skill/MCP service snapshot commands" },
        ForbiddenToken { family: concat!("provider-", "com", "pat", "-construction"), token: "LegacyAgentExecutionAdapter", rationale: "agent execution must use ServiceClientAgentExecutionAdapter against service.agent_execution" },
        ForbiddenToken { family: "web-direct-runtime-field", token: "state.llm", rationale: "Web must use the LLM service client instead of direct provider handles" },
        ForbiddenToken { family: "web-direct-runtime-field", token: "state.router", rationale: "Web must use service-backed model routing instead of direct router handles" },
        ForbiddenToken { family: "web-direct-runtime-field", token: "state.memory_runtime", rationale: "Web must use memory/context service clients instead of runtime internals" },
        ForbiddenToken { family: "web-direct-runtime-field", token: "state.driver_registry", rationale: "Web must use the driver service client instead of registry internals" },
        ForbiddenToken { family: "web-direct-runtime-field", token: "state.llm_client", rationale: "Web must use focused SDK LLM clients instead of shell-owned provider bridges" },
        ForbiddenToken { family: "direct-runtime-catalog-read", token: "collect_tools()", rationale: "tool catalogs must be fetched through driver service snapshot commands" },
        ForbiddenToken { family: "direct-runtime-catalog-read", token: ".definitions().await", rationale: "MCP definitions must be fetched through MCP service snapshot commands" },
        ForbiddenToken { family: "kernel-non-kernel-module", token: "macaca_kernel::web3", rationale: "Web3 must be accessed through optional module or web3 service providers" },
        ForbiddenToken { family: "kernel-non-kernel-module", token: "macaca_kernel::evm", rationale: "EVM must be accessed through optional module or EVM service providers" },
        ForbiddenToken { family: "kernel-non-kernel-module", token: "macaca_kernel::a2a", rationale: "A2A must be accessed through payment/A2A service providers" },
        ForbiddenToken { family: "kernel-non-kernel-module", token: "macaca_kernel::payment_policy", rationale: "payment policy must be owned by payment service providers" },
        ForbiddenToken { family: "kernel-non-kernel-module", token: "kernel::web3", rationale: "kernel must not grow new Web3 module references" },
        ForbiddenToken { family: "kernel-non-kernel-module", token: "kernel::evm", rationale: "kernel must not grow new EVM module references" },
        ForbiddenToken { family: "kernel-non-kernel-module", token: "kernel::a2a", rationale: "kernel must not grow new A2A module references" },
        ForbiddenToken { family: "kernel-non-kernel-module", token: "kernel::payment_policy", rationale: "kernel must not grow new payment policy references" },
        ForbiddenToken { family: "multi-path-coordination-patch", token: "suppress_executor_lifecycle", rationale: "single execution owner makes lifecycle suppression patches unnecessary" },
        ForbiddenToken { family: "multi-path-coordination-patch", token: concat!("leg", "acy_chat_main_thread_goal_pause"), rationale: "execution-control policy must come from manifest projection, not shell patches" },
        ForbiddenToken { family: "multi-path-coordination-patch", token: concat!("leg", "acy_unmarked"), rationale: "hosted execution must not grow additional retired authority markers" },
        ForbiddenToken { family: "multi-path-coordination-patch", token: "non_authoritative", rationale: "hosted execution must not grow non-authoritative bypass branches" },
        ForbiddenToken { family: "multi-path-coordination-patch", token: "TaskGraphOwner::TaskServiceAuxiliary", rationale: "task graph ownership must converge on application_execution authority only" },
        ForbiddenToken { family: "multi-path-coordination-patch", token: "TaskGraphOwner::DiagnosticOnly", rationale: "diagnostic-only graph owners must not become new execution bypasses" },
    ]
}
