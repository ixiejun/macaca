//! Contract tests for audit replay and GenUI surface storage via WASM host imports.

use std::sync::Arc;

use macaca_proto::{
    ApplicationHostCommand, ApplicationHostCommandStatus, ApplicationImport, TraceContext,
    UiComponent, UiComponentKind, UiComponentTree, UiIntent, UiRenderSurface,
};
use serde_json::json;

use super::super::{WasmHostImportBridge, WasmHostImportBridgeConfig};
use crate::{ApplicationGenUiSurfaceStore, ServiceRuntime, ServiceRuntimeConfig};

use super::support::{host_import_command, register_mock_service};

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
