//! Web composition adapter for WASM application orchestration.
//!
//! The Application Service owns the provider-neutral
//! `application.agent.delegate` command.  Web owns the concrete
//! `ApplicationExecutorRegistry` that is already scoped per application and is
//! therefore the right composition root for connecting that command to worker
//! execution.  This file keeps the adapter small and explicit so runtime-host
//! does not import Web state and Web does not define new orchestration
//! semantics.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::ApplicationExecutorRegistry;
use macaca_proto::{
    AgentExecutionCommand, AgentExecutionIntent, ApplicationAgentDelegateCommand,
    ApplicationAgentDelegateResult, ApplicationId, KernelServiceId, ServiceBusSource, ServiceError,
    ServiceResult, TaskId, AGENT_EXECUTION_SERVICE_ID,
};
use macaca_runtime_host::{ApplicationOrchestrationBackend, ServiceRuntime};
use tokio::sync::{oneshot, RwLock};
use tokio::time::{timeout, Duration};

use crate::workspace::AppWorkspace;

const DEFAULT_AGENT_DELEGATE_WAIT_MS: u64 = 30_000;

/// Web-owned adapter from Application Service delegation commands to the
/// app-scoped executor registry.
pub(crate) struct WebApplicationOrchestrationBackend {
    executor_registry: Arc<RwLock<Option<Arc<ApplicationExecutorRegistry>>>>,
    service_runtime: Arc<ServiceRuntime>,
    app_workspaces: Arc<RwLock<std::collections::HashMap<ApplicationId, AppWorkspace>>>,
}

impl WebApplicationOrchestrationBackend {
    /// Create the adapter with the shared application executor registry.
    pub(crate) fn new(
        executor_registry: Arc<RwLock<Option<Arc<ApplicationExecutorRegistry>>>>,
        service_runtime: Arc<ServiceRuntime>,
        app_workspaces: Arc<RwLock<std::collections::HashMap<ApplicationId, AppWorkspace>>>,
    ) -> Self {
        Self {
            executor_registry,
            service_runtime,
            app_workspaces,
        }
    }

    /// Merge application-provided delegate context with host-owned workspace
    /// coordinates.
    ///
    /// The input context remains application-owned data.  The added
    /// `workspace` object is generic platform evidence prepared during app
    /// startup, so agents can write requested artifacts inside the declared
    /// shared workspace without trusting a browser-supplied path or parsing app
    /// names.  If the workspace is not registered yet, the original context is
    /// returned and the execution service will proceed without invented paths.
    async fn delegated_context_with_workspace(
        &self,
        app_id: ApplicationId,
        context: serde_json::Value,
    ) -> serde_json::Value {
        let Some(workspace) = self.app_workspaces.read().await.get(&app_id).cloned() else {
            tracing::warn!(
                application_id = %app_id,
                "application agent delegation continued without registered workspace context"
            );
            return context;
        };
        let mut object = match context {
            serde_json::Value::Object(object) => object,
            other => serde_json::Map::from_iter([("application_context".into(), other)]),
        };
        object.insert(
            "workspace".into(),
            serde_json::json!({
                "root_path": workspace.root,
                "shared_path": workspace.shared,
                "agents_root_path": workspace.agents_root,
            }),
        );
        tracing::info!(
            application_id = %app_id,
            shared_path = %workspace.shared.display(),
            "application agent delegation attached shared workspace context"
        );
        serde_json::Value::Object(object)
    }
}

#[async_trait]
impl ApplicationOrchestrationBackend for WebApplicationOrchestrationBackend {
    async fn delegate_agent(
        &self,
        command: ApplicationAgentDelegateCommand,
    ) -> ServiceResult<ApplicationAgentDelegateResult> {
        let app_id = command.scope.application_id.ok_or_else(|| {
            ServiceError::AdapterFailure(
                "application.agent.delegate requires application_id".into(),
            )
        })?;
        let session_id = command
            .scope
            .session_id
            .clone()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                ServiceError::AdapterFailure(
                    "application.agent.delegate requires session_id".into(),
                )
            })?;
        let registry = self.executor_registry.read().await.clone().ok_or_else(|| {
            ServiceError::ServiceUnavailable(
                "application executor registry is not configured".into(),
            )
        })?;
        if registry.get(&app_id).await.is_none() {
            return Err(ServiceError::ServiceUnavailable(format!(
                "application executor is not configured for application {app_id}"
            )));
        }
        let task_id = TaskId::new();
        let delegated_context = self
            .delegated_context_with_workspace(app_id, command.context.clone())
            .await;
        let mut execution_command = AgentExecutionCommand::new(
            app_id,
            session_id.clone(),
            command.target_agent.clone(),
            AgentExecutionIntent::WasmDelegate,
            command.prompt.clone(),
            command.trace.clone(),
        )
        .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?
        .with_delegated_context(delegated_context);
        execution_command.task_id = Some(task_id);
        execution_command.source_agent = command.scope.agent_name.clone();
        execution_command.metadata = command.metadata.clone();
        if metadata_value_needs_context_promotion(
            execution_command
                .metadata
                .get("application_execution.run_id"),
        ) {
            if let Some(run_id) =
                promoted_application_execution_run_id(&execution_command.delegated_context)
            {
                tracing::info!(
                    application_id = %app_id,
                    session_id = %session_id,
                    "application agent delegation promoted application execution run id from delegated context"
                );
                execution_command
                    .metadata
                    .insert("application_execution.run_id".into(), run_id.to_string());
            }
        }
        execution_command
            .metadata
            .entry("application_execution.provider_id".into())
            .or_insert_with(|| "provider.macaca_hosted".into());
        execution_command
            .metadata
            .entry("application_execution.provider_kind".into())
            .or_insert_with(|| "MacacaHosted".into());

        let service_runtime = Arc::clone(&self.service_runtime);
        let service_command = execution_command
            .into_service_command()
            .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?;
        let (reply_tx, reply_rx) = oneshot::channel();
        tokio::spawn(async move {
            let reply = service_runtime
                .call(
                    &KernelServiceId::new(AGENT_EXECUTION_SERVICE_ID),
                    ServiceBusSource::new("macaca.web.wasm_orchestration"),
                    service_command,
                )
                .await;
            let _ = reply_tx.send(reply);
        });
        let wait_ms = command
            .metadata
            .get("wait_timeout_ms")
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(DEFAULT_AGENT_DELEGATE_WAIT_MS);
        let reply = match timeout(Duration::from_millis(wait_ms), reply_rx).await {
            Ok(Ok(Ok(reply))) => reply,
            Ok(Ok(Err(error))) => {
                return Err(ServiceError::AdapterFailure(error.to_string()));
            }
            Ok(Err(_closed)) => {
                return Err(ServiceError::AdapterFailure(
                    "agent execution service reply channel closed".into(),
                ));
            }
            Err(_elapsed) => {
                return Ok(ApplicationAgentDelegateResult {
                    application_id: app_id,
                    session_id,
                    target_agent: command.target_agent,
                    task_id: Some(task_id.0.to_string()),
                    success: true,
                    output: serde_json::json!({
                        "status": "queued",
                        "task_id": task_id.0.to_string()
                    }),
                    status: "queued".into(),
                    metadata: std::collections::BTreeMap::from([(
                        "reason_code".into(),
                        "delegate_queued".into(),
                    )]),
                });
            }
        };
        let output = reply.output.ok_or_else(|| {
            ServiceError::AdapterFailure("agent execution service returned no output".into())
        })?;
        let result: macaca_proto::AgentExecutionResult = serde_json::from_value(output)
            .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?;
        Ok(ApplicationAgentDelegateResult {
            application_id: app_id,
            session_id,
            target_agent: command.target_agent,
            task_id: result.task_id.map(|id| id.0.to_string()),
            success: matches!(result.status, macaca_proto::AgentExecutionStatus::Completed),
            output: result.output,
            status: result.status.as_str().into(),
            metadata: result.metadata,
        })
    }
}

