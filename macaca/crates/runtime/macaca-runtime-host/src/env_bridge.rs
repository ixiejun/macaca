//! Process-wide MCP environment export.
//!
//! [`MacacaConfig::mcp.env`](macaca_proto::McpConfigSection) declares key/value
//! pairs that the backend should re-publish into its own process environment
//! before spawning any MCP child process. Because stdio MCP clients rely on
//! `tokio::process::Command` default env inheritance, this is the cleanest way
//! to deliver secrets (Figma API key, etc.) to MCP servers without coupling
//! their schema to a bespoke transport field.
//!
//! Value semantics mirror [`LlmProviderConfig::resolve_api_key`]:
//! - empty / whitespace-only → skipped
//! - `ALL_CAPS_WITH_UNDERSCORES` identifier → treated as the *name* of an
//!   environment variable that must already exist; value is forwarded verbatim
//!   (handy for `FIGMA_API_KEY = "FIGMA_API_KEY"` style indirection when
//!   operators prefer shell export)
//! - anything else → treated as a literal secret and set verbatim
//!
//! Any entry whose value is empty OR whose *value* looks like a placeholder
//! (starts with `REPLACE_`, `YOUR_`, or equals `<...>`) is logged at INFO and
//! skipped so committing a scaffolded `config/default.toml` never overwrites a
//! real env var with a placeholder.

use std::collections::HashMap;

use tracing::{info, warn};

/// Outcome of a single entry application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpEnvApplyOutcome {
    /// Key exported with a literal value.
    SetLiteral,
    /// Key exported by forwarding an existing env var.
    ForwardedFromEnv,
    /// Value matched a placeholder pattern; entry ignored.
    SkippedPlaceholder,
    /// Value referenced an env var that is not set; entry ignored.
    SkippedMissingEnvVar,
    /// Value was empty/whitespace; entry ignored.
    SkippedEmpty,
}

fn looks_like_placeholder(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.is_empty()
        || trimmed.starts_with("YOUR_")
        || trimmed.starts_with("REPLACE_")
        || (trimmed.starts_with('<') && trimmed.ends_with('>'))
}

fn looks_like_env_var_name(value: &str) -> bool {
    let v = value.trim();
    !v.is_empty()
        && v.chars()
            .all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit())
}

/// Decide what should happen for a single `(key, value)` pair **without**
/// touching the process environment. Exposed for unit tests; the real work is
/// done by [`apply_mcp_env`].
pub fn classify_entry(value: &str) -> McpEnvApplyOutcome {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return McpEnvApplyOutcome::SkippedEmpty;
    }
    if looks_like_placeholder(trimmed) {
        return McpEnvApplyOutcome::SkippedPlaceholder;
    }
    if looks_like_env_var_name(trimmed) {
        // Resolution happens at apply time (needs real env lookup).
        McpEnvApplyOutcome::ForwardedFromEnv
    } else {
        McpEnvApplyOutcome::SetLiteral
    }
}

