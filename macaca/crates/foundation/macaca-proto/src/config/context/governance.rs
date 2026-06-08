//! Context governance runtime: provider timeouts, redaction, deny lists, and trust promotion.

use serde::{Deserialize, Serialize};

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
