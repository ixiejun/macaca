//! Core `SoftwareDriver` trait and supporting types.

use async_trait::async_trait;
use macaca_proto::{DriverId, MacacaResult};
use macaca_tools::Tool;
use serde::{Deserialize, Serialize};

/// The type of interaction a driver uses to control its target software.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DriverType {
    /// Controls CLI programs via subprocess (stdin/stdout).
    CliSubprocess,
    /// Interacts via REST or GraphQL APIs.
    RestApi,
    /// Controls GUI applications via accessibility/AppleScript.
    UiAutomation,
    /// Communicates via files or IPC pipes.
    FileIpc,
    /// Connects to an MCP server.
    McpProtocol,
}

/// Metadata describing an installed driver.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverManifest {
    pub id: DriverId,
    pub name: String,
    pub version: String,
    pub driver_type: DriverType,
    pub description: String,
    pub capabilities: Vec<String>,
    /// Driver 支持发送的 trace 事件类型列表（可选）
    /// 例如: ["thinking", "tool_call", "tool_result", "text", "compilation", "error"]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_event_types: Option<Vec<String>>,
}

/// The core trait every software driver must implement.
///
/// A driver exposes the software it controls as a set of `Tool` instances,
/// allowing agents to interact with any software through the unified tool API.
///
/// # Lifecycle
///
/// 1. Register with `DriverRegistry`
/// 2. `initialize()` — start subprocess, connect to API, etc.
/// 3. `tools()` — expose capabilities as Tools
/// 4. `shutdown()` — graceful cleanup
#[async_trait]
pub trait SoftwareDriver: Send + Sync {
    /// Driver metadata for discovery and management.
    fn manifest(&self) -> &DriverManifest;

    /// Initialize the driver (e.g., start subprocess, connect to API).
    async fn initialize(&mut self) -> MacacaResult<()>;

    /// Expose the tools this driver provides.
    fn tools(&self) -> Vec<Box<dyn Tool>>;

    /// Check if the driver and its target software are healthy.
    async fn health_check(&self) -> MacacaResult<bool>;

    /// Graceful shutdown — close connections, terminate subprocesses.
    async fn shutdown(&mut self) -> MacacaResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn driver_type_serialize_roundtrip() {
        let dt = DriverType::CliSubprocess;
        let json = serde_json::to_string(&dt).unwrap();
        let parsed: DriverType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, DriverType::CliSubprocess);
    }

    #[test]
    fn driver_manifest_serialize() {
        let manifest = DriverManifest {
            id: DriverId::new(),
            name: "shell".into(),
            version: "0.1.0".into(),
            driver_type: DriverType::CliSubprocess,
            description: "Execute shell commands".into(),
            trace_event_types: None,
            capabilities: vec!["execute_command".into()],
        };
        let json = serde_json::to_string(&manifest).unwrap();
        assert!(json.contains("shell"));
        assert!(json.contains("CliSubprocess"));
    }
}
