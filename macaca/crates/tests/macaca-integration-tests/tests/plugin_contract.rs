use macaca_proto::{
    DeveloperId, PluginHostOperation, PluginHostStatus, PluginId, PluginRuntimeKind, PluginVersion,
    TraceContext,
};
use macaca_runtime_host::PluginHostLifecycleSupervisor;
use macaca_sdk::{PluginContext, PluginSdk};

#[test]
fn plugin_sdk_contract_test_accepts_descriptor_manifest() {
    let sdk = PluginSdk::new();
    let manifest = sdk
        .manifest()
        .plugin_id(PluginId::new("plugin.contract.descriptor"))
        .version(PluginVersion::new("1.0.0"))
        .developer_id(DeveloperId::new("developer.contract"))
        .runtime(PluginRuntimeKind::DescriptorOnly)
        .permission("permission.contract", "Contract fixture permission")
        .resource(
            "resource.contract",
            "workspace",
            "Contract fixture resource",
        )
        .signature_fixture()
        .build()
        .unwrap();
    let registration = sdk.registration().manifest(manifest).build().unwrap();

    let report = sdk.contract_tests().validate_registration(&registration);
    assert!(report.is_success(), "{report:?}");
}

#[tokio::test]
async fn plugin_host_supervisor_keeps_process_plugins_unavailable_safe() {
    let context = PluginContext::new(
        PluginId::new("plugin.contract.process"),
        PluginVersion::new("1.0.0"),
        PluginRuntimeKind::Process,
    );
    let supervisor = PluginHostLifecycleSupervisor::new();
    let result = supervisor
        .execute_operation(
            context.host_descriptor(),
            PluginHostOperation::Start,
            serde_json::json!({ "entry": "ignored-by-skeleton" }),
            TraceContext::new("plugin-contract-trace"),
        )
        .await
        .unwrap();

    assert_eq!(result.status, PluginHostStatus::Unavailable);
    assert_eq!(
        result.error_code.as_deref(),
        Some("plugin_host_unavailable")
    );
    assert_eq!(result.trace.trace_id, "plugin-contract-trace");
}