/// Re-export every `[mcp.env]` entry into the current process environment.
///
/// Returns a per-key outcome map so operators can see in logs which secrets
/// were applied, forwarded, or skipped.
///
/// The `config` crate normalises TOML keys to lowercase during deserialization
/// (to unify with its nested-env-var override source). POSIX env var names are
/// conventionally uppercase, so we explicitly uppercase each key here before
/// calling `set_var`. The returned `HashMap`'s keys use the same uppercased
/// form so callers can log a stable identifier.
#[deprecated(note = "Use `RuntimeEnvBuilder::apply_process_env` instead.")]
pub fn apply_mcp_env(env: &HashMap<String, String>) -> HashMap<String, McpEnvApplyOutcome> {
    let mut outcomes = HashMap::with_capacity(env.len());
    for (raw_key, value) in env {
        let key = raw_key.to_ascii_uppercase();
        let outcome = classify_entry(value);
        match &outcome {
            McpEnvApplyOutcome::SetLiteral => {
                std::env::set_var(&key, value.trim());
                info!(key = %key, "MCP env set from config (literal)");
            }
            McpEnvApplyOutcome::ForwardedFromEnv => {
                let forwarded = value.trim();
                match std::env::var(forwarded) {
                    Ok(val) => {
                        std::env::set_var(&key, val);
                        info!(key = %key, from = %forwarded, "MCP env forwarded from existing shell env");
                    }
                    Err(_) => {
                        warn!(
                            key = %key,
                            from = %forwarded,
                            "MCP env config references missing env var; skipped"
                        );
                        outcomes.insert(key, McpEnvApplyOutcome::SkippedMissingEnvVar);
                        continue;
                    }
                }
            }
            McpEnvApplyOutcome::SkippedPlaceholder => {
                info!(key = %key, "MCP env placeholder detected; skipped");
            }
            McpEnvApplyOutcome::SkippedEmpty => {
                // Completely empty — skip silently at DEBUG-ish level.
            }
            McpEnvApplyOutcome::SkippedMissingEnvVar => {
                // Shouldn't be produced by classify_entry; included for completeness.
            }
        }
        outcomes.insert(key, outcome);
    }
    outcomes
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::*;

    #[test]
    fn classify_entry_recognises_placeholders() {
        assert_eq!(
            classify_entry("YOUR_FIGMA_TOKEN_HERE"),
            McpEnvApplyOutcome::SkippedPlaceholder
        );
        assert_eq!(
            classify_entry("<paste-token>"),
            McpEnvApplyOutcome::SkippedPlaceholder
        );
        assert_eq!(
            classify_entry("REPLACE_ME"),
            McpEnvApplyOutcome::SkippedPlaceholder
        );
        assert_eq!(classify_entry(""), McpEnvApplyOutcome::SkippedEmpty);
        assert_eq!(classify_entry("   "), McpEnvApplyOutcome::SkippedEmpty);
    }

    #[test]
    fn classify_entry_distinguishes_env_ref_vs_literal() {
        assert_eq!(
            classify_entry("FIGMA_API_KEY"),
            McpEnvApplyOutcome::ForwardedFromEnv
        );
        assert_eq!(
            classify_entry("figd_abcdef123"),
            McpEnvApplyOutcome::SetLiteral
        );
        assert_eq!(
            classify_entry("sk-plan-xxxxx"),
            McpEnvApplyOutcome::SetLiteral
        );
    }

    #[test]
    fn apply_sets_literal_and_skips_placeholder() {
        let mut env = HashMap::new();
        env.insert(
            "MCP_TEST_LITERAL_EXPORT".into(),
            "figd_exported_via_apply".into(),
        );
        env.insert("MCP_TEST_PLACEHOLDER_SKIP".into(), "YOUR_TOKEN_HERE".into());

        let outcomes = apply_mcp_env(&env);

        assert_eq!(
            outcomes.get("MCP_TEST_LITERAL_EXPORT"),
            Some(&McpEnvApplyOutcome::SetLiteral)
        );
        assert_eq!(
            std::env::var("MCP_TEST_LITERAL_EXPORT").ok().as_deref(),
            Some("figd_exported_via_apply")
        );
        assert_eq!(
            outcomes.get("MCP_TEST_PLACEHOLDER_SKIP"),
            Some(&McpEnvApplyOutcome::SkippedPlaceholder)
        );
        assert!(std::env::var("MCP_TEST_PLACEHOLDER_SKIP").is_err());

        // Cleanup to avoid leaking into other tests running in the same process.
        std::env::remove_var("MCP_TEST_LITERAL_EXPORT");
    }

    #[test]
    fn apply_forwards_from_existing_env() {
        std::env::set_var("MCP_TEST_SRC_PRESENT", "real-value-from-shell");

        let mut env = HashMap::new();
        env.insert("MCP_TEST_DST_PRESENT".into(), "MCP_TEST_SRC_PRESENT".into());

        let outcomes = apply_mcp_env(&env);
        assert_eq!(
            outcomes.get("MCP_TEST_DST_PRESENT"),
            Some(&McpEnvApplyOutcome::ForwardedFromEnv)
        );
        assert_eq!(
            std::env::var("MCP_TEST_DST_PRESENT").ok().as_deref(),
            Some("real-value-from-shell")
        );

        std::env::remove_var("MCP_TEST_SRC_PRESENT");
        std::env::remove_var("MCP_TEST_DST_PRESENT");
    }

    #[test]
    fn apply_skips_when_forwarded_source_missing() {
        // Make sure source is absent.
        std::env::remove_var("MCP_TEST_SRC_MISSING");

        let mut env = HashMap::new();
        env.insert("MCP_TEST_DST_MISSING".into(), "MCP_TEST_SRC_MISSING".into());

        let outcomes = apply_mcp_env(&env);
        assert_eq!(
            outcomes.get("MCP_TEST_DST_MISSING"),
            Some(&McpEnvApplyOutcome::SkippedMissingEnvVar)
        );
        assert!(std::env::var("MCP_TEST_DST_MISSING").is_err());
    }

    #[test]
    fn apply_uppercases_lowercase_toml_keys() {
        // The `config` crate lowercases TOML keys; ensure apply restores POSIX
        // convention so downstream MCP servers (e.g. figma-developer-mcp
        // expecting FIGMA_API_KEY) find the variable.
        let mut env = HashMap::new();
        env.insert("mcp_test_lowercase_input".into(), "literal-value-99".into());

        let outcomes = apply_mcp_env(&env);
        assert_eq!(
            outcomes.get("MCP_TEST_LOWERCASE_INPUT"),
            Some(&McpEnvApplyOutcome::SetLiteral),
            "outcome map should be keyed by the uppercased name"
        );
        assert_eq!(
            std::env::var("MCP_TEST_LOWERCASE_INPUT").ok().as_deref(),
            Some("literal-value-99")
        );
        assert!(
            std::env::var("mcp_test_lowercase_input").is_err(),
            "the lowercase form must NOT be set"
        );

        std::env::remove_var("MCP_TEST_LOWERCASE_INPUT");
    }
}
