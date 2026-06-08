//! MCP configuration and wire-type definitions (transport, tools, resources).

use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::message::ContentBlock;

// ---------------------------------------------------------------------------
// MCP configuration
// ---------------------------------------------------------------------------

/// Transport configuration for an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "transport", rename_all = "snake_case")]
pub enum McpTransportConfig {
    /// Launch a local MCP server process and communicate over stdio.
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: BTreeMap<String, String>,
        #[serde(default)]
        cwd: Option<PathBuf>,
    },
    /// Connect to an MCP server over legacy SSE transport.
    Sse {
        url: String,
        #[serde(default)]
        headers: BTreeMap<String, String>,
    },
    /// Connect to an MCP server over streamable HTTP transport.
    StreamableHttp {
        url: String,
        #[serde(default)]
        headers: BTreeMap<String, String>,
    },
}

/// Session behavior for an MCP client.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum McpSessionMode {
    /// One initialized MCP session is reused until close.
    Stateful,
    /// A scoped MCP session may be opened per logical call by higher layers.
    Stateless,
}

/// Timeout configuration for MCP operations.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpTimeouts {
    pub connect: Duration,
    pub list_tools: Duration,
    pub call_tool: Duration,
}

impl Default for McpTimeouts {
    fn default() -> Self {
        Self {
            connect: Duration::from_secs(15),
            list_tools: Duration::from_secs(15),
            call_tool: Duration::from_secs(60),
        }
    }
}

/// Deterministic policy for MCP tool name collisions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpToolNameConflictPolicy {
    /// Fail registration if any MCP tool name already exists.
    Raise,
    /// Skip colliding tools.
    Skip,
    /// Register tools with this prefix.
    Prefix(String),
}

/// Options used when registering MCP tools into a framework [`Toolkit`].
#[derive(Clone)]
pub struct McpToolRegistrationOptions {
    pub group_name: String,
    pub conflict_policy: McpToolNameConflictPolicy,
    pub disabled_tools: HashSet<String>,
    pub on_close: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl McpToolRegistrationOptions {
    pub fn new(group_name: impl Into<String>) -> Self {
        Self {
            group_name: group_name.into(),
            conflict_policy: McpToolNameConflictPolicy::Raise,
            disabled_tools: HashSet::new(),
            on_close: None,
        }
    }
}

// ---------------------------------------------------------------------------
// MCP type definitions
// ---------------------------------------------------------------------------

/// An MCP tool definition received from the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDef {
    pub name: String,
    pub description: String,
    #[serde(default, alias = "inputSchema")]
    pub input_schema: Value,
}

/// Result from calling an MCP tool.
#[derive(Debug, Clone)]
pub struct McpCallResult {
    pub content: Vec<ContentBlock>,
    pub is_error: bool,
    pub metadata: Option<Value>,
}

/// Resource metadata returned by MCP `resources/list`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpResourceDef {
    pub uri: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, alias = "mimeType")]
    pub mime_type: Option<String>,
}

/// Template metadata returned by MCP `resources/templates/list`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpResourceTemplateDef {
    #[serde(alias = "uriTemplate")]
    pub uri_template: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, alias = "mimeType")]
    pub mime_type: Option<String>,
}

/// Bounded resource content returned by MCP `resources/read`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpResourceRead {
    pub uri: String,
    #[serde(default, alias = "mimeType")]
    pub mime_type: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub blob: Option<String>,
}
