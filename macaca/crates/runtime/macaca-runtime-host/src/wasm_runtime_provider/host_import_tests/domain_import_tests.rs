//! Contract tests for task, agent-delegate, and trace-only WASM host imports.

use std::sync::Arc;

use macaca_proto::{
    ApplicationHostCommand, ApplicationHostCommandStatus, ApplicationId, ApplicationImport,
    TodoStatus, TraceContext, APPLICATION_AGENT_DELEGATE_COMMAND, APPLICATION_SERVICE_ID,
};
use macaca_task::TodoStore;
use serde_json::json;

use super::super::{WasmHostImportBridge, WasmHostImportBridgeConfig};
use crate::{ServiceRuntime, ServiceRuntimeConfig};

use super::support::{register_mock_service, register_task_service, test_store};

#[tokio::test]
async fn wasm_host_import_task_create_goal_routes_to_task_service_boundary() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let task_service_id = register_mock_service(&runtime, "service.task").await;
    let bridge =
        WasmHostImportBridge::new(Arc::clone(&runtime), WasmHostImportBridgeConfig::default());
    let mut command = ApplicationHostCommand::with_trace(
        ApplicationImport::TaskCreateGoal,
        json!({"description": "Plan BTC analysis"}),
        TraceContext::new("trace-wasm-task-create-goal"),
    );
    command.metadata.insert("app.id".into(), "app-wasm".into());
    command
        .metadata
        .insert("session.id".into(), "session-wasm-task".into());
    command
        .metadata
        .insert("capability".into(), "task.manage".into());

    let result = bridge
        .dispatch(command, TraceContext::new("trace-wasm-task-create-goal"))
        .await;

    assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));
    assert_eq!(result.output["description"], json!("Plan BTC analysis"));
    assert_eq!(result.output["app_id"], json!("app-wasm"));
    assert_eq!(result.output["session_id"], json!("session-wasm-task"));
    assert_eq!(
        result.output["trace"]["trace_id"],
        json!("trace-wasm-task-create-goal")
    );
    assert_eq!(
        result.metadata.get("service_id").map(String::as_str),
        Some(task_service_id.as_str())
    );
    assert_eq!(
        result.metadata.get("service.operation").map(String::as_str),
        Some("task.create_goal")
    );
}

#[tokio::test]
async fn wasm_host_import_agent_delegate_routes_to_application_service_boundary() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let task_store = Arc::new(TodoStore::new(test_store(
        "wasm-agent-delegate-task-service",
    )));
    register_task_service(&runtime, Arc::clone(&task_store)).await;
    let application_service_id = register_mock_service(&runtime, APPLICATION_SERVICE_ID).await;
    let bridge =
        WasmHostImportBridge::new(Arc::clone(&runtime), WasmHostImportBridgeConfig::default());
    let app_id = ApplicationId::new();
    let mut command = ApplicationHostCommand::with_trace(
        ApplicationImport::AgentDelegate,
        json!({
            "target_agent": "analyst",
            "prompt": "Analyze BTC buy and sell points",
            "context": {"symbol": "BTC"}
        }),
        TraceContext::new("trace-wasm-agent-delegate"),
    );
    command.metadata.insert("app.id".into(), app_id.to_string());
    command
        .metadata
        .insert("session.id".into(), "session-wasm-agent".into());
    command
        .metadata
        .insert("capability".into(), "agent.delegate".into());

    let result = bridge
        .dispatch(command, TraceContext::new("trace-wasm-agent-delegate"))
        .await;

    assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));
    assert_eq!(result.output["target_agent"], json!("analyst"));
    assert_eq!(
        result.output["scope"]["application_id"],
        json!(app_id.to_string())
    );
    assert_eq!(
        result.output["scope"]["session_id"],
        json!("session-wasm-agent")
    );
    assert_eq!(
        result.output["trace"]["trace_id"],
        json!("trace-wasm-agent-delegate")
    );
    assert_eq!(
        result.metadata.get("service_id").map(String::as_str),
        Some(application_service_id.as_str())
    );
    assert_eq!(
        result.metadata.get("service.operation").map(String::as_str),
        Some(APPLICATION_AGENT_DELEGATE_COMMAND)
    );
    let tasks = task_store
        .list_all_todos_for_session(&app_id, "session-wasm-agent")
        .await;
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].assigned_agent, "analyst");
    assert_eq!(tasks[0].status, TodoStatus::Completed);
}

#[tokio::test]
async fn wasm_host_import_trace_emit_records_process_step_without_service_route() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let bridge =
        WasmHostImportBridge::new(Arc::clone(&runtime), WasmHostImportBridgeConfig::default());
    let command = ApplicationHostCommand::with_trace(
        ApplicationImport::TraceEmit,
        json!({"event": "analysis_start", "symbol": "BTC"}),
        TraceContext::new("trace-wasm-process-step"),
    );

    let result = bridge
        .dispatch(command, TraceContext::new("trace-wasm-process-step"))
        .await;

    assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));
    assert_eq!(result.output["emitted"], json!(true));
    assert_eq!(
        result.metadata.get("reason_code").map(String::as_str),
        Some("trace_emit_recorded")
    );
}

#[tokio::test]
async fn wasm_host_import_task_query_requires_session_scope() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let _ = register_mock_service(&runtime, "service.task").await;
    let bridge =
        WasmHostImportBridge::new(Arc::clone(&runtime), WasmHostImportBridgeConfig::default());
    let mut command = ApplicationHostCommand::with_trace(
        ApplicationImport::TaskQuery,
        json!({}),
        TraceContext::new("trace-wasm-task-query-missing-session"),
    );
    command.metadata.insert("app.id".into(), "app-wasm".into());
    command
        .metadata
        .insert("capability".into(), "task.manage".into());

    let result = bridge
        .dispatch(
            command,
            TraceContext::new("trace-wasm-task-query-missing-session"),
        )
        .await;

    assert!(matches!(
        result.status,
        ApplicationHostCommandStatus::DisabledByPolicy { .. }
    ));
    assert_eq!(
        result.metadata.get("reason_code").map(String::as_str),
        Some("scope_missing")
    );
}
