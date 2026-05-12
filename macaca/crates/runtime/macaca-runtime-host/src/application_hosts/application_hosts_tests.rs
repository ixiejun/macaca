use macaca_proto::{
    ApplicationHostCommand, ApplicationImport, DeveloperId, PackageDescriptor, PackageEntry,
    PackageId, PackageManifest, PackageRuntime, PackageRuntimeKind, PackageType, TraceContext,
};
use serde_json::json;

use super::{
    is_application_runtime_unavailable, ApplicationHostRuntime, UnavailableWasmApplicationHost,
    WasmApplicationHostFactory,
};

fn wasm_package() -> PackageDescriptor {
    let mut descriptor = PackageDescriptor::new(PackageManifest::new(
        PackageId::new("fixture.application.wasm"),
        PackageType::Application,
        "1.0.0",
        DeveloperId::new("developer.fixture"),
        PackageRuntime::new(PackageRuntimeKind::WasmComponent, "0"),
    ));
    descriptor.manifest.entry = Some(PackageEntry {
        kind: "component".into(),
        value: "component.wasm".into(),
    });
    descriptor
        .manifest
        .metadata
        .insert("application.id".into(), "fixture.application.wasm".into());
    descriptor
}

#[tokio::test]
async fn wasm_application_host_returns_runtime_unavailable_with_trace_metadata() {
    let host = WasmApplicationHostFactory.create_for_package(&wasm_package());
    let command = ApplicationHostCommand::with_trace(
        ApplicationImport::ServiceCall,
        json!({"payload": "not_logged"}),
        TraceContext::new("runtime-host-wasm-unavailable"),
    );

    let result = host.dispatch(command).await.unwrap();

    assert!(is_application_runtime_unavailable(&result));
    assert_eq!(
        result.metadata.get("reason_code").map(String::as_str),
        Some("wasm_component_runtime_unavailable")
    );
    assert_eq!(
        result.metadata.get("runtime_kind").map(String::as_str),
        Some("wasm_component")
    );
}

#[tokio::test]
async fn wasm_application_host_rejects_missing_trace() {
    let host = UnavailableWasmApplicationHost::default();
    let command = ApplicationHostCommand::without_trace(ApplicationImport::TraceEmit, json!({}));

    let error = host.dispatch(command).await.unwrap_err();

    assert_eq!(
        error,
        macaca_proto::ApplicationAbiError::MissingTraceContext
    );
}
