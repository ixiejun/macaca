use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::error::{MacacaError, MacacaResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacacaConfig {
    pub kernel: KernelConfig,
    pub llm: LlmConfig,
    #[serde(default)]
    pub context: ContextConfig,
    pub memory: MemoryConfig,
    pub ipc: IpcConfig,
    pub persist: PersistConfig,
    pub gateway: GatewayConfig,
    pub observability: ObservabilityConfig,
    #[serde(default)]
    pub workspace: WorkspaceConfig,
    #[serde(default)]
    pub mcp: McpConfigSection,
    #[serde(default)]
    pub drivers: DriversConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceGuideSourcesConfig {
    #[serde(default = "default_workspace_guide_entries")]
    pub entries: Vec<WorkspaceGuideEntry>,
}

impl Default for WorkspaceGuideSourcesConfig {
    fn default() -> Self {
        Self {
            entries: default_workspace_guide_entries(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceGuideEntry {
    pub relative_path: String,
    #[serde(default)]
    pub priority: i32,
    #[serde(default = "default_workspace_guide_max_bytes")]
    pub max_bytes: u32,
}

fn default_workspace_guide_max_bytes() -> u32 {
    16 * 1024
}

fn default_workspace_guide_entries() -> Vec<WorkspaceGuideEntry> {
    vec![
        WorkspaceGuideEntry {
            relative_path: "AGENTS.md".into(),
            priority: 0,
            max_bytes: default_workspace_guide_max_bytes(),
        },
        WorkspaceGuideEntry {
            relative_path: "SOUL.md".into(),
            priority: 10,
            max_bytes: default_workspace_guide_max_bytes(),
        },
        WorkspaceGuideEntry {
            relative_path: "TOOLS.md".into(),
            priority: 20,
            max_bytes: default_workspace_guide_max_bytes(),
        },
        WorkspaceGuideEntry {
            relative_path: "IDENTITY.md".into(),
            priority: 30,
            max_bytes: default_workspace_guide_max_bytes(),
        },
        WorkspaceGuideEntry {
            relative_path: "USER.md".into(),
            priority: 40,
            max_bytes: default_workspace_guide_max_bytes(),
        },
        WorkspaceGuideEntry {
            relative_path: "HEARTBEAT.md".into(),
            priority: 50,
            max_bytes: default_workspace_guide_max_bytes(),
        },
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextRecallRuntimeConfig {
    /// When true, the web runtime exposes read-only `memory_search` / `memory_get` tools.
    #[serde(default)]
    pub expose_memory_tools: bool,
    #[serde(default = "default_memory_search_limit")]
    pub memory_search_default_limit: u32,
    #[serde(default)]
    pub preflight_recall_enabled: bool,
    #[serde(default)]
    pub preflight_allowed_tools: Vec<String>,
    #[serde(default = "default_preflight_timeout_ms")]
    pub preflight_timeout_ms: u64,
    #[serde(default = "default_preflight_max_chars")]
    pub preflight_max_chars: usize,
    #[serde(default = "default_preflight_max_tokens")]
    pub preflight_max_tokens: u32,
    #[serde(default)]
    pub preflight_fatal_on_failure: bool,
}

fn default_memory_search_limit() -> u32 {
    8
}

fn default_preflight_timeout_ms() -> u64 {
    1_500
}

fn default_preflight_max_chars() -> usize {
    4_000
}

fn default_preflight_max_tokens() -> u32 {
    1_000
}

impl Default for ContextRecallRuntimeConfig {
    fn default() -> Self {
        Self {
            expose_memory_tools: false,
            memory_search_default_limit: default_memory_search_limit(),
            preflight_recall_enabled: false,
            preflight_allowed_tools: Vec::new(),
            preflight_timeout_ms: default_preflight_timeout_ms(),
            preflight_max_chars: default_preflight_max_chars(),
            preflight_max_tokens: default_preflight_max_tokens(),
            preflight_fatal_on_failure: false,
        }
    }
}

/// Where to resolve agent profile Markdown files (`AGENTS.md`, `SOUL.md`, etc.).
///
/// The OS layer stays application-agnostic: resolution uses only configured paths and
/// well-known filenames — never hard-coded business agent names.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AgentProfileRootKind {
    /// Application install: `{app_dir}/personas/{agent_name}` (same tree as `IDENTITY.md` / `TOOLS.md`).
    #[default]
    PersonaDirectory,
    /// Data workspace private sandbox: `{data_dir}/workspaces/{app_id}/agents/{agent_name}`.
    AgentPrivateWorkspace,
}

fn default_agent_profile_max_file_bytes() -> u64 {
    2 * 1024 * 1024
}

fn default_agent_profile_inject_heartbeat() -> bool {
    true
}

fn default_agent_profile_include_memory_seed() -> bool {
    true
}

/// Tunables for [`crate::config::ContextConfig::agent_profile`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentProfileContextConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub root_kind: AgentProfileRootKind,
    /// Per-file byte budget before UTF-8 truncation/skip diagnostics.
    #[serde(default = "default_agent_profile_max_file_bytes")]
    pub max_file_bytes: u64,
    /// When false, `HEARTBEAT.md` is never read by the profile provider (operators may gate cadence elsewhere).
    #[serde(default = "default_agent_profile_inject_heartbeat")]
    pub inject_heartbeat: bool,
    /// When false, `MEMORY.md` is omitted from candidates (seed/audit injection disabled for this agent).
    #[serde(default = "default_agent_profile_include_memory_seed")]
    pub include_memory_seed: bool,
    /// When > 0, reject profile bodies exceeding this **line** count after frontmatter stripping.
    /// `0` disables the check (default).
    #[serde(default)]
    pub profile_max_content_lines: u32,
}

impl Default for AgentProfileContextConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            root_kind: AgentProfileRootKind::default(),
            max_file_bytes: default_agent_profile_max_file_bytes(),
            inject_heartbeat: default_agent_profile_inject_heartbeat(),
            include_memory_seed: default_agent_profile_include_memory_seed(),
            profile_max_content_lines: 0,
        }
    }
}

/// Tunables for composer-stage **active vector memory** recall (`MemoryActiveRecallContextProvider`).
///
/// This controls whether long-term memory is prefetched through [`macaca_context::ActiveRecallCapability`]
/// before each model call. It is intentionally separate from tool-exposed memory APIs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActiveVectorMemoryContextConfig {
    /// Master switch: when false, no composer recall runs (explicit memory tools remain governed elsewhere).
    #[serde(default)]
    pub enabled: bool,
    /// Include hits tagged for the **current agent** private route when routing metadata exists.
    #[serde(default = "default_avm_include_agent_private")]
    pub include_agent_private: bool,
    /// Include session-scoped shared hits (typically entries without a competing agent owner).
    #[serde(default = "default_avm_include_session_shared")]
    pub include_session_shared: bool,
    /// Hard limit around the `ActiveRecallCapability::prefetch` async call (fail-open on expiry).
    #[serde(default = "default_avm_timeout_ms")]
    pub timeout_ms: u64,
    /// Passed through [`macaca_context::MemoryRecallQuery::max_tokens`] as a soft ceiling for providers.
    #[serde(default = "default_avm_max_query_tokens")]
    pub max_query_tokens: u32,
    /// Budget knobs mapped into [`macaca_context::ActiveRecallPolicy`].
    #[serde(default = "default_avm_max_hits")]
    pub max_hits: usize,
    #[serde(default = "default_avm_max_chars")]
    pub max_chars: usize,
    #[serde(default = "default_avm_max_tokens_budget")]
    pub max_tokens: u32,
}

fn default_avm_include_agent_private() -> bool {
    true
}

fn default_avm_include_session_shared() -> bool {
    true
}

fn default_avm_timeout_ms() -> u64 {
    1_500
}

fn default_avm_max_query_tokens() -> u32 {
    2_048
}

fn default_avm_max_hits() -> usize {
    8
}

fn default_avm_max_chars() -> usize {
    4_000
}

fn default_avm_max_tokens_budget() -> u32 {
    1_000
}

impl Default for ActiveVectorMemoryContextConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            include_agent_private: default_avm_include_agent_private(),
            include_session_shared: default_avm_include_session_shared(),
            timeout_ms: default_avm_timeout_ms(),
            max_query_tokens: default_avm_max_query_tokens(),
            max_hits: default_avm_max_hits(),
            max_chars: default_avm_max_chars(),
            max_tokens: default_avm_max_tokens_budget(),
        }
    }
}

