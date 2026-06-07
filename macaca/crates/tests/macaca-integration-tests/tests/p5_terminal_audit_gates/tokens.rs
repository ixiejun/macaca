//! Forbidden-token catalog for P5 terminal audit gates (subset of escape-hatch freeze).
//!
//! Tokens are grouped by **family** so each gate (Strategy) can filter the catalog
//! without duplicating scan logic. Keep this list aligned with
//! `serviceization_escape_hatches.rs` when adding freeze rules.

/// One forbidden source literal with governance metadata.
pub struct ForbiddenToken {
    pub family: &'static str,
    pub token: &'static str,
    pub rationale: &'static str,
}

macro_rules! token {
    ($family:expr, $token:expr, $rationale:expr) => {
        ForbiddenToken {
            family: $family,
            token: $token,
            rationale: $rationale,
        }
    };
}

/// Returns the P5-relevant forbidden tokens shared across audit gates.
pub fn forbidden_tokens() -> Vec<ForbiddenToken> {
    vec![
        // --- no-direct-provider-call / shell-not-semantic-owner ---
        token!(
            "application-runtime-direct-start",
            "AppRuntime::start_app",
            "application lifecycle must enter through Application Service commands"
        ),
        token!(
            "application-runtime-direct-start",
            "start_app_from_file",
            "file-backed application starts must pass through traced service commands"
        ),
        token!(
            "web-direct-runtime-field",
            "state.driver_runtime",
            "presentation shells must use driver service clients instead of runtime internals"
        ),
        token!(
            "web-direct-runtime-field",
            "state.mcp_runtime",
            "presentation shells must use MCP service snapshot commands"
        ),
        token!(
            "web-direct-runtime-field",
            "state.runtime",
            "presentation shells must use application service clients instead of AppRuntime anchors"
        ),
        token!(
            "web-direct-runtime-field",
            "state.registry",
            "presentation shells must use service-backed application metadata views"
        ),
        token!(
            "web-direct-runtime-field",
            "state.llm",
            "presentation shells must use LLM service clients instead of direct provider handles"
        ),
        token!(
            "web-direct-runtime-field",
            "state.router",
            "presentation shells must use service-backed model routing"
        ),
        token!(
            "web-direct-runtime-field",
            "state.memory_runtime",
            "presentation shells must use memory/context service clients"
        ),
        token!(
            "web-direct-runtime-field",
            "state.driver_registry",
            "presentation shells must use driver service clients"
        ),
        token!(
            "web-direct-runtime-field",
            "state.llm_client",
            "presentation shells must use focused SDK LLM clients"
        ),
        token!(
            "provider-compat-construction",
            "KernelProviderCompat",
            "provider bundles must be replaced by service-client execution wiring"
        ),
        token!(
            "provider-compat-construction",
            "LegacyLlmProvider",
            "LLM access must flow through service clients"
        ),
        token!(
            "provider-compat-construction",
            "LegacyToolCatalog",
            "tool catalogs must flow through driver/skill/MCP service snapshots"
        ),
        token!(
            "provider-compat-construction",
            "LegacyAgentExecutionAdapter",
            "agent execution must use ServiceClientAgentExecutionAdapter"
        ),
        token!(
            "direct-runtime-catalog-read",
            "collect_tools()",
            "tool catalogs must be fetched through driver service snapshot commands"
        ),
        token!(
            "direct-runtime-catalog-read",
            ".definitions().await",
            "MCP definitions must be fetched through MCP service snapshot commands"
        ),
        // --- shell-not-semantic-owner (execution ownership) ---
        token!(
            "shell-semantic-execution-owner",
            "agent.reply(",
            "shell must delegate agent turns to service.agent_execution, not execute in-shell"
        ),
        token!(
            "shell-semantic-execution-owner",
            "FrameworkRunner::build_runtime_agent",
            "runtime agent construction belongs in runtime-host service providers"
        ),
        // --- no-hardcoded-name ---
        token!(
            "hardcoded-agent-role",
            "\"coordinator\"",
            "agent role names must come from manifests or service descriptors"
        ),
        token!(
            "hardcoded-agent-role",
            "\"planner\"",
            "agent role names must come from manifests or service descriptors"
        ),
        token!(
            "hardcoded-agent-role",
            "\"worker\"",
            "agent role names must come from manifests or service descriptors"
        ),
        token!(
            "hardcoded-agent-role",
            "\"backend\"",
            "agent role names must come from manifests or service descriptors"
        ),
        token!(
            "hardcoded-agent-role",
            "\"frontend\"",
            "agent role names must come from manifests or service descriptors"
        ),
        token!(
            "hardcoded-agent-role",
            "\"architect\"",
            "agent role names must come from manifests or service descriptors"
        ),
        token!(
            "provider-model-routing-name",
            "\"openai\"",
            "provider routing names must stay inside LLM service descriptors"
        ),
        token!(
            "provider-model-routing-name",
            "\"anthropic\"",
            "provider routing names must stay inside LLM service descriptors"
        ),
        token!(
            "provider-model-routing-name",
            "\"dashscope\"",
            "provider routing names must stay inside LLM service descriptors"
        ),
        token!(
            "provider-model-routing-name",
            "\"deepseek\"",
            "provider routing names must stay inside LLM service descriptors"
        ),
        token!(
            "provider-model-routing-name",
            "\"minimax\"",
            "provider routing names must stay inside LLM service descriptors"
        ),
        token!(
            "provider-model-routing-name",
            "\"openrouter\"",
            "provider routing names must stay inside LLM service descriptors"
        ),
        token!(
            "provider-model-routing-name",
            "\"gpt-",
            "model-family prefixes must be descriptor data inside the LLM service"
        ),
        token!(
            "provider-model-routing-name",
            "\"claude-",
            "model-family prefixes must be descriptor data inside the LLM service"
        ),
        token!(
            "provider-model-routing-name",
            "\"qwen",
            "model-family prefixes must be descriptor data inside the LLM service"
        ),
        token!(
            "provider-model-routing-name",
            "\"deepseek-",
            "model-family prefixes must be descriptor data inside the LLM service"
        ),
        token!(
            "provider-model-routing-name",
            "\"minimax-",
            "model-family prefixes must be descriptor data inside the LLM service"
        ),
    ]
}
