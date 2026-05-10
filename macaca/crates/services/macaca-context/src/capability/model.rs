//! Neutral capability DTOs for the context engine (Adapter / anti-corruption boundary).
//!
//! Upstream runtimes (`macaca-skill`, `macaca-runtime-host`) produce their own snapshots; host
//! crates map those into these structures so `macaca-context` never depends on skill or MCP crates.
//! This preserves the OS layering rule: the context crate only composes text, it does not discover
//! skills or speak MCP transport.

use serde::{Deserialize, Serialize};

/// High-level capability family used in reports and optional policy switches.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityKind {
    SkillKnowledge,
    McpToolSurface,
    /// Local framework / router tool names (not remote MCP).
    RuntimeTool,
}

/// Namespace prefix for stable IDs shown to the model (avoids collisions between sources).
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityNamespace {
    Skill { source_scope: String },
    McpServer { server_id: String },
    RuntimeTools,
}

/// Declared dependency on an abstract capability lane (not a transport handle).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeclaredCapabilityDependency {
    /// Stable category, e.g. `mcp_server:playwright`, `requires_bin:git`.
    pub capability_id: String,
    /// Operator-facing note (optional).
    pub notes: Option<String>,
}

/// Dedup / collision record for audit (namespaced tool keys, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityCollisionRecord {
    pub local_key: String,
    pub claimants: Vec<String>,
}

/// Tier-1 skill row: **no** `SKILL.md` body — metadata only (progressive disclosure).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillCapabilityRecord {
    /// Unique within the catalog for this assembly (used as composer `source_id` fragment).
    pub stable_id: String,
    pub name: String,
    pub description: String,
    pub location_ref: String,
    pub source_label: String,
    pub source_scope: String,
    pub declared_dependencies: Vec<DeclaredCapabilityDependency>,
}

/// Filtered skill diagnostic mirrored from the skill runtime (policy / env / OS gates).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillFilterDiagnostic {
    pub name: String,
    pub reason: String,
}

/// Input bundle for [`super::SkillContextProvider`].
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillCapabilityCatalog {
    pub entries: Vec<SkillCapabilityRecord>,
    pub filtered: Vec<SkillFilterDiagnostic>,
    pub truncated: bool,
}

/// One MCP server's **compact** capability surface (tools list + transport summary).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpServerCapabilitySummary {
    pub server_id: String,
    pub transport: String,
    pub lifecycle: String,
    pub state: String,
    pub exposed_tools: Vec<String>,
}

/// MCP catalog for providers; remote **resource bodies / prompt templates are intentionally absent**.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpCapabilityCatalog {
    pub servers: Vec<McpServerCapabilitySummary>,
    pub collisions: Vec<CapabilityCollisionRecord>,
}

/// Runtime tool names mirrored from framework `Toolkit` JSON definitions (names only).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeToolCapabilityCatalog {
    pub tool_names: Vec<String>,
}