/// Controls [`macaca_context::KnowledgeDigestContextProvider`] **and** digest-vs-raw suppression in
/// [`macaca_context::ContextFacade::assemble_model_context`].
///
/// The memory governance layer does the heavy lifting (claims, tombstones, redaction). This struct
/// only tunes how compiled digests enter the composer and how they interact with raw vector recall
/// candidates (Strategy pattern — tunable thresholds without altering memory fabric code).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeDigestContextConfig {
    /// Enables the provider family and the post-provider digest-vs-raw merge pass.
    #[serde(default)]
    pub enabled: bool,
    /// Wall-clock ceiling for `KnowledgeDigestCapability::digest_for_request` inside the provider.
    #[serde(default = "default_knowledge_digest_timeout_ms")]
    pub timeout_ms: u64,
    /// Minimum [`KnowledgeClaim::confidence`] scalar for a digest to participate in **suppression**
    /// of overlapping raw recall (0..=1 domain matching memory compiler conventions).
    #[serde(default = "default_knowledge_digest_min_confidence")]
    pub min_confidence_for_suppression: f32,
    /// Minimum [`KnowledgeClaim::freshness`] for a digest to be classified as **strong** (not stale).
    #[serde(default = "default_knowledge_digest_min_freshness_strong")]
    pub min_freshness_strong: f32,
    /// Freshness at or below this cutoff is **stale**: such digests never suppress fresh raw recall.
    #[serde(default = "default_knowledge_digest_stale_cutoff")]
    pub stale_freshness_cutoff: f32,
    /// Minimum number of evidence rows referenced by a claim before suppression logic activates.
    #[serde(default = "default_knowledge_digest_min_evidence")]
    pub min_evidence_count: usize,
    /// Result row ceiling forwarded to the workspace adapter when synthesizing compiler inputs.
    #[serde(default = "default_knowledge_digest_max_compiler_rows")]
    pub max_compiler_rows: usize,
}

fn default_knowledge_digest_timeout_ms() -> u64 {
    1_200
}

fn default_knowledge_digest_min_confidence() -> f32 {
    0.55
}

fn default_knowledge_digest_min_freshness_strong() -> f32 {
    0.35
}

fn default_knowledge_digest_stale_cutoff() -> f32 {
    0.22
}

fn default_knowledge_digest_min_evidence() -> usize {
    1
}

fn default_knowledge_digest_max_compiler_rows() -> usize {
    24
}

