use macaca_proto::{MacacaError, MacacaResult, MemoryEntry, MemoryId};

use crate::core::facade::{
    MemoryDeleteRequest, MemoryFacade, MemoryGetRequest, MemorySearchRequest, MemoryWriteRequest,
};
use crate::core::provider::{MemoryProvider, MemoryProviderDescriptor};
use crate::core::status::MemoryCapabilitySet;
use crate::core::status::MemoryStatusReport;

use super::config::{MemoryProviderConfig, MemoryProviderMcpServerConfig};
use super::resilience::{redact_text, MemoryProviderResilience};
/// MCP-backed memory provider adapter.
///
/// The adapter does not assume a specific MCP transport implementation beyond
/// the configured command and tool names. It keeps the provider pluggable by
/// treating the server as an external capability boundary rather than a Rust
/// dependency.
pub struct McpMemoryProvider {
    provider_id: String,
    display_name: String,
    server: MemoryProviderMcpServerConfig,
    resilience: MemoryProviderResilience,
}

impl McpMemoryProvider {
    /// Build an MCP provider from a memory provider config.
    pub fn new(config: &MemoryProviderConfig, server: MemoryProviderMcpServerConfig) -> Self {
        Self {
            provider_id: config.id.clone(),
            display_name: config
                .display_name
                .clone()
                .unwrap_or_else(|| config.id.clone()),
            server,
            resilience: MemoryProviderResilience::new(&config.resilience),
        }
    }

    fn command_line(&self) -> String {
        let mut parts = vec![self.server.command.clone()];
        parts.extend(self.server.args.clone());
        parts.join(" ")
    }

    fn redact(&self, text: &str) -> String {
        let mut markers = vec![self.server.command.clone()];
        markers.extend(self.server.args.clone());
        markers.extend(self.server.env.values().cloned());
        redact_text(text, &markers)
    }

    /// Report the configured server diagnostics string.
    pub fn diagnostics(&self) -> String {
        self.redact(&self.command_line())
    }
}

#[async_trait::async_trait]
impl MemoryFacade for McpMemoryProvider {
    async fn remember(&self, request: MemoryWriteRequest) -> MacacaResult<MemoryId> {
        self.resilience
            .execute("mcp.write", self.command_line().len(), || async {
                let _ = request;
                Err(MacacaError::Memory(
                    "MCP write transport is not wired to a live client yet".into(),
                ))
            })
            .await
    }

    async fn search(&self, request: MemorySearchRequest) -> MacacaResult<Vec<MemoryEntry>> {
        self.resilience
            .execute("mcp.search", self.command_line().len(), || async {
                let _ = request;
                Err(MacacaError::Memory(
                    "MCP search transport is not wired to a live client yet".into(),
                ))
            })
            .await
    }

    async fn get(&self, request: MemoryGetRequest) -> MacacaResult<Option<MemoryEntry>> {
        self.resilience
            .execute("mcp.get", self.command_line().len(), || async {
                let _ = request;
                Err(MacacaError::Memory(
                    "MCP get transport is not wired to a live client yet".into(),
                ))
            })
            .await
    }

    async fn delete(&self, request: MemoryDeleteRequest) -> MacacaResult<()> {
        self.resilience
            .execute("mcp.delete", self.command_line().len(), || async {
                let _ = request;
                Err(MacacaError::Memory(
                    "MCP delete transport is not wired to a live client yet".into(),
                ))
            })
            .await
    }

    fn status(&self) -> MemoryStatusReport {
        MemoryStatusReport::healthy(
            self.provider_id.clone(),
            MemoryCapabilitySet {
                store: true,
                search: true,
                prompt: true,
                lifecycle: true,
                flush: true,
                artifact: false,
                governance: true,
            },
        )
    }
}

impl MemoryProvider for McpMemoryProvider {
    fn descriptor(&self) -> MemoryProviderDescriptor {
        MemoryProviderDescriptor::new(
            self.provider_id.clone(),
            self.display_name.clone(),
            MemoryCapabilitySet {
                store: true,
                search: true,
                prompt: true,
                lifecycle: true,
                flush: true,
                artifact: false,
                governance: true,
            },
        )
    }
}

/// Helper for callers that only need a diagnostics string.
pub fn mcp_provider_diagnostics(provider: &McpMemoryProvider) -> String {
    provider.diagnostics()
}
