//! MCP runtime environment forwarding configuration.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

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