impl Default for KnowledgeDigestContextConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            timeout_ms: default_knowledge_digest_timeout_ms(),
            min_confidence_for_suppression: default_knowledge_digest_min_confidence(),
            min_freshness_strong: default_knowledge_digest_min_freshness_strong(),
            stale_freshness_cutoff: default_knowledge_digest_stale_cutoff(),
            min_evidence_count: default_knowledge_digest_min_evidence(),
            max_compiler_rows: default_knowledge_digest_max_compiler_rows(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    #[serde(default = "default_context_engine")]
    pub default_engine: String,
    #[serde(default = "default_context_fallback_engine")]
    pub fallback_engine: String,
    #[serde(default = "default_context_emit_reports")]
    pub emit_reports: bool,
    #[serde(default = "default_context_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_context_reserve_output_tokens")]
    pub reserve_output_tokens: u32,
    #[serde(default)]
    pub workspace_guides: WorkspaceGuideSourcesConfig,
    #[serde(default)]
    pub recall: ContextRecallRuntimeConfig,
    #[serde(default)]
    pub agent_profile: AgentProfileContextConfig,
    #[serde(default)]
    pub active_vector_memory: ActiveVectorMemoryContextConfig,
    /// Governed knowledge digest provider + digest-vs-raw merge (OpenSpec `knowledge-digest-context`).
    #[serde(default)]
    pub knowledge_digest: KnowledgeDigestContextConfig,
    /// Provider-runtime governance: timeouts, redaction, allow/deny, and failure isolation
    /// applied at the [`macaca_context::ContextFacade`] composer boundary.
    #[serde(default)]
    pub governance: ContextGovernanceRuntimeConfig,
    /// Ordered, configuration-driven selection of built-in or registry-backed provider families.
    /// Empty means: use the documented built-in default order inside the catalog assembler.
    #[serde(default)]
    pub provider_families: Vec<ContextProviderFamilyConfig>,
    /// Declarative external context adapters installed into the runtime at startup.
    ///
    /// These rows remain application-agnostic: they describe transport, safety, and fallback
    /// policy only. The web/runtime layer decides how to instantiate each transport and then
    /// overlays the resulting engine into the neutral `ContextEngineRegistry`.
    #[serde(default)]
    pub external_adapters: Vec<ContextExternalAdapterConfig>,
    /// Optional trust promotion rules (`ContextCandidate::trust`) evaluated after redaction/deny
    /// and before composer budgeting — application-neutral pattern matching only.
    #[serde(default)]
    pub trust_governance: ContextTrustGovernanceConfig,
}

/// One configured provider **family** row. `family_id` matches stable neutral keys consumed by
/// [`macaca_context::catalog`] (for example `agent_profile`, `skill_capability`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextProviderFamilyConfig {
    pub family_id: String,
    #[serde(default = "default_context_provider_family_enabled")]
    pub enabled: bool,
    /// Opaque JSON parameters forwarded to [`macaca_context::ProviderFactoryInput::params`].
    #[serde(default)]
    pub params: serde_json::Value,
}

fn default_context_provider_family_enabled() -> bool {
    true
}

fn default_context_external_adapter_enabled() -> bool {
    true
}

fn default_external_adapter_transport() -> ContextExternalAdapterTransportKind {
    ContextExternalAdapterTransportKind::HttpJson
}

/// Transport kinds supported by the Phase 9 external adapter installer.
///
/// Additional transports can be added without changing the surrounding config shape: each row keeps
/// neutral engine metadata plus transport-specific endpoint settings.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ContextExternalAdapterTransportKind {
    /// POST [`macaca_context::ContextAssembleInput`] as JSON and expect
    /// [`macaca_context::ContextAssembleResult`] JSON in response.
    #[default]
    HttpJson,
}

/// Endpoint settings for the `http_json` external adapter transport.
///
/// Header values follow the same convention used elsewhere in config:
/// - literal strings are sent verbatim
/// - `ALL_CAPS_WITH_UNDERSCORES` values are treated as environment variable names
///   and resolved at startup
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextExternalAdapterHttpJsonConfig {
    pub url: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

fn default_external_adapter_timeout_ms() -> u64 {
    2_000
}

fn default_external_adapter_max_payload_bytes() -> usize {
    256 * 1024
}

fn default_external_adapter_require_schema_validation() -> bool {
    true
}

fn default_external_adapter_require_budget_validation() -> bool {
    true
}

fn default_external_adapter_circuit_breaker_failures() -> u32 {
    3
}

/// Runtime safety guardrails mapped into `macaca-context` external adapter validation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextExternalAdapterSafetyConfig {
    #[serde(default = "default_external_adapter_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default = "default_external_adapter_max_payload_bytes")]
    pub max_payload_bytes: usize,
    #[serde(default = "default_external_adapter_require_schema_validation")]
    pub require_schema_validation: bool,
    #[serde(default = "default_external_adapter_require_budget_validation")]
    pub require_budget_validation: bool,
    #[serde(default = "default_external_adapter_circuit_breaker_failures")]
    pub circuit_breaker_failures: u32,
}

impl Default for ContextExternalAdapterSafetyConfig {
    fn default() -> Self {
        Self {
            timeout_ms: default_external_adapter_timeout_ms(),
            max_payload_bytes: default_external_adapter_max_payload_bytes(),
            require_schema_validation: default_external_adapter_require_schema_validation(),
            require_budget_validation: default_external_adapter_require_budget_validation(),
            circuit_breaker_failures: default_external_adapter_circuit_breaker_failures(),
        }
    }
}

fn default_external_adapter_fallback_engine_id() -> String {
    "legacy".into()
}

fn default_external_adapter_empty_contribution_fallback() -> bool {
    true
}

