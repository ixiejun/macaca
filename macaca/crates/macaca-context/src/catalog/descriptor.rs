//! Serializable metadata rows for **catalog diagnostics** and the registry’s `descriptor()`.  
//! This stays PII-free: versions and capability **tags** only.

use serde::{Deserialize, Serialize};

use super::constants::{
    BUILTIN_FAMILY_ORDER, FAMILY_AGENT_PROFILE, FAMILY_KNOWLEDGE_DIGEST, FAMILY_MCP_CAPABILITY,
    FAMILY_MEMORY_ACTIVE_RECALL, FAMILY_RUNTIME_TOOL_CAPABILITY, FAMILY_SKILL_CAPABILITY,
};

/// Crate semantic version (`CARGO_PKG_VERSION`) reused for all first-party built-ins.
pub const MACACA_CONTEXT_CRATE_SEMVER: &str = env!("CARGO_PKG_VERSION");

/// Describes one installable provider family (registry plugin or built-in shim).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderFamilyDescriptor {
    pub family_id: String,
    /// Best-effort semver for the **provider implementation** (crate, plugin, or adapter).
    pub implementation_version: String,
    #[serde(default)]
    /// Unstructured capability tags for operator UIs (`"builtin"`, `"experimental"`, …).
    pub capability_tags: Vec<String>,
}

impl ProviderFamilyDescriptor {
    /// Convenience constructor to avoid repeated boilerplate in tests.
    pub fn builtin(family_id: impl Into<String>, tags: &[&str]) -> Self {
        Self {
            family_id: family_id.into(),
            implementation_version: MACACA_CONTEXT_CRATE_SEMVER.to_string(),
            capability_tags: tags.iter().copied().map(String::from).collect(),
        }
    }
}

/// Returns descriptors for every neutral built-in family — **independent** of whether the
/// current environment can actually instantiate them (kernel may omit catalogs).
pub fn list_builtin_family_descriptors() -> Vec<ProviderFamilyDescriptor> {
    BUILTIN_FAMILY_ORDER
        .iter()
        .map(|fid| match *fid {
            FAMILY_AGENT_PROFILE => {
                ProviderFamilyDescriptor::builtin(FAMILY_AGENT_PROFILE, &["builtin", "profile"])
            }
            FAMILY_SKILL_CAPABILITY => ProviderFamilyDescriptor::builtin(
                FAMILY_SKILL_CAPABILITY,
                &["builtin", "capability_index", "skill"],
            ),
            FAMILY_MCP_CAPABILITY => ProviderFamilyDescriptor::builtin(
                FAMILY_MCP_CAPABILITY,
                &["builtin", "capability_index", "mcp"],
            ),
            FAMILY_RUNTIME_TOOL_CAPABILITY => ProviderFamilyDescriptor::builtin(
                FAMILY_RUNTIME_TOOL_CAPABILITY,
                &["builtin", "capability_index", "tool"],
            ),
            FAMILY_KNOWLEDGE_DIGEST => ProviderFamilyDescriptor::builtin(
                FAMILY_KNOWLEDGE_DIGEST,
                &["builtin", "memory", "governance", "digest"],
            ),
            FAMILY_MEMORY_ACTIVE_RECALL => ProviderFamilyDescriptor::builtin(
                FAMILY_MEMORY_ACTIVE_RECALL,
                &["builtin", "memory", "recall"],
            ),
            _ => ProviderFamilyDescriptor::builtin(*fid, &["builtin"]),
        })
        .collect()
}
