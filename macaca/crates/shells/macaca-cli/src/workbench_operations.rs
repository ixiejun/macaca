//! CLI adapters for Interactive Agent Workbench diagnostics.
//!
//! The CLI remains a terminal adapter. With `--app-id` it reads the already
//! running Web API facade; without it, it uses SDK Null Object focused clients
//! to prove structured unavailable behavior without constructing providers.

use macaca_proto::{
    ApplicationId, MacacaError, MacacaResult, TraceContext, WorkbenchCommand, WorkbenchCommandScope,
};
use macaca_sdk::{
    SystemWorkbenchClient, UnavailableWorkbenchServiceClient, WorkbenchClientCatalog,
};
use serde_json::Value;
use tracing::info;

#[derive(Debug, Clone, Default)]
pub struct WorkbenchCliRuntimeTarget {
    pub app_id: Option<String>,
    pub api_base: Option<String>,
}

impl WorkbenchCliRuntimeTarget {
    fn live_client(&self) -> MacacaResult<Option<LiveWorkbenchOperationsClient>> {
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
                "workbench live operations require a valid app id: {error}"
            ))
        })?;
        let api_base = self
            .api_base
            .clone()
            .or_else(|| std::env::var("MACACA_API_BASE").ok())
            .unwrap_or_else(|| "http://127.0.0.1:3001".into());
        Ok(Some(LiveWorkbenchOperationsClient::new(
            api_base,
            app_id.into(),
        )))
    }
}

/// Print a bounded workbench diagnostics snapshot through shell-safe adapters.
pub async fn execute_workbench_operations_snapshot(
    target: WorkbenchCliRuntimeTarget,
) -> MacacaResult<()> {
    if let Some(client) = target.live_client()? {
        let response = client.get().await?;
        info!(
            app_id = %client.app_id,
            "CLI emitted live workbench operations snapshot from public Web API facade"
        );
        return print_json(response);
    }

    let app_id = ApplicationId::from_name("cli-workbench-operations");
    let scope = WorkbenchCommandScope::new(app_id, "cli-workbench-operations")?;
    let trace = TraceContext::new("cli-workbench-operations-snapshot");
    let catalog = WorkbenchClientCatalog::unavailable();
    let mut services = Vec::new();
    for client in unavailable_clients(&catalog) {
        let descriptor = client.descriptor();
        let command = WorkbenchCommand::new(
            scope.clone(),
            trace.clone(),
            "service.snapshot",
            Value::Null,
        )?;
        let result = client.call("service.snapshot", command).await?;
        services.push(serde_json::json!({
            "service_id": descriptor.base.id.to_string(),
            "commands": descriptor.commands,
            "status": result.status,
            "trace_id": result.trace.trace_id,
        }));
    }

    info!(
        trace_id = %trace.trace_id,
        services = services.len(),
        "CLI emitted SDK Null Object workbench operations snapshot"
    );
    print_json(serde_json::json!({
        "trace_id": trace.trace_id,
        "services": services,
    }))
}

fn unavailable_clients<'a>(
    catalog: &'a WorkbenchClientCatalog<UnavailableWorkbenchServiceClient>,
) -> Vec<&'a UnavailableWorkbenchServiceClient> {
    vec![
        &catalog.interaction,
        &catalog.app_protocol,
        &catalog.file,
        &catalog.process,
        &catalog.sandbox,
        &catalog.approval,
        &catalog.hook,
        &catalog.config,
        &catalog.plugin_marketplace,
        &catalog.code_intelligence,
        &catalog.git,
        &catalog.review,
        &catalog.diagnostics,
        &catalog.realtime,
        &catalog.remote_environment,
    ]
}

#[derive(Debug, Clone)]
struct LiveWorkbenchOperationsClient {
    api_base: String,
    app_id: String,
    client: reqwest::Client,
}

impl LiveWorkbenchOperationsClient {
    fn new(api_base: String, app_id: String) -> Self {
        Self {
            api_base: api_base.trim_end_matches('/').to_string(),
            app_id,
            client: reqwest::Client::new(),
        }
    }

    async fn get(&self) -> MacacaResult<Value> {
        let response = self
            .client
            .get(format!(
                "{}/api/apps/{}/workbench/operations",
                self.api_base, self.app_id
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
                "workbench operations request failed with status {status}: {payload}"
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
    fn workbench_operations_cli_remains_a_thin_adapter() {
        let source = include_str!("workbench_operations.rs");
        assert!(source.contains("WorkbenchClientCatalog::unavailable"));
        assert!(source.contains("/workbench/operations"));
        assert!(!source.contains(&format!("{}{}", "Local", "FileProvider")));
        assert!(!source.contains(&format!("{}{}", "Local", "ProcessProvider")));
        assert!(!source.contains(&format!("{}{}", "Policy", "Denied")));
    }
}