/// Fallback behavior used when an external adapter times out, errors, or emits an invalid result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextExternalAdapterFallbackConfig {
    #[serde(default = "default_external_adapter_fallback_engine_id")]
    pub fallback_engine_id: String,
    #[serde(default = "default_external_adapter_empty_contribution_fallback")]
    pub empty_external_contribution: bool,
}

impl Default for ContextExternalAdapterFallbackConfig {
    fn default() -> Self {
        Self {
            fallback_engine_id: default_external_adapter_fallback_engine_id(),
            empty_external_contribution: default_external_adapter_empty_contribution_fallback(),
        }
    }
}

/// One declarative external adapter engine installation row.
///
/// The `id` becomes the selectable `ContextEngineInfo.id`. Operators can choose it through
/// `context.default_engine`, app manifest context, or agent-level overrides once the runtime
/// installs the row successfully.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextExternalAdapterConfig {
    pub id: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default = "default_context_external_adapter_enabled")]
    pub enabled: bool,
    #[serde(default = "default_external_adapter_transport")]
    pub transport: ContextExternalAdapterTransportKind,
    #[serde(default)]
    pub http_json: Option<ContextExternalAdapterHttpJsonConfig>,
    #[serde(default)]
    pub safety: ContextExternalAdapterSafetyConfig,
    #[serde(default)]
    pub fallback: ContextExternalAdapterFallbackConfig,
}

/// Rule-based promotion of `TrustLevel` on [`macaca_context::composer::ContextCandidate`].
///
/// All matching is structural (`source_id` prefix, optional kind filter) — never app/workflow keys.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextTrustGovernanceConfig {
    #[serde(default)]
    pub promotions: Vec<TrustPromotionRule>,
}

impl Default for ContextTrustGovernanceConfig {
    fn default() -> Self {
        Self {
            promotions: Vec::new(),
        }
    }
}

/// Single promotion rule. String trust levels mirror [`macaca_context::prompt::TrustLevel`] serde:
/// `trusted` | `untrusted`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrustPromotionRule {
    /// When non-empty, `source_id` must start with this literal prefix.
    #[serde(default)]
    pub match_source_id_prefix: String,
    /// Optional [`macaca_context::composer::ContextCandidateKind`] name in `snake_case`
    /// (for example `capability_index`).
    #[serde(default)]
    pub match_candidate_kind: Option<String>,
    /// Only applies when `candidate.trust` is **at most** this level (ordering: untrusted < trusted).
    #[serde(default = "default_rule_trust_at_most")]
    pub if_trust_at_most: String,
    #[serde(default = "default_rule_promote_to")]
    pub promote_to: String,
}

fn default_rule_trust_at_most() -> String {
    "untrusted".into()
}

fn default_rule_promote_to() -> String {
    "trusted".into()
}

fn default_context_engine() -> String {
    "legacy".into()
}

fn default_context_fallback_engine() -> String {
    "legacy".into()
}

fn default_context_emit_reports() -> bool {
    true
}

fn default_context_max_tokens() -> u32 {
    120_000
}

fn default_context_reserve_output_tokens() -> u32 {
    4_096
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            default_engine: default_context_engine(),
            fallback_engine: default_context_fallback_engine(),
            emit_reports: default_context_emit_reports(),
            max_tokens: default_context_max_tokens(),
            reserve_output_tokens: default_context_reserve_output_tokens(),
            workspace_guides: WorkspaceGuideSourcesConfig::default(),
            recall: ContextRecallRuntimeConfig::default(),
            agent_profile: AgentProfileContextConfig::default(),
            active_vector_memory: ActiveVectorMemoryContextConfig::default(),
            knowledge_digest: KnowledgeDigestContextConfig::default(),
            governance: ContextGovernanceRuntimeConfig::default(),
            provider_families: Vec::new(),
            external_adapters: Vec::new(),
            trust_governance: ContextTrustGovernanceConfig::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// Context governance / provider runtime (Neutral policy surface for macaca-context)
// ---------------------------------------------------------------------------

fn default_governance_enabled() -> bool {
    false
}

fn default_governance_provider_timeout_ms() -> u64 {
    30_000
}

fn default_governance_fail_open() -> bool {
    true
}

/// Tunables for the governed provider pipeline inside `ContextFacade`.
///
/// This configuration is intentionally **application-agnostic**: it references
/// technical policies only (`source_id` prefixes, substrings) — never app, workflow,
/// or business names as selection keys.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextGovernanceRuntimeConfig {
    /// When true, each `ContextProvider::contribute` is wrapped with governance (timeouts, filters).
    /// Defaults to **false** so embedding callers preserve the historical ungoverned fan-in unless
    /// configuration explicitly opts in.
    #[serde(default = "default_governance_enabled")]
    pub enabled: bool,
    /// Maximum wall-clock time to wait on any single `ContextProvider::contribute` call.
    #[serde(default = "default_governance_provider_timeout_ms")]
    pub per_provider_timeout_ms: u64,
    /// When true, provider errors/timeouts become diagnostics while remaining providers run.
    #[serde(default = "default_governance_fail_open")]
    pub fail_open_on_provider_error: bool,
    /// After governance filters, optionally cap total **declared** candidate tokens (0 = disabled).
    /// The composer still applies its own budget; this is an extra guardrail at the facade.
    #[serde(default)]
    pub max_total_candidate_tokens: u32,
    /// Substrings removed from candidate text (replaced with `[REDACTED]`) — use for known secret
    /// markers; avoid overly broad patterns that destroy model-visible semantics.
    #[serde(default)]
    pub redact_substrings: Vec<String>,
    /// Drop candidates whose `source_id` starts with any of these literal prefixes.
    #[serde(default)]
    pub deny_source_id_prefixes: Vec<String>,
    /// Optional operator-visible label duplicated into context reports for reproducibility.
    #[serde(default)]
    pub policy_label: String,
}

