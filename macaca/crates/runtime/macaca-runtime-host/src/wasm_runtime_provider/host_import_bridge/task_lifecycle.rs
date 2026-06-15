//! Task Service lifecycle Adapter for `agent_delegate` imports.
//!
//! Delegated agent execution must leave durable task-board evidence for audit
//! and replay. This module opens, advances, and closes Task Service records
//! using only provider-neutral metadata supplied by trusted host scope.

use macaca_proto::{
    ApplicationHostCommandResult, KernelServiceId, ServiceCommandName, TraceContext,
    WasmHostImportCommand,
};
use serde_json::Value;

use crate::ServiceRouteRequest;

use super::constants::{
    APPLICATION_EXECUTION_GRAPH_OWNER, TASK_CLAIM_OPERATION, TASK_CREATE_ASSIGNMENT_OPERATION,
    TASK_REVIEW_OPERATION, TASK_SERVICE_ID, TASK_START_OPERATION, TASK_SUBMIT_REVIEW_OPERATION,
};
use super::routing_support::{extract_task_id_from_assignment, task_lifecycle_payload};
use super::WasmHostImportBridge;

impl WasmHostImportBridge {
    /// Create and start a Task Service record for an `agent_delegate` import.
    ///
    /// `agent_delegate` is a generic Application ABI operation: a guest asks the
    /// host to run one app-scoped agent. The OS must therefore own the durable
    /// task-board lifecycle for that delegation instead of leaving application
    /// UIs to synthesize state from trace events. This Adapter writes only
    /// provider-neutral task metadata and avoids any application-name or
    /// workflow-specific branching.
    pub(super) async fn open_agent_delegate_task_lifecycle(
        &self,
        command: &WasmHostImportCommand,
        trace: &TraceContext,
    ) -> Result<Option<String>, ApplicationHostCommandResult> {
        let Some(app_id) = command.metadata.get("app.id").cloned() else {
            return Ok(None);
        };
        let Some(session_id) = command
            .metadata
            .get("session.id")
            .cloned()
            .or_else(|| trace.session_id.clone())
        else {
            return Ok(None);
        };
        let target_agent = command
            .payload
            .get("target_agent")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("agent");
        let created_by = command
            .metadata
            .get("agent.name")
            .map(String::as_str)
            .unwrap_or("wasm-guest");
        let priority = command
            .metadata
            .get("priority")
            .and_then(|value| value.parse::<u8>().ok())
            .unwrap_or(5);
        let prompt = command
            .payload
            .get("prompt")
            .and_then(Value::as_str)
            .unwrap_or("Application requested delegated agent execution.");
        let task_id = self
            .route_task_lifecycle_command(
                command,
                TASK_CREATE_ASSIGNMENT_OPERATION,
                serde_json::json!({
                    "app_id": app_id,
                    "session_id": session_id,
                    "agent_name": target_agent,
                    "created_by": created_by,
                    "title": format!("Delegate task to {target_agent}"),
                    "description": prompt,
                    "acceptance_criteria": [
                        "The delegated agent reports a traceable completion result.",
                        "The Task Service records the lifecycle transition for audit and replay."
                    ],
                    "priority": priority,
                    "depends_on": [],
                    "parent_task": null,
                    "graph_owner": APPLICATION_EXECUTION_GRAPH_OWNER,
                    "trace": trace,
                }),
                trace,
            )
            .await?
            .and_then(extract_task_id_from_assignment);
        let Some(task_id) = task_id else {
            return Ok(None);
        };
        self.route_task_lifecycle_command(
            command,
            TASK_CLAIM_OPERATION,
            task_lifecycle_payload(&app_id, &session_id, target_agent, &task_id, trace),
            trace,
        )
        .await?;
        self.route_task_lifecycle_command(
            command,
            TASK_START_OPERATION,
            task_lifecycle_payload(&app_id, &session_id, target_agent, &task_id, trace),
            trace,
        )
        .await?;
        Ok(Some(task_id))
    }

    /// Submit and review the Task Service record for a completed delegation.
    pub(super) async fn close_agent_delegate_task_lifecycle(
        &self,
        command: &WasmHostImportCommand,
        trace: &TraceContext,
        task_id: &str,
        passed: bool,
        summary: &str,
    ) -> Result<(), ApplicationHostCommandResult> {
        let Some(app_id) = command.metadata.get("app.id").cloned() else {
            return Ok(());
        };
        let Some(session_id) = command
            .metadata
            .get("session.id")
            .cloned()
            .or_else(|| trace.session_id.clone())
        else {
            return Ok(());
        };
        let target_agent = command
            .payload
            .get("target_agent")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("agent");
        let mut submit_payload =
            task_lifecycle_payload(&app_id, &session_id, target_agent, task_id, trace);
        submit_payload["summary"] = Value::String(summary.to_string());
        self.route_task_lifecycle_command(
            command,
            TASK_SUBMIT_REVIEW_OPERATION,
            submit_payload,
            trace,
        )
        .await?;
        let review_payload = serde_json::json!({
            "app_id": app_id,
            "session_id": session_id,
            "agent_name": target_agent,
            "task_id": task_id,
            "result": {
                "passed": passed,
                "feedback": summary,
                "verified_criteria": [
                    ["The delegated agent reports a traceable completion result.", passed],
                    ["The Task Service records the lifecycle transition for audit and replay.", true]
                ]
            },
            "trace": trace,
        });
        self.route_task_lifecycle_command(command, TASK_REVIEW_OPERATION, review_payload, trace)
            .await?;
        Ok(())
    }

    /// Route one internal Task Service lifecycle command.
    ///
    /// Internal lifecycle calls reuse the same ServiceRuntime decorators as
    /// guest calls, so policy, audit, trace, and structured unavailable behavior
    /// stay centralized. The source command is passed only for policy scope and
    /// error shaping; no application-specific behavior is inspected here.
    async fn route_task_lifecycle_command(
        &self,
        source: &WasmHostImportCommand,
        operation: &str,
        payload: Value,
        trace: &TraceContext,
    ) -> Result<Option<Value>, ApplicationHostCommandResult> {
        match self
            .router
            .route(ServiceRouteRequest {
                app_id: source.metadata.get("app.id").cloned(),
                tenant_id: source.metadata.get("tenant.id").cloned(),
                session_id: source.metadata.get("session.id").cloned(),
                service_id: KernelServiceId::new(TASK_SERVICE_ID),
                operation: ServiceCommandName::new(operation),
                payload,
                metadata: source.metadata.clone(),
                trace: trace.clone(),
            })
            .await
        {
            Ok(reply) => Ok(Some(reply.output)),
            Err(error) => Err(self.error_result(source, error)),
        }
    }
}
