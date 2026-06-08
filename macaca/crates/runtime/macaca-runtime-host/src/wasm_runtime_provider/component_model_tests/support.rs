//! Object Mother + Test Double helpers for component-model WASM contract tests.
//!
//! Builders materialize portable component fixtures, register mock services, and
//! construct traced session requests so each test file asserts one architectural contract.

use std::sync::Arc;

use macaca_kernel::MockSystemService;
use macaca_proto::{
    ApplicationHostCommand, KernelServiceId, ServiceDescriptor, ServiceType, TraceContext,
    TraceSchemaRef, WasmExecutionProfile, WasmRuntimeArtifactRef, WasmRuntimeSessionRequest,
};

use crate::{
    ServiceProviderInstance, ServiceRuntime, StaticServiceProviderFactory,
};

pub(super) async fn register_mock_service(runtime: &ServiceRuntime, service_id: &str) -> KernelServiceId {
    let descriptor = ServiceDescriptor::new(
        KernelServiceId::new(service_id),
        ServiceType::new("test.service"),
        TraceSchemaRef::new("trace.test.service.v1"),
    );
    let service_id = runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(
                descriptor.clone(),
                Arc::new(MockSystemService::new(descriptor)),
            )),
            Default::default(),
        )
        .await
        .unwrap();
    runtime
        .start(
            &service_id,
            TraceContext::new("trace-component-service-start"),
        )
        .await
        .unwrap();
    service_id
}

pub(super) fn traced_request(trace_id: &str, artifact_path: &std::path::Path) -> WasmRuntimeSessionRequest {
    WasmRuntimeSessionRequest::new(
        TraceContext::new(trace_id),
        "fixture.application",
        "main",
        WasmRuntimeArtifactRef::new(format!("file://{}", artifact_path.display())),
        WasmExecutionProfile::default_wasm_component(),
    )
    .unwrap()
}

pub(super) fn write_fixture_component(name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let directory = tempfile::Builder::new()
        .prefix("macaca-wasm-component-provider-")
        .tempdir()
        .unwrap()
        .keep();
    let path = directory.join(format!("{name}.wasm"));
    std::fs::write(&path, bytes).unwrap();
    path
}

pub(super) fn component_fixture_bytes() -> &'static [u8] {
    b"\0asm\x01\0\0\0macaca:component-model:v1\0export=app:start\0wit=macaca:application/runtime@1"
}

pub(super) fn component_fixture_bytes_with_host_command(command: &ApplicationHostCommand) -> Vec<u8> {
    let encoded = serde_json::to_string(command).unwrap();
    format!(
        "\0asm\x01\0\0\0macaca:component-model:v1\0export=app:start\0wit=macaca:application/runtime@1\0host-command={encoded}\0"
    )
    .into_bytes()
}

pub(super) fn component_fixture_bytes_with_duplicate_host_command(
    command: &ApplicationHostCommand,
) -> Vec<u8> {
    let encoded = serde_json::to_string(command).unwrap();
    format!(
        "\0asm\x01\0\0\0macaca:component-model:v1\0export=app:start\0wit=macaca:application/runtime@1\0host-command={encoded}\0.rodata.macaca_component\0host-command={encoded}\0"
    )
    .into_bytes()
}

pub(super) fn component_fixture_bytes_with_host_commands(commands: &[ApplicationHostCommand]) -> Vec<u8> {
    let encoded = commands
        .iter()
        .map(|command| format!("host-command={}", serde_json::to_string(command).unwrap()))
        .collect::<Vec<_>>()
        .join("\0");
    format!(
        "\0asm\x01\0\0\0macaca:component-model:v1\0export=app:start\0wit=macaca:application/runtime@1\0{encoded}\0"
    )
    .into_bytes()
}
