//! Neutral string identifiers for **built-in** provider families.
//!
//! These values appear in `context.provider_families[].family_id` and registry lookups. They MUST
//! remain free of application or workflow semantics — only technical roles (profile, capability, …).

/// Loads persona / `IDENTITY.md` material when [`crate::profile`] dependencies exist.
pub const FAMILY_AGENT_PROFILE: &str = "agent_profile";

/// Progressive disclosure for skills — metadata index only.
pub const FAMILY_SKILL_CAPABILITY: &str = "skill_capability";

/// MCP server/tool summaries (compact indices).
pub const FAMILY_MCP_CAPABILITY: &str = "mcp_capability";

/// Registered framework tool *names* only.
pub const FAMILY_RUNTIME_TOOL_CAPABILITY: &str = "runtime_tool_capability";

/// Active vector / workspace memory recall path.
pub const FAMILY_MEMORY_ACTIVE_RECALL: &str = "memory_active_recall";

/// Implicit default ordering when `provider_families` in TOML is empty.
pub const BUILTIN_FAMILY_ORDER: &[&str] = &[
    FAMILY_AGENT_PROFILE,
    FAMILY_SKILL_CAPABILITY,
    FAMILY_MCP_CAPABILITY,
    FAMILY_RUNTIME_TOOL_CAPABILITY,
    FAMILY_MEMORY_ACTIVE_RECALL,
];
