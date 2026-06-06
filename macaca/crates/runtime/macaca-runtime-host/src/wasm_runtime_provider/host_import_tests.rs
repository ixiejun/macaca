use std::sync::Arc;

use macaca_kernel::MockSystemService;
use macaca_persist::RedbStore;
use macaca_proto::{
    ApplicationHostCommand, ApplicationHostCommandStatus, ApplicationId, ApplicationImport,
    KernelServiceId, ServiceCommandName, ServiceDescriptor, ServiceHealth, ServiceLifecycleState,
    ServiceType, TodoStatus, TraceContext, TraceSchemaRef, UiComponent, UiComponentKind,
    UiComponentTree, UiIntent, UiRenderSurface, WasmExecutionProfile, WasmRuntimeArtifactRef,
    WasmRuntimeSessionRequest, APPLICATION_AGENT_DELEGATE_COMMAND, APPLICATION_SERVICE_ID,
    MCP_SERVICE_ID, MCP_TOOL_INVOKE_COMMAND,
};
use macaca_task::TodoStore;
use serde_json::json;
use tempfile::tempdir;

use super::{
    DefaultInProcessWasmRuntimeProvider, WasmApplicationRuntimeProvider, WasmHostImportBridge,
    WasmHostImportBridgeConfig,
};
use crate::{
    task_service_provider::TaskSystemServiceProvider, ApplicationGenUiSurfaceStore,
    ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig, StaticServiceProviderFactory,
};

#[tokio::test]
async fn wasm_host_import_service_call_routes_through_service_runtime() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let service_id = register_mock_service(&runtime, "wasm.host.service.allowed").await;
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let provider = DefaultInProcessWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request("trace-host-import-provider"))
        .await
        .unwrap();
    let command = host_import_command(
        "trace-host-import-command",
        &service_id,
        "invoke",
        json!({"input": true}),
        "service.call",
    );

    let result = session.dispatch(command).await.unwrap();

    assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));
    assert_eq!(result.output["input"], json!(true));
    assert_eq!(
        result.metadata.get("reason_code").map(String::as_str),
        Some("import_completed")
    );
    assert_eq!(
        result.metadata.get("service_id").map(String::as_str),
        Some(service_id.as_str())
    );
}

#[tokio::test]
async fn wasm_host_import_mcp_tool_invoke_uses_generic_service_call_path() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let service_id = register_mock_service(&runtime, MCP_SERVICE_ID).await;
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let provider = DefaultInProcessWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request("trace-host-import-mcp-provider"))
        .await
        .unwrap();
    let command = host_import_command(
        "trace-host-import-mcp-command",
        &service_id,
        MCP_TOOL_INVOKE_COMMAND,
        json!({
            "server_id": "server-a",
            "backend_tool_name": "lookup",
            "visible_tool_name": "mcp_lookup"
        }),
        "service.call",
    );

    let result = session.dispatch(command).await.unwrap();

    assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));
    assert_eq!(result.output["server_id"], json!("server-a"));
    assert_eq!(
        result.metadata.get("service_id").map(String::as_str),
        Some(MCP_SERVICE_ID)
    );
    assert_eq!(
        result.metadata.get("service.operation").map(String::as_str),
        Some(MCP_TOOL_INVOKE_COMMAND)
    );
}

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

#[tokio::test]
async fn wasm_host_import_bridge_replays_service_call_audit_chain() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let service_id = register_mock_service(&runtime, "wasm.host.service.audit.replay").await;
    let bridge =
        WasmHostImportBridge::new(Arc::clone(&runtime), WasmHostImportBridgeConfig::default());
    let mut command = host_import_command(
        "trace-host-import-audit",
        &service_id,
        "invoke",
        json!({"input": true}),
        "service.call",
    );
    command
        .metadata
        .insert("session.id".into(), "session-host-import-audit".into());

    let result = bridge
        .dispatch(command, TraceContext::new("trace-host-import-audit"))
        .await;
    assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));

    let replay = bridge
        .replay_service_call_audit_by_trace_id("trace-host-import-audit")
        .unwrap();
    assert!(replay
        .iter()
        .any(|event| event.stage == "service_call_requested"));
    assert!(replay
        .iter()
        .any(|event| event.stage == "service_call_succeeded"));
    let session_replay = bridge
        .replay_service_call_audit_by_session_id("session-host-import-audit")
        .unwrap();
    assert!(!session_replay.is_empty());
}

