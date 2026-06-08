//! YAML registry entry → runtime definition conversion (Adapter).
//!
//! Translates operator-authored `mcp.yaml` entries into transport configs,
//! applying declarative concurrency-isolation policies from the mapping registry.

use std::path::PathBuf;

use macaca_framework::mcp::{McpSessionMode, McpTransportConfig};

use crate::skill_mcp_mapping_registry::{default_skill_mcp_mapping_registry, SkillMcpMappingRegistry};

use super::policy::apply_concurrency_isolation;
use super::types::{McpDefinitionSource, McpServerConfigEntry, McpServerDefinition};

impl McpServerConfigEntry {
    pub(crate) fn into_definition_with_registry(
        self,
        id: String,
        registry: &SkillMcpMappingRegistry,
    ) -> Result<McpServerDefinition, String> {
        let transport = match self.transport.as_str() {
            "stdio" => {
                let command = self
                    .command
                    .ok_or_else(|| format!("MCP server {id} missing command"))?;
                let policy = self
                    .concurrency_isolation
                    .clone()
                    .or_else(|| registry.policy_for_command(&command));
                let args = policy
                    .as_ref()
                    .map(|p| apply_concurrency_isolation(p, self.args.clone()))
                    .unwrap_or(self.args);
                McpTransportConfig::Stdio {
                    command,
                    args,
                    env: self.env,
                    cwd: self.cwd,
                }
            }
            "sse" => McpTransportConfig::Sse {
                url: self
                    .url
                    .ok_or_else(|| format!("MCP server {id} missing url"))?,
                headers: self.headers,
            },
            "streamable_http" => McpTransportConfig::StreamableHttp {
                url: self
                    .url
                    .ok_or_else(|| format!("MCP server {id} missing url"))?,
                headers: self.headers,
            },
            other => return Err(format!("Unsupported MCP transport: {other}")),
        };
        Ok(McpServerDefinition {
            id,
            transport,
            lifecycle: self.lifecycle,
            session_mode: self.session_mode,
            tool_prefix: self.tool_prefix,
            required_bins: self.required_bins,
            enabled: self.enabled,
            source: McpDefinitionSource::Global,
            concurrency_isolation: self.concurrency_isolation,
        })
    }

    pub(crate) fn into_definition(self, id: String) -> Result<McpServerDefinition, String> {
        self.into_definition_with_registry(id, default_skill_mcp_mapping_registry())
    }
}

pub(crate) fn default_mcp_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".macaca").join("mcp.yaml"))
}