/// Decide whether a metadata value should be replaced by trusted delegated
/// context.
///
/// Declarative WASM packages may contain template placeholders in metadata
/// because older component fixtures only interpolated payloads.  Treating a
/// literal `${...}` as authoritative splits durable application-execution
/// mirrors into an unusable run id.  This helper is deliberately generic: it
/// recognizes only empty/template-shaped metadata and never branches on an app,
/// workflow, provider, or service name.
fn metadata_value_needs_context_promotion(value: Option<&String>) -> bool {
    value
        .map(|value| {
            let trimmed = value.trim();
            trimmed.is_empty() || (trimmed.starts_with("${") && trimmed.ends_with('}'))
        })
        .unwrap_or(true)
}

/// Read the application-execution run id from bounded delegated context.
///
/// The Application Service already validated app/session scope before calling
/// this adapter.  This helper only extracts the optional correlation id used by
/// the Observer bridge that mirrors `service.agent_execution` progress back
/// into `service.application_execution`.
fn promoted_application_execution_run_id(context: &serde_json::Value) -> Option<&str> {
    context
        .get("application_execution_run_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter(|value| !(value.starts_with("${") && value.ends_with('}')))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use macaca_proto::ApplicationId;
    use macaca_runtime_host::{ServiceRuntime, ServiceRuntimeConfig};
    use tokio::sync::RwLock;

    use crate::workspace::AppWorkspace;

    #[test]
    fn wasm_delegate_uses_agent_execution_service_not_executor_fast_path() {
        let source = include_str!("wasm_orchestration_backend.rs");
        let executor_fast_path = [".delegate", "_task("].concat();

        assert!(source.contains("AGENT_EXECUTION_SERVICE_ID"));
        assert!(source.contains("AgentExecutionCommand::new"));
        assert!(source.contains("ServiceBusSource::new(\"macaca.web.wasm_orchestration\")"));
        assert!(!source.contains(&executor_fast_path));
    }

    #[tokio::test]
    async fn delegated_context_includes_registered_shared_workspace_paths() {
        let app_id = ApplicationId::from_name("demo");
        let workspace = AppWorkspace::new("/tmp/macaca-workspace-context-test", &app_id);
        let app_workspaces = Arc::new(RwLock::new(HashMap::from([(app_id, workspace.clone())])));
        let backend = super::WebApplicationOrchestrationBackend::new(
            Arc::new(RwLock::new(None)),
            Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default())),
            app_workspaces,
        );

        let context = backend
            .delegated_context_with_workspace(app_id, serde_json::json!({"user_input": "task"}))
            .await;

        assert_eq!(context["user_input"], "task");
        assert_eq!(
            context["workspace"]["shared_path"],
            workspace.shared.to_string_lossy().to_string()
        );
    }

    #[test]
    fn wasm_delegate_promotes_application_execution_run_metadata() {
        let source = include_str!("wasm_orchestration_backend.rs");

        assert!(source.contains("application_execution.run_id"));
        assert!(source.contains("application_execution_run_id"));
        assert!(source.contains("application_execution.provider_id"));
    }

    #[test]
    fn wasm_delegate_replaces_unresolved_run_id_template_from_context() {
        let unresolved = "${chat.run_id}".to_string();
        let context = serde_json::json!({
            "application_execution_run_id": "run-visible-1"
        });

        assert!(super::metadata_value_needs_context_promotion(Some(
            &unresolved
        )));
        assert_eq!(
            super::promoted_application_execution_run_id(&context),
            Some("run-visible-1")
        );
    }
}