#[tokio::test]
async fn wasm_host_import_ui_render_stores_declared_genui_surface() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let store = ApplicationGenUiSurfaceStore::default();
    let bridge =
        WasmHostImportBridge::new(Arc::clone(&runtime), WasmHostImportBridgeConfig::default())
            .with_genui_surface_store(store.clone());
    let trace = TraceContext::new("trace-host-import-ui-render");
    let intent = UiIntent {
        app_id: "app-ui-render".into(),
        session_id: "session-ui-render".into(),
        surface_id: UiRenderSurface::new("main"),
        tree: UiComponentTree {
            root: UiComponent::new("root", UiComponentKind::Card, json!({"title": "Signal"})),
            trace_markers: Vec::new(),
            metadata: Default::default(),
        },
        permission_prompts: Vec::new(),
        approval_prompts: Vec::new(),
        trace: Some(trace.clone()),
        metadata: Default::default(),
    };
    let command = ApplicationHostCommand::with_trace(
        ApplicationImport::UiRender,
        serde_json::to_value(&intent).unwrap(),
        trace.clone(),
    );

    let result = bridge.dispatch(command, trace).await;
    let stored = store
        .get("app-ui-render", "session-ui-render", Some("main"))
        .await
        .unwrap();

    assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));
    assert_eq!(
        result.metadata.get("reason_code").map(String::as_str),
        Some("ui_render_stored")
    );
    assert_eq!(stored.unwrap().tree.root.id, "root");
}

#[tokio::test]
async fn wasm_host_import_missing_trace_is_denied_before_service_runtime() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let service_id = register_mock_service(&runtime, "wasm.host.service.trace").await;
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let provider = DefaultInProcessWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request("trace-host-import-provider"))
        .await
        .unwrap();
    let mut command = ApplicationHostCommand::without_trace(
        ApplicationImport::ServiceCall,
        json!({"input": true}),
    );
    command
        .metadata
        .insert("service.id".into(), service_id.to_string());
    command
        .metadata
        .insert("service.operation".into(), "invoke".into());
    command
        .metadata
        .insert("capability".into(), "service.call".into());

    let error = session.dispatch(command).await.unwrap_err();

    assert_eq!(
        error,
        macaca_proto::ApplicationAbiError::MissingTraceContext
    );
}

#[tokio::test]
async fn wasm_host_import_missing_capability_is_denied() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let service_id = register_mock_service(&runtime, "wasm.host.service.capability").await;
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let provider = DefaultInProcessWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request("trace-host-import-provider"))
        .await
        .unwrap();
    let command = host_import_command(
        "trace-host-import-capability",
        &service_id,
        "invoke",
        json!({"input": true}),
        "",
    );

    let result = session.dispatch(command).await.unwrap();

    assert!(matches!(
        result.status,
        ApplicationHostCommandStatus::DisabledByPolicy { .. }
    ));
    assert_eq!(
        result.metadata.get("reason_code").map(String::as_str),
        Some("capability_missing")
    );
}