impl Default for ContextGovernanceRuntimeConfig {
    fn default() -> Self {
        Self {
            enabled: default_governance_enabled(),
            per_provider_timeout_ms: default_governance_provider_timeout_ms(),
            fail_open_on_provider_error: default_governance_fail_open(),
            max_total_candidate_tokens: 0,
            redact_substrings: Vec::new(),
            deny_source_id_prefixes: Vec::new(),
            policy_label: String::new(),
        }
    }
}

pub struct MacacaConfigBuilder {
    inner: MacacaConfig,
}

impl MacacaConfigBuilder {
    pub fn new() -> Self {
        Self {
            inner: MacacaConfig::default(),
        }
    }

    pub fn kernel(mut self, kernel: KernelConfig) -> Self {
        self.inner.kernel = kernel;
        self
    }

    pub fn llm(mut self, llm: LlmConfig) -> Self {
        self.inner.llm = llm;
        self
    }

    pub fn workspace(mut self, workspace: WorkspaceConfig) -> Self {
        self.inner.workspace = workspace;
        self
    }

    pub fn drivers(mut self, drivers: DriversConfig) -> Self {
        self.inner.drivers = drivers;
        self
    }

    pub fn build(self) -> MacacaConfig {
        self.inner
    }
}

impl Default for MacacaConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// External driver plugin configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriversConfig {
    /// Directory containing external driver plugins (relative to working dir or absolute).
    #[serde(default = "default_drivers_directory")]
    pub directory: String,
    /// Whether to automatically load all drivers at startup.
    #[serde(default = "default_auto_load")]
    pub auto_load: bool,
}

fn default_drivers_directory() -> String {
    "drivers".to_string()
}

fn default_auto_load() -> bool {
    true
}

impl Default for DriversConfig {
    fn default() -> Self {
        Self {
            directory: default_drivers_directory(),
            auto_load: default_auto_load(),
        }
    }
}

/// MCP runtime configuration. Currently exposes a key/value `env` map that is
/// re-exported into the backend process environment during `start_server` so
/// every downstream MCP child process inherits the declared values
/// (stdio MCP clients rely on `tokio::process::Command` env inheritance).
///
/// Values follow the same "literal vs env-var-name" convention as LLM keys:
/// if the value looks like an `ALL_CAPS_WITH_UNDERSCORES` identifier it is
/// interpreted as the name of an existing environment variable to forward;
/// otherwise it is treated as a literal value and set verbatim.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpConfigSection {
    #[serde(default)]
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub root_dir: String,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            root_dir: "./data/workspaces".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelConfig {
    pub max_agents: usize,
    pub heartbeat_interval_ms: u64,
    pub agent_timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub default_provider: String,
    pub default_model: Option<String>,
    pub max_tokens_per_request: u32,
    pub rate_limit_rpm: u32,
    pub providers: HashMap<String, LlmProviderConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProviderConfig {
    /// Coding-plan / subscription key (e.g. MiniMax [Token Plan](https://platform.minimaxi.com/docs/token-plan/intro)).
    /// When non-empty after resolution, **takes precedence** over [`Self::api_key`] (pay-as-you-go).
    /// Value: raw key, or `ALL_CAPS` env var name (same rules as `api_key`).
    #[serde(default)]
    pub api_key_plan: Option<String>,
    /// Pay-as-you-go API key: raw `sk-…` string, or env var name if `ALL_CAPS_WITH_UNDERSCORES`
    /// (e.g. `OPENAI_API_KEY`). Used when `api_key_plan` is unset or empty.
    #[serde(default)]
    pub api_key: String,
    pub base_url: String,
    /// Default model for this provider (e.g. "" for DashScope, "gpt-4o" for OpenAI)
    #[serde(default)]
    pub default_model: Option<String>,
}

pub struct LlmProviderConfigBuilder {
    inner: LlmProviderConfig,
}

impl LlmProviderConfigBuilder {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            inner: LlmProviderConfig {
                api_key_plan: None,
                api_key: String::new(),
                base_url: base_url.into(),
                default_model: None,
            },
        }
    }

    pub fn api_key_plan(mut self, api_key_plan: impl Into<String>) -> Self {
        self.inner.api_key_plan = Some(api_key_plan.into());
        self
    }

    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.inner.api_key = api_key.into();
        self
    }

    pub fn default_model(mut self, default_model: impl Into<String>) -> Self {
        self.inner.default_model = Some(default_model.into());
        self
    }

    pub fn build(self) -> LlmProviderConfig {
        self.inner
    }
}

/// Resolve one key field: empty → `Ok("")`; `ALL_CAPS` → `std::env::var`; else literal.
fn resolve_llm_key_field(raw: &str) -> MacacaResult<String> {
    let v = raw.trim();
    if v.is_empty() {
        return Ok(String::new());
    }
    let is_env_var_name = v
        .chars()
        .all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit());
    if is_env_var_name {
        std::env::var(v).map_err(|_| MacacaError::Config(format!("{v} not set")))
    } else {
        Ok(v.to_string())
    }
}

fn resolve_llm_optional_key(opt: &Option<String>) -> MacacaResult<String> {
    match opt {
        None => Ok(String::new()),
        Some(s) => resolve_llm_key_field(s),
    }
}

