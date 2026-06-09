//! Explicit MCP session lease token for runtime-host instance ownership.
//!
//! **Value Object / RAII token**: `McpSessionLease` wraps a single `McpRuntimeKey` so MCP
//! runtime maps can track which session owns a spawned server instance. The lease does not
//! perform teardown itself — callers return the key to the runtime manager when releasing
//! resources, which keeps lifecycle policy in one place and makes double-free bugs visible.
//!
//! Trade-off: lightweight newtype over `McpRuntimeKey` instead of full RAII Drop guards,
//! because MCP teardown may require async context not available in `Drop`.

use crate::mcp_runtime::McpRuntimeKey;

/// Explicit lease representing ownership of one runtime-host MCP instance key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpSessionLease {
    key: McpRuntimeKey,
}

impl McpSessionLease {
    pub fn new(key: McpRuntimeKey) -> Self {
        Self { key }
    }

    pub fn key(&self) -> &McpRuntimeKey {
        &self.key
    }

    pub fn into_key(self) -> McpRuntimeKey {
        self.key
    }
}