#[tokio::test]
async fn wasm_host_import_oversized_payload_is_denied_before_service_runtime() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let service_id = register_mock_service(&runtime, "wasm.host.service.payload").await;
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig {
            max_payload_bytes: 8,
            ..WasmHostImportBridgeConfig::default()
        },
    ));
    let provider = DefaultInProcessWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request("trace-host-import-provider"))
        .await
        .unwrap();
    let command = host_import_command(
        "trace-host-import-payload",
        &service_id,
        "invoke",
        json!({"raw_payload": "secret should stay out"}),
        "service.call",
    );

    let result = session.dispatch(command).await.unwrap();
    let encoded = serde_json::to_string(&result).unwrap().to_lowercase();

    assert!(matches!(
        result.status,
        ApplicationHostCommandStatus::DisabledByPolicy { .. }
    ));
    assert_eq!(
        result.metadata.get("reason_code").map(String::as_str),
        Some("payload_too_large")
    );
    assert!(!encoded.contains("secret should stay out"));
}

#[tokio::test]
async fn wasm_host_import_unknown_service_is_structured_unavailable() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let provider = DefaultInProcessWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request("trace-host-import-provider"))
        .await
        .unwrap();
    let service_id = KernelServiceId::new("wasm.host.service.missing");
    let command = host_import_command(
        "trace-host-import-missing",
        &service_id,
        "invoke",
        json!({"input": true}),
        "service.call",
    );

    let result = session.dispatch(command).await.unwrap();

    assert!(matches!(
        result.status,
        ApplicationHostCommandStatus::Unavailable { .. }
    ));
    assert_eq!(
        result.metadata.get("reason_code").map(String::as_str),
        Some("service_unavailable")
    );
}

#[tokio::test]
async fn wasm_host_import_service_failure_is_structured_unavailable_not_policy_denied() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let service_id =
        register_mock_service_with_failure(&runtime, "wasm.host.service.failing", true).await;
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let provider = DefaultInProcessWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request("trace-host-import-provider"))
        .await
        .unwrap();
    let command = host_import_command(
        "trace-host-import-service-failure",
        &service_id,
        "invoke",
        json!({"input": true}),
        "service.call",
    );

    let result = session.dispatch(command).await.unwrap();

    assert!(matches!(
        result.status,
        ApplicationHostCommandStatus::Unavailable { .. }
    ));
    assert_eq!(
        result.metadata.get("reason_code").map(String::as_str),
        Some("service_failed")
    );
}

#[tokio::test]
async fn wasm_host_import_applies_app_scoped_policy_override_and_denies_service() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let service_id = register_mock_service(&runtime, "wasm.host.service.policy.denied").await;
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let provider = DefaultInProcessWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request("trace-host-import-provider"))
        .await
        .unwrap();
    let mut command = host_import_command(
        "trace-host-import-policy-override",
        &service_id,
        "invoke",
        json!({"input": true}),
        "service.call",
    );
    command
        .metadata
        .insert("app.id".into(), "app-policy-a".into());
    command
        .metadata
        .insert("policy.deny_services".into(), service_id.to_string());

    let result = session.dispatch(command).await.unwrap();

    assert!(matches!(
        result.status,
        ApplicationHostCommandStatus::DisabledByPolicy { .. }
    ));
    assert_eq!(
        result.metadata.get("reason_code").map(String::as_str),
        Some("policy_denied")
    );
}

#[tokio::test]
async fn wasm_host_import_sanitizes_service_result_metadata() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let service_id = register_mock_service(&runtime, "wasm.host.service.sanitize").await;
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let provider = DefaultInProcessWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request("trace-host-import-provider"))
        .await
        .unwrap();
    let command = host_import_command(
        "trace-host-import-sanitize",
        &service_id,
        "invoke",
        json!({"raw_prompt": "secret prompt must not escape"}),
        "service.call",
    );

    let result = session.dispatch(command).await.unwrap();
    let encoded = serde_json::to_string(&result).unwrap().to_lowercase();

    assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));
    assert!(!encoded.contains("secret prompt must not escape"));
    assert!(!encoded.contains("raw_prompt"));
}

async fn register_mock_service(runtime: &ServiceRuntime, service_id: &str) -> KernelServiceId {
    register_mock_service_with_failure(runtime, service_id, false).await
}

