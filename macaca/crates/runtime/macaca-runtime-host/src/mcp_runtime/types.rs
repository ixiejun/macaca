//! MCP runtime value objects, definitions, and policy types.
//!
//! Centralizes serializable configuration models and runtime context keys used
//! across facade, manager, probe, and skill-definition modules.

use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

use macaca_framework::mcp::{McpSessionMode, McpTransportConfig};
use macaca_proto::ApplicationId;
use serde::{Deserialize, Serialize};

/// Lifecycle scope for Agent OS managed MCP instances.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum McpLifecycleScope {
    Global,
    App,
    Session,
    AgentSession,
    Call,
}

impl Default for McpLifecycleScope {
    fn default() -> Self {
        Self::Session
    }
}

/// Declarative concurrency isolation policy applied to stdio transport args.
///
/// Use this instead of hard-coded product-specific branches (e.g. "ensure
/// `--isolated` when command contains `playwright`"). The policy is
/// populated from stable operator mappings or from `McpServerConfigEntry`
/// YAML declarations.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ConcurrencyIsolationPolicy {
    /// Args that MUST be present after policy application.
    #[serde(default)]
    pub required_args: Vec<String>,
    /// If any existing arg starts with one of these prefixes, the policy is
    /// a no-op (operator already overrode isolation behavior explicitly).
    #[serde(default)]
    pub skip_if_any_arg_prefix: Vec<String>,
}

/// Agent OS MCP server definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerDefinition {
    pub id: String,
    #[serde(flatten)]
    pub transport: McpTransportConfig,
    #[serde(default)]
    pub lifecycle: McpLifecycleScope,
    #[serde(default = "default_session_mode")]
    pub session_mode: McpSessionMode,
    #[serde(default)]
    pub tool_prefix: Option<String>,
    #[serde(default)]
    pub required_bins: Vec<String>,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub source: McpDefinitionSource,
    /// Declarative concurrency isolation policy — carried on the definition
    /// for audit/trace purposes. Applied at build time to stdio args.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub concurrency_isolation: Option<ConcurrencyIsolationPolicy>,
}

/// Source that produced an MCP server definition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum McpDefinitionSource {
    Global,
    App,
    Skill,
    Mapping,
}

impl Default for McpDefinitionSource {
    fn default() -> Self {
        Self::Global
    }
}

fn default_session_mode() -> McpSessionMode {
    McpSessionMode::Stateful
}

/// YAML file model for `~/.macaca/mcp.yaml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpRegistryConfig {
    #[serde(default, rename = "mcpServers")]
    pub mcp_servers: BTreeMap<String, McpServerConfigEntry>,
}

/// Per-server YAML config entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfigEntry {
    pub transport: String,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub cwd: Option<PathBuf>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub lifecycle: McpLifecycleScope,
    #[serde(default = "default_session_mode")]
    pub session_mode: McpSessionMode,
    #[serde(default, rename = "toolPrefix")]
    pub tool_prefix: Option<String>,
    #[serde(default, rename = "requiredBins")]
    pub required_bins: Vec<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Declarative concurrency-isolation policy authored by the operator
    /// (YAML schema: `concurrency_isolation: { required_args, skip_if_any_arg_prefix }`).
    #[serde(default, rename = "concurrencyIsolation")]
    pub concurrency_isolation: Option<ConcurrencyIsolationPolicy>,
}

fn default_true() -> bool {
    true
}

/// Runtime status state for an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum McpRuntimeStatusState {
    Ready,
    Failed,
    DependencyMissing,
    Disabled,
}

/// Redacted status returned by APIs and trace payloads.
#[derive(Debug, Clone, Serialize)]
pub struct McpRuntimeStatus {
    pub server_id: String,
    pub transport: String,
    pub lifecycle: McpLifecycleScope,
    pub session_mode: McpSessionMode,
    pub state: McpRuntimeStatusState,
    pub exposed_tools: Vec<String>,
    pub failure_reason: Option<String>,
}

/// Agent/application policy for MCP visibility.
#[derive(Debug, Clone, Default)]
pub struct McpToolPolicy {
    pub allow_servers: Option<HashSet<String>>,
    pub deny_servers: HashSet<String>,
    pub allow_tools: Option<HashSet<String>>,
    pub deny_tools: HashSet<String>,
}

impl McpToolPolicy {
    pub(crate) fn allows_server(&self, server_id: &str) -> bool {
        if self.deny_servers.contains(server_id) {
            return false;
        }
        self.allow_servers
            .as_ref()
            .map(|allow| allow.contains(server_id))
            .unwrap_or(true)
    }

    pub(crate) fn allows_tool(&self, tool_name: &str) -> bool {
        if self.deny_tools.contains(tool_name) {
            return false;
        }
        self.allow_tools
            .as_ref()
            .map(|allow| allow.contains(tool_name))
            .unwrap_or(true)
    }
}

/// Runtime context used to compute lifecycle keys and trace ownership.
#[derive(Debug, Clone, Default)]
pub struct McpRuntimeContext {
    pub app_id: Option<ApplicationId>,
    pub session_id: Option<String>,
    pub agent_name: Option<String>,
}

impl McpRuntimeContext {
    pub fn for_agent(app_id: &ApplicationId, session_id: Option<&str>, agent_name: &str) -> Self {
        Self {
            app_id: Some(app_id.clone()),
            session_id: session_id.map(ToString::to_string),
            agent_name: Some(agent_name.to_string()),
        }
    }
}

/// Runtime instance key used for lifecycle accounting.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct McpRuntimeKey {
    pub server_id: String,
    pub scope: McpLifecycleScope,
    pub app_id: Option<String>,
    pub session_id: Option<String>,
    pub agent_name: Option<String>,
}
