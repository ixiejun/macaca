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
