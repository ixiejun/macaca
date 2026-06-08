//! Object Mother + Test Double helpers for WASM host-import contract tests.
//!
//! Builders register mock services, construct traced host commands, and materialize
//! minimal WASM fixtures so each test file asserts one architectural contract.

use std::sync::Arc;

use macaca_kernel::MockSystemService;
use macaca_persist::RedbStore;
use macaca_proto::{
    ApplicationHostCommand, ApplicationImport, KernelServiceId, ServiceCommandName,
    ServiceDescriptor, ServiceHealth, ServiceLifecycleState, TraceContext, TraceSchemaRef,
    ServiceType, WasmExecutionProfile, WasmRuntimeArtifactRef, WasmRuntimeSessionRequest,
};
use tempfile::tempdir;

use super::super::{WasmHostImportBridge, WasmHostImportBridgeConfig};
use crate::{
    task_service_provider::TaskSystemServiceProvider, ServiceProviderInstance, ServiceRuntime,
    ServiceRuntimeConfig, StaticServiceProviderFactory,
};
use macaca_task::TodoStore;

pub(super) async fn register_mock_service(runtime: &ServiceRuntime, service_id: &str) -> KernelServiceId {
    register_mock_service_with_failure(runtime, service_id, false).await
}

pub(super) async fn register_task_service(runtime: &ServiceRuntime, store: Arc<TodoStore>) -> KernelServiceId {
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

pub(super) async fn register_mock_service_with_failure(
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

pub(super) fn traced_request(trace_id: &str) -> WasmRuntimeSessionRequest {
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

pub(super) fn test_store(name: &str) -> Arc<dyn macaca_persist::PersistBackend> {
    let dir = tempdir().expect("tempdir should be available for host import task test");
    let path = dir.path().join(format!("{name}.redb"));
    // Keep the temporary directory alive for the test process.  The redb backend
    // owns an open database handle, and dropping the directory immediately can
    // remove the file before async assertions finish on some platforms.
    let _dir = Box::leak(Box::new(dir));
    Arc::new(RedbStore::open(path).expect("redb store should open for host import task test"))
}

pub(super) fn host_import_command(
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