impl LlmProviderConfig {
    /// Effective API key: **`api_key_plan` (coding plan) first**, then **`api_key` (按量)**.
    pub fn resolve_api_key(&self) -> MacacaResult<String> {
        let plan = resolve_llm_optional_key(&self.api_key_plan)?;
        if !plan.is_empty() {
            return Ok(plan);
        }
        resolve_llm_key_field(&self.api_key)
    }
}

#[cfg(test)]
mod llm_provider_config_tests {
    use super::*;

    #[test]
    fn resolve_api_key_prefers_api_key_plan() {
        let c = LlmProviderConfig {
            api_key_plan: Some("  sk-plan  ".into()),
            api_key: "SHOULD_NOT_READ".into(),
            base_url: "https://example.com/v1".into(),
            default_model: None,
        };
        assert_eq!(c.resolve_api_key().unwrap(), "sk-plan");
    }

    #[test]
    fn resolve_api_key_falls_back_to_api_key_paygo() {
        let c = LlmProviderConfig {
            api_key_plan: None,
            api_key: "sk-paygo-inline".into(),
            base_url: "https://example.com/v1".into(),
            default_model: None,
        };
        assert_eq!(c.resolve_api_key().unwrap(), "sk-paygo-inline");
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub session_ttl_seconds: u64,
    pub file_store_path: String,
    pub auto_retrieve_on: String,
    pub vector: VectorConfig,
    pub embedding: EmbeddingConfig,
    pub compression: CompressionConfig,
    #[serde(default)]
    pub provider_runtime: MemoryProviderRuntimeConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryProviderEndpointConfig {
    pub url: String,
    #[serde(default)]
    pub api_key_env: Option<String>,
    #[serde(default)]
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryProviderTransportKind {
    #[default]
    Builtin,
    Remote,
    Mcp,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryProviderResilienceConfig {
    #[serde(default = "default_memory_provider_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default = "default_memory_provider_payload_limit_bytes")]
    pub payload_limit_bytes: usize,
    #[serde(default = "default_memory_provider_retry_count")]
    pub retry_count: u32,
    #[serde(default = "default_memory_provider_circuit_threshold")]
    pub circuit_failure_threshold: u32,
    #[serde(default = "default_memory_provider_circuit_cooldown_ms")]
    pub circuit_cooldown_ms: u64,
}

fn default_memory_provider_timeout_ms() -> u64 {
    5_000
}

fn default_memory_provider_payload_limit_bytes() -> usize {
    256 * 1024
}

fn default_memory_provider_retry_count() -> u32 {
    1
}

fn default_memory_provider_circuit_threshold() -> u32 {
    3
}

fn default_memory_provider_circuit_cooldown_ms() -> u64 {
    30_000
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryProviderToolConfig {
    pub name: String,
    #[serde(default)]
    pub namespaced: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryProviderComponentSlotConfig {
    #[serde(default)]
    pub agent_private_provider: Option<String>,
    #[serde(default)]
    pub session_shared_provider: Option<String>,
    #[serde(default)]
    pub embedding_provider: Option<String>,
    #[serde(default)]
    pub vector_backend: Option<String>,
    #[serde(default)]
    pub active_recall: Option<String>,
    #[serde(default)]
    pub knowledge_compiler: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryProviderConfig {
    pub id: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub transport: MemoryProviderTransportKind,
    #[serde(default)]
    pub endpoint: Option<MemoryProviderEndpointConfig>,
    #[serde(default)]
    pub resilience: MemoryProviderResilienceConfig,
    #[serde(default)]
    pub tools: Vec<MemoryProviderToolConfig>,
    #[serde(default)]
    pub components: MemoryProviderComponentSlotConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryProviderProfileConfig {
    #[serde(default)]
    pub agent_private_provider: Option<String>,
    #[serde(default)]
    pub session_shared_provider: Option<String>,
    #[serde(default)]
    pub embedding_provider: Option<String>,
    #[serde(default)]
    pub vector_backend: Option<String>,
    #[serde(default)]
    pub active_recall: Option<String>,
    #[serde(default)]
    pub knowledge_compiler: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryProviderProfilesConfig {
    #[serde(default)]
    pub default_profile: String,
    #[serde(default)]
    pub profiles: HashMap<String, MemoryProviderProfileConfig>,
    #[serde(default)]
    pub agents: HashMap<String, MemoryProviderProfileConfig>,
    #[serde(default)]
    pub sessions: HashMap<String, MemoryProviderProfileConfig>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryProviderRuntimeConfig {
    #[serde(default)]
    pub profile: String,
    #[serde(default)]
    pub providers: HashMap<String, MemoryProviderConfig>,
    #[serde(default)]
    pub profiles: MemoryProviderProfilesConfig,
    #[serde(default)]
    pub mcp: MemoryProviderMcpConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryProviderMcpConfig {
    #[serde(default)]
    pub servers: HashMap<String, MemoryProviderMcpServerConfig>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryProviderMcpServerConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub timeout_ms: u64,
    #[serde(default)]
    pub trust_external: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorConfig {
    pub backend: String,
    pub milvus_url: String,
    pub collection_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub provider: String,
    pub model: String,
    /// Raw API key or `ALL_CAPS` env var name (same rules as [`LlmProviderConfig::api_key`]).
    pub api_key: String,
    pub dimensions: usize,
    /// Base URL for the embedding API endpoint.
    #[serde(default)]
    pub base_url: String,
}

impl EmbeddingConfig {
    pub fn resolve_api_key(&self) -> MacacaResult<String> {
        resolve_llm_key_field(&self.api_key)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    pub enabled: bool,
    pub threshold_entries: usize,
    pub strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcConfig {
    pub nats_url: String,
    pub nats_auto_start: bool,
    pub reconnect_max_attempts: u32,
    pub reconnect_delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistConfig {
    pub engine: String,
    pub data_dir: String,
    pub snapshot_interval_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    pub enabled: bool,
    pub telegram: Option<TelegramConfig>,
    pub discord: Option<DiscordConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub enabled: bool,
    pub bot_token_env: String,
    pub allowed_user_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    pub enabled: bool,
    pub bot_token_env: String,
    pub command_prefix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    pub log_level: String,
    pub tracing_enabled: bool,
    pub otlp_endpoint: String,
    /// File logging configuration
    #[serde(default)]
    pub log_file: LogFileConfig,
}

/// Configuration for file-based logging with rotation and compression.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogFileConfig {
    /// Enable file logging
    #[serde(default = "default_log_file_enabled")]
    pub enabled: bool,
    /// Directory to store log files
    #[serde(default = "default_log_dir")]
    pub dir: String,
    /// Log file name prefix
    #[serde(default = "default_log_prefix")]
    pub prefix: String,
    /// Log format: "json" or "text"
    #[serde(default = "default_log_format")]
    pub format: String,
    /// Maximum number of days to retain log files
    #[serde(default = "default_log_retention_days")]
    pub retention_days: u64,
    /// Compress old log files
    #[serde(default = "default_log_compress")]
    pub compress: bool,
}

fn default_log_file_enabled() -> bool {
    true
}
fn default_log_dir() -> String {
    "./logs".into()
}
fn default_log_prefix() -> String {
    "macaca".into()
}
fn default_log_format() -> String {
    "json".into()
}
fn default_log_retention_days() -> u64 {
    10
}
fn default_log_compress() -> bool {
    true
}

impl Default for LogFileConfig {
    fn default() -> Self {
        Self {
            enabled: default_log_file_enabled(),
            dir: default_log_dir(),
            prefix: default_log_prefix(),
            format: default_log_format(),
            retention_days: default_log_retention_days(),
            compress: default_log_compress(),
        }
    }
}

impl Default for MacacaConfig {
    fn default() -> Self {
        Self {
            kernel: KernelConfig {
                max_agents: 16,
                heartbeat_interval_ms: 5000,
                agent_timeout_ms: 30000,
            },
            llm: LlmConfig {
                default_provider: "anthropic".into(),
                default_model: None,
                max_tokens_per_request: 8192,
                rate_limit_rpm: 60,
                providers: HashMap::new(),
            },
            context: ContextConfig::default(),
            memory: MemoryConfig {
                session_ttl_seconds: 3600,
                file_store_path: "./data/memory/files".into(),
                auto_retrieve_on: "task_start".into(),
                vector: VectorConfig {
                    backend: "milvus".into(),
                    milvus_url: "http://localhost:19530".into(),
                    collection_name: "agent_memory".into(),
                },
                embedding: EmbeddingConfig {
                    provider: "dashscope".into(),
                    model: "text-embedding-v4".into(),
                    api_key: "DASHSCOPE_API_KEY".into(),
                    dimensions: 1024,
                    base_url: "https://dashscope.aliyuncs.com/api/v1/services/embeddings/text-embedding/text-embedding".into(),
                },
                compression: CompressionConfig {
                    enabled: true,
                    threshold_entries: 100,
                    strategy: "llm_summarize".into(),
                },
                provider_runtime: MemoryProviderRuntimeConfig::default(),
            },
            ipc: IpcConfig {
                nats_url: "nats://localhost:4222".into(),
                nats_auto_start: true,
                reconnect_max_attempts: 10,
                reconnect_delay_ms: 1000,
            },
            persist: PersistConfig {
                engine: "redb".into(),
                data_dir: "./data/persist".into(),
                snapshot_interval_seconds: 300,
            },
            gateway: GatewayConfig {
                enabled: true,
                telegram: Some(TelegramConfig {
                    enabled: true,
                    bot_token_env: "TELEGRAM_BOT_TOKEN".into(),
                    allowed_user_ids: Vec::new(),
                }),
                discord: Some(DiscordConfig {
                    enabled: true,
                    bot_token_env: "DISCORD_BOT_TOKEN".into(),
                    command_prefix: "!".into(),
                }),
            },
            observability: ObservabilityConfig {
                log_level: "info".into(),
                tracing_enabled: true,
                otlp_endpoint: String::new(),
                log_file: LogFileConfig::default(),
            },
            workspace: WorkspaceConfig::default(),
            mcp: McpConfigSection::default(),
            drivers: DriversConfig::default(),
        }
    }
}

impl MacacaConfig {
    /// Load config from a TOML file, with environment variable overrides.
    /// Env vars use the format: AOS_SECTION__KEY (double underscore for nesting).
    pub fn load(path: impl AsRef<Path>) -> MacacaResult<Self> {
        let builder = config::Config::builder()
            .add_source(config::File::from(path.as_ref()).required(false))
            .add_source(
                config::Environment::with_prefix("AOS")
                    .separator("__")
                    .try_parsing(true),
            )
            .build()
            .map_err(|e| MacacaError::Config(e.to_string()))?;

        builder
            .try_deserialize()
            .map_err(|e| MacacaError::Config(e.to_string()))
    }

    /// Load from default path (config/default.toml) or fall back to defaults.
    pub fn load_default() -> Self {
        Self::load("config/default.toml").unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_recall_is_disabled_by_default() {
        let cfg = MacacaConfig::default();
        assert!(!cfg.context.recall.expose_memory_tools);
        assert!(!cfg.context.recall.preflight_recall_enabled);
    }

    #[test]
    fn default_config_is_valid() {
        let cfg = MacacaConfig::default();
        assert_eq!(cfg.kernel.max_agents, 16);
        assert_eq!(cfg.context.default_engine, "legacy");
        assert!(cfg.context.emit_reports);
        assert!(!cfg.context.agent_profile.enabled);
        assert_eq!(cfg.context.workspace_guides.entries.len(), 6);
        assert!(cfg.context.external_adapters.is_empty());
        assert_eq!(cfg.memory.embedding.model, "text-embedding-v4");
        assert_eq!(cfg.memory.vector.backend, "milvus");
        assert_eq!(cfg.memory.embedding.dimensions, 1024);
        assert!(cfg.memory.provider_runtime.providers.is_empty());
        assert!(cfg.gateway.telegram.unwrap().enabled);
    }

    #[test]
    fn load_nonexistent_falls_back_to_default() {
        let cfg = MacacaConfig::load_default();
        assert_eq!(cfg.persist.engine, "redb");
    }

    #[test]
    fn macaca_config_builder_matches_default_then_overrides() {
        let built = MacacaConfigBuilder::new()
            .workspace(WorkspaceConfig {
                root_dir: "/tmp/workspaces".into(),
            })
            .drivers(DriversConfig {
                directory: "custom-drivers".into(),
                auto_load: false,
            })
            .build();

        assert_eq!(
            built.kernel.max_agents,
            MacacaConfig::default().kernel.max_agents
        );
        assert_eq!(built.workspace.root_dir, "/tmp/workspaces");
        assert_eq!(built.drivers.directory, "custom-drivers");
        assert!(!built.drivers.auto_load);
    }

    #[test]
    fn llm_provider_config_builder_matches_manual_construction() {
        let manual = LlmProviderConfig {
            api_key_plan: Some("PLAN_KEY".into()),
            api_key: "PAYGO_KEY".into(),
            base_url: "https://example.com/v1".into(),
            default_model: Some("gpt-test".into()),
        };

        let built = LlmProviderConfigBuilder::new("https://example.com/v1")
            .api_key_plan("PLAN_KEY")
            .api_key("PAYGO_KEY")
            .default_model("gpt-test")
            .build();

        assert_eq!(built.api_key_plan, manual.api_key_plan);
        assert_eq!(built.api_key, manual.api_key);
        assert_eq!(built.base_url, manual.base_url);
        assert_eq!(built.default_model, manual.default_model);
    }

    #[test]
    fn macaca_config_builder_preserves_serde_shape() {
        let json = serde_json::to_value(MacacaConfigBuilder::new().build()).unwrap();
        assert!(json.get("kernel").is_some());
        assert!(json.get("llm").is_some());
        assert!(json.get("drivers").is_some());
    }

    #[test]
    fn load_external_context_adapter_config_from_toml() {
        let raw = r#"
            [kernel]
            max_agents = 1
            heartbeat_interval_ms = 1000
            agent_timeout_ms = 1000

            [llm]
            default_provider = "test"
            max_tokens_per_request = 1
            rate_limit_rpm = 1

            [llm.providers.test]
            api_key = "KEY"
            base_url = "https://example.com"

            [memory]
            session_ttl_seconds = 1
            file_store_path = "./tmp"
            auto_retrieve_on = "task_start"

            [memory.vector]
            backend = "milvus"
            milvus_url = "http://localhost:19530"
            collection_name = "agent_memory"

            [memory.embedding]
            provider = "dashscope"
            model = "text-embedding-v4"
            api_key = "KEY"
            dimensions = 1024
            base_url = "https://example.com"

            [memory.compression]
            enabled = false
            threshold_entries = 1
            strategy = "none"

            [ipc]
            nats_url = "nats://localhost:4222"
            nats_auto_start = false
            reconnect_max_attempts = 1
            reconnect_delay_ms = 1

            [persist]
            engine = "redb"
            data_dir = "./data"
            snapshot_interval_seconds = 1

            [gateway]
            enabled = false

            [observability]
            log_level = "info"
            tracing_enabled = false
            otlp_endpoint = ""

            [[context.external_adapters]]
            id = "workspace-sidecar"
            transport = "http_json"

            [context.external_adapters.http_json]
            url = "http://127.0.0.1:8787/assemble"
            headers = { AUTHORIZATION = "EXTERNAL_CONTEXT_TOKEN" }

            [context.external_adapters.fallback]
            fallback_engine_id = "legacy"
            empty_external_contribution = true
        "#;

        let cfg: MacacaConfig = toml::from_str(raw).unwrap();
        assert_eq!(cfg.context.external_adapters.len(), 1);
        let adapter = &cfg.context.external_adapters[0];
        assert_eq!(adapter.id, "workspace-sidecar");
        assert_eq!(
            adapter.transport,
            ContextExternalAdapterTransportKind::HttpJson
        );
        assert_eq!(
            adapter
                .http_json
                .as_ref()
                .unwrap()
                .url,
            "http://127.0.0.1:8787/assemble"
        );
        assert_eq!(
            adapter
                .http_json
                .as_ref()
                .unwrap()
                .headers
                .get("AUTHORIZATION")
                .unwrap(),
            "EXTERNAL_CONTEXT_TOKEN"
        );
    }
}
