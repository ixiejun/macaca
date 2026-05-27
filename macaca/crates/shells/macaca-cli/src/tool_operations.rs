//! CLI adapters for Tool Capability Plane diagnostics.
//!
//! CLI is a presentation shell. These commands either read the already-running
//! Web API facade for an application-scoped live runtime, or fall back to the
//! SDK Null Object client. The module never constructs providers, evaluates
//! policy, opens artifacts, or reads Driver/Skill/MCP runtime state directly.

use macaca_proto::{MacacaError, MacacaResult, ToolGenericTraceCommand, TraceContext};
use macaca_sdk::{SystemToolClient, UnavailableSystemToolClient};
use serde_json::Value;
use tracing::info;

#[derive(Debug, Clone, Default)]
pub struct ToolCliRuntimeTarget {
    pub app_id: Option<String>,
    pub api_base: Option<String>,
}

impl ToolCliRuntimeTarget {
    fn live_client(&self) -> MacacaResult<Option<LiveToolOperationsClient>> {
        let Some(app_id) = self
            .app_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Ok(None);
        };
        uuid::Uuid::parse_str(app_id).map_err(|error| {
            MacacaError::Config(format!(
                "tool live operations require a valid app id: {error}"
            ))
        })?;
        let api_base = self
            .api_base
            .clone()
            .or_else(|| std::env::var("MACACA_API_BASE").ok())
            .unwrap_or_else(|| "http://127.0.0.1:3001".into());
        Ok(Some(LiveToolOperationsClient::new(api_base, app_id.into())))
    }
}

/// Print a sanitized Tool diagnostics snapshot through a shell-safe boundary.
pub async fn execute_tool_operations_snapshot(target: ToolCliRuntimeTarget) -> MacacaResult<()> {
    if let Some(client) = target.live_client()? {
        let response = client.get("").await?;
        info!(
            app_id = %client.app_id,
            "CLI emitted live Tool operations snapshot from public Web API facade"
        );
        return print_json(response);
    }

    let client = UnavailableSystemToolClient;
    let trace = TraceContext::new("cli-tool-operations-snapshot");
    let command = ToolGenericTraceCommand::new(trace.clone())?;
    info!(
        trace_id = %trace.trace_id,
        command = "tool.operations.snapshot",
        "CLI forwarding Tool operations snapshot through SDK Tool facade"
    );
    let snapshot = client.catalog_snapshot(command.clone()).await?;
    let health = client.provider_health(trace.clone()).await?;
    let audit = client.audit_query(command).await?;
    print_json(serde_json::json!({
        "trace_id": trace.trace_id,
        "snapshot": snapshot,
        "provider_health": health,
        "audit": audit,
    }))
}

#[derive(Debug, Clone)]
struct LiveToolOperationsClient {
    api_base: String,
    app_id: String,
    client: reqwest::Client,
}

impl LiveToolOperationsClient {
    fn new(api_base: String, app_id: String) -> Self {
        Self {
            api_base: api_base.trim_end_matches('/').to_string(),
            app_id,
            client: reqwest::Client::new(),
        }
    }

    async fn get(&self, path: &str) -> MacacaResult<Value> {
        let response = self
            .client
            .get(format!(
                "{}/api/apps/{}/tools/operations{}",
                self.api_base, self.app_id, path
            ))
            .send()
            .await
            .map_err(|error| MacacaError::Config(error.to_string()))?;
        let status = response.status();
        let payload = response
            .json::<Value>()
            .await
            .map_err(|error| MacacaError::Config(error.to_string()))?;
        if !status.is_success() {
            return Err(MacacaError::Config(format!(
                "tool operations request failed with status {status}: {payload}"
            )));
        }
        Ok(payload)
    }
}

fn print_json(value: Value) -> MacacaResult<()> {
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn tool_operations_cli_remains_a_thin_adapter() {
        let source = include_str!("tool_operations.rs");
        assert!(source.contains("UnavailableSystemToolClient"));
        assert!(source.contains("/tools/operations"));
        assert!(!source.contains(&format!("{}{}", "Driver", "Registry")));
        assert!(!source.contains(&format!("{}{}", "SkillRuntime", "Facade")));
        assert!(!source.contains(&format!("{}{}", "McpRuntime", "Facade")));
        assert!(!source.contains(&format!("{}{}", "Policy", "Denied")));
    }
}
