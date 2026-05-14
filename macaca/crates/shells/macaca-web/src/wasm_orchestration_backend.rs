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
    ApplicationAgentDelegateCommand, ApplicationAgentDelegateResult, ServiceError, ServiceResult,
    TaskContext,
};
use macaca_runtime_host::ApplicationOrchestrationBackend;
use tokio::sync::RwLock;
use tokio::time::{timeout, Duration};

const DEFAULT_AGENT_DELEGATE_WAIT_MS: u64 = 30_000;
const AGENT_DELEGATE_POLL_MS: u64 = 100;

/// Web-owned adapter from Application Service delegation commands to the
/// app-scoped executor registry.
pub(crate) struct WebApplicationOrchestrationBackend {
    executor_registry: Arc<RwLock<Option<Arc<ApplicationExecutorRegistry>>>>,
}

impl WebApplicationOrchestrationBackend {
    /// Create the adapter with the shared application executor registry.
    pub(crate) fn new(
        executor_registry: Arc<RwLock<Option<Arc<ApplicationExecutorRegistry>>>>,
    ) -> Self {
        Self { executor_registry }
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
        let executor = registry.get(&app_id).await.ok_or_else(|| {
            ServiceError::ServiceUnavailable(format!(
                "application executor is not configured for application {app_id}"
            ))
        })?;
        let from_agent = command
            .scope
            .agent_name
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("wasm-guest");
        let priority = command
            .metadata
            .get("priority")
            .and_then(|value| value.parse::<u8>().ok())
            .unwrap_or(5);
        let parallel = command
            .metadata
            .get("parallel")
            .map(|value| value == "true")
            .unwrap_or(false);
        let context = TaskContext {
            session_id: Some(session_id.clone()),
            artifacts: Vec::new(),
            env: std::collections::HashMap::new(),
        };
        let delegated_prompt = delegate_prompt_with_context(&command.prompt, &command.context);
        let task_id = executor
            .delegate_task(
                from_agent,
                &command.target_agent,
                delegated_prompt,
                priority,
                parallel,
                Some(context),
            )
            .await
            .map_err(ServiceError::AdapterFailure)?;

        // WASM guests execute host commands as a deterministic chain.  Returning
        // only "queued" would make `${host.results.N.output}` unusable for a
        // following aggregation step, so the Web composition adapter waits for a
        // bounded result while still preserving a structured queued fallback.
        let wait_ms = command
            .metadata
            .get("wait_timeout_ms")
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(DEFAULT_AGENT_DELEGATE_WAIT_MS);
        let wait_result = timeout(Duration::from_millis(wait_ms), async {
            let mut poll = tokio::time::interval(Duration::from_millis(AGENT_DELEGATE_POLL_MS));
            loop {
                poll.tick().await;
                if let Some(result) = executor.get_task_result(task_id).await {
                    return result;
                }
            }
        })
        .await;

        if let Ok(result) = wait_result {
            return Ok(ApplicationAgentDelegateResult {
                application_id: app_id,
                session_id,
                target_agent: command.target_agent,
                task_id: Some(task_id.0.to_string()),
                success: result.success,
                output: serde_json::json!({
                    "status": if result.success { "completed" } else { "failed" },
                    "task_id": task_id.0.to_string(),
                    "output": result.output,
                    "error": result.error,
                    "artifacts": result.artifacts
                }),
                status: if result.success {
                    "completed".into()
                } else {
                    "failed".into()
                },
                metadata: std::collections::BTreeMap::from([(
                    "reason_code".into(),
                    "delegate_completed".into(),
                )]),
            });
        }

        Ok(ApplicationAgentDelegateResult {
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
        })
    }
}

/// Render the provider-neutral delegate command into the concrete worker
/// prompt used by the Web executor.  The Application Service contract carries
/// `context` as structured JSON; workers currently receive a prompt string, so
/// this adapter appends the bounded evidence verbatim instead of relying on
/// hidden executor state.  That keeps WASM app orchestration generic while
/// making every child-agent decision reproducible from the persisted task text.
fn delegate_prompt_with_context(prompt: &str, context: &serde_json::Value) -> String {
    if context.is_null() {
        return prompt.to_string();
    }
    let rendered_context =
        serde_json::to_string_pretty(context).unwrap_or_else(|_| context.to_string());
    format!(
        "{prompt}\n\nStructured evidence context for this delegated task:\n```json\n{rendered_context}\n```"
    )
}