async fn register_task_service(runtime: &ServiceRuntime, store: Arc<TodoStore>) -> KernelServiceId {
    let service: Arc<dyn macaca_kernel::SystemService> =
        Arc::new(TaskSystemServiceProvider::local(store));
    let descriptor = service.descriptor();
    let service_id = runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(
                descriptor.clone(),
                service,
            )),
            Default::default(),
        )
        .await
        .unwrap();
    runtime
        .start(&service_id, TraceContext::new("trace-task-service-start"))
        .await
        .unwrap();
    service_id
}

async fn register_mock_service_with_failure(
    runtime: &ServiceRuntime,
    service_id: &str,
    fail_calls: bool,
) -> KernelServiceId {
    let descriptor = ServiceDescriptor::new(
        KernelServiceId::new(service_id),
        ServiceType::new("test.service"),
        TraceSchemaRef::new("trace.test.service.v1"),
    );
    let service: Arc<dyn macaca_kernel::SystemService> = if fail_calls {
        Arc::new(MockSystemService::failing(descriptor.clone()))
    } else {
        Arc::new(MockSystemService::new(descriptor.clone()))
    };
    let service_id = runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(
                descriptor.clone(),
                service,
            )),
            Default::default(),
        )
        .await
        .unwrap();
    runtime
        .start(&service_id, TraceContext::new("trace-host-import-start"))
        .await
        .unwrap();
    let snapshot = runtime.snapshot().unwrap();
    assert_eq!(
        snapshot.services[0].lifecycle_state,
        ServiceLifecycleState::Running
    );
    assert_eq!(snapshot.services[0].health, ServiceHealth::Healthy);
    service_id
}

fn traced_request(trace_id: &str) -> WasmRuntimeSessionRequest {
    let artifact_path = write_fixture_wasm("host-import", minimal_start_module());
    WasmRuntimeSessionRequest::new(
        TraceContext::new(trace_id),
        "fixture.application",
        "main",
        WasmRuntimeArtifactRef::new(format!("file://{}", artifact_path.display())),
        WasmExecutionProfile::default_wasm_component(),
    )
    .unwrap()
}

fn test_store(name: &str) -> Arc<dyn macaca_persist::PersistBackend> {
    let dir = tempdir().expect("tempdir should be available for host import task test");
    let path = dir.path().join(format!("{name}.redb"));
    // Keep the temporary directory alive for the test process.  The redb backend
    // owns an open database handle, and dropping the directory immediately can
    // remove the file before async assertions finish on some platforms.
    let _dir = Box::leak(Box::new(dir));
    Arc::new(RedbStore::open(path).expect("redb store should open for host import task test"))
}

fn host_import_command(
    trace_id: &str,
    service_id: &KernelServiceId,
    operation: &str,
    payload: serde_json::Value,
    capability: &str,
) -> ApplicationHostCommand {
    let mut command = ApplicationHostCommand::with_trace(
        ApplicationImport::ServiceCall,
        payload,
        TraceContext::new(trace_id),
    );
    command
        .metadata
        .insert("service.id".into(), service_id.to_string());
    command.metadata.insert(
        "service.operation".into(),
        ServiceCommandName::new(operation).to_string(),
    );
    if !capability.is_empty() {
        command
            .metadata
            .insert("capability".into(), capability.to_string());
    }
    command
}

fn write_fixture_wasm(name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let directory = tempfile::Builder::new()
        .prefix("macaca-wasm-host-import-")
        .tempdir()
        .unwrap()
        .keep();
    let path = directory.join(format!("{name}.wasm"));
    std::fs::write(&path, bytes).unwrap();
    path
}

fn minimal_start_module() -> &'static [u8] {
    &[
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x04, 0x01, 0x60, 0x00, 0x00, 0x03,
        0x02, 0x01, 0x00, 0x07, 0x0d, 0x01, 0x09, b'a', b'p', b'p', b':', b's', b't', b'a', b'r',
        b't', 0x00, 0x00, 0x0a, 0x04, 0x01, 0x02, 0x00, 0x0b,
    ]
}
