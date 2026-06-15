//! Plugin system design contract fixtures.
//!
//! These integration tests define several small "test plugins" as SDK/protocol
//! fixtures. They intentionally do not compile or execute third-party plugin
//! code. The goal is to verify that Macaca's current Plugin Fabric matches the
//! intended design: contract-first metadata, service-boundary management,
//! capability/hook registration, unavailable-safe execution hosts, sanitized
//! diagnostics, and traceable lifecycle surfaces.

use std::sync::Arc;

#[path = "plugin_system_design_contracts/fixtures.rs"]
mod fixtures;

use fixtures::{
    base_manifest, blocking_hook, control_plane_manifest, install_request, observer_hook,
    tool_capability, BuiltInAdapterPlugin, DescriptorToolPlugin, ProcessSkeletonPlugin,
    RemoteProxySkeletonPlugin, TestPluginFixture, WasmSkeletonPlugin,
};

use macaca_proto::{
    CapabilityId, KernelServiceId, PluginActivationState, PluginCapabilityCallCommand,
    PluginCapabilityQueryCommand, PluginHealth, PluginHookContext, PluginHookDecision,
    PluginHookInvokeCommand, PluginHookKind, PluginHookName, PluginHookQueryCommand,
    PluginHookRegisterCommand, PluginHostOperation, PluginHostStatus, PluginId, PluginRuntimeKind,
    PluginServiceCategory, PluginTargetCommand, TraceContext,
};
use macaca_runtime_host::{
    PluginCapabilityService, PluginControlService, PluginHookBus, PluginHostLifecycleSupervisor,
};
use macaca_sdk::PluginSdk;

#[test]
fn sdk_test_plugins_satisfy_contract_first_registration() {
    for plugin in [
        Box::new(DescriptorToolPlugin) as Box<dyn TestPluginFixture>,
        Box::new(BuiltInAdapterPlugin),
        Box::new(WasmSkeletonPlugin),
        Box::new(ProcessSkeletonPlugin),
        Box::new(RemoteProxySkeletonPlugin),
    ] {
        let report = plugin.contract_report();
        assert!(
            report.is_success(),
            "fixture {} should satisfy SDK contracts: {report:?}",
            plugin.plugin_id()
        );

        let boundary_report = PluginSdk::new()
            .contract_tests()
            .validate_boundary_compliance(&plugin.registration().manifest);
        assert!(
            boundary_report.is_success(),
            "fixture {} should preserve protocol microkernel plugin boundary: {boundary_report:?}",
            plugin.plugin_id()
        );
    }
}

#[test]
fn sdk_test_plugin_rejects_missing_permission_declaration() {
    let sdk = PluginSdk::new();
    let manifest = base_manifest(
        &sdk,
        PluginId::new("plugin.test.invalid_missing_permission"),
        PluginRuntimeKind::DescriptorOnly,
    )
    .service(
        KernelServiceId::new("service.test.invalid"),
        PluginServiceCategory::Skill,
        "permission.test.missing",
    )
    .resource(
        "resource.test.invalid",
        "workspace",
        "Invalid fixture resource",
    )
    .signature_fixture()
    .build()
    .unwrap();
    let report = sdk
        .contract_tests()
        .validate_registration(&sdk.registration().manifest(manifest).build().unwrap());

    assert!(!report.is_success());
    assert!(report
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "missing_permission_declaration"));
}

#[tokio::test]
async fn control_plane_installs_descriptor_plugin_and_sanitizes_diagnostics() {
    let capability_service = Arc::new(PluginCapabilityService::in_memory());
    let control = PluginControlService::builder()
        .capability_service(Arc::clone(&capability_service))
        .build();
    let mut request = install_request(control_plane_manifest(), true);
    request
        .metadata
        .insert("required_config".into(), "test.enabled".into());
    request
        .metadata
        .insert("required_secrets".into(), "TEST_PLUGIN_TOKEN".into());
    request
        .metadata
        .insert("secret_value".into(), "must-not-leak".into());

    let result = control.install(request).await.unwrap();
    assert!(result.record.enabled);
    assert_eq!(
        result.record.activation_state,
        PluginActivationState::Enabled
    );
    assert!(result
        .events
        .iter()
        .all(|event| !event.trace.trace_id.is_empty()));

    let listed = control
        .list(macaca_proto::PluginListCommand {
            repository_id: None,
            include_disabled: true,
            trace: TraceContext::new("trace-plugin-test-list"),
        })
        .await
        .unwrap();
    assert_eq!(listed.len(), 1);

    let diagnostics = control
        .diagnostics(PluginTargetCommand::new(
            PluginId::new("plugin.test.controlled_descriptor"),
            TraceContext::new("trace-plugin-test-diagnostics"),
        ))
        .await
        .unwrap();
    let encoded = serde_json::to_string(&diagnostics).unwrap();
    assert!(encoded.contains("TEST_PLUGIN_TOKEN"));
    assert!(!encoded.contains("must-not-leak"));

    let capabilities = capability_service
        .query(PluginCapabilityQueryCommand {
            kind: None,
            plugin_id: Some(PluginId::new("plugin.test.controlled_descriptor")),
            slot_namespace: None,
            slot_key: None,
            include_inactive: false,
            trace: TraceContext::new("trace-plugin-test-capability-query"),
        })
        .await
        .unwrap();
    assert_eq!(capabilities.len(), 1);
}

#[tokio::test]
async fn capability_plugin_returns_structured_unavailable_without_execution_host() {
    let service = PluginCapabilityService::in_memory();
    let descriptor = tool_capability(
        PluginId::new("plugin.test.capability_call"),
        "capability.test.call",
    )
    .build();
    service
        .register(macaca_proto::PluginCapabilityRegisterCommand {
            plugin_id: PluginId::new("plugin.test.capability_call"),
            descriptors: vec![descriptor],
            trace: TraceContext::new("trace-plugin-test-register-capability"),
            metadata: Default::default(),
        })
        .await
        .unwrap();

    let err = service
        .call(PluginCapabilityCallCommand {
            capability_id: CapabilityId::new("capability.test.call"),
            plugin_id: Some(PluginId::new("plugin.test.capability_call")),
            input: serde_json::json!({ "bounded": true }),
            trace: TraceContext::new("trace-plugin-test-call-capability"),
            metadata: Default::default(),
        })
        .await
        .unwrap_err();

    assert!(err.to_string().contains("execution host is unavailable"));
}

#[tokio::test]
async fn hook_plugins_have_deterministic_order_and_safe_outcomes() {
    let bus = PluginHookBus::in_memory();
    bus.register(PluginHookRegisterCommand {
        plugin_id: PluginId::new("plugin.test.hook_observer_b"),
        descriptors: vec![observer_hook("plugin.test.hook_observer_b", 20)],
        trace: TraceContext::new("trace-plugin-test-register-hook-b"),
        metadata: Default::default(),
    })
    .await
    .unwrap();
    bus.register(PluginHookRegisterCommand {
        plugin_id: PluginId::new("plugin.test.hook_observer_a"),
        descriptors: vec![observer_hook("plugin.test.hook_observer_a", 10)],
        trace: TraceContext::new("trace-plugin-test-register-hook-a"),
        metadata: Default::default(),
    })
    .await
    .unwrap();

    let registrations = bus
        .query(PluginHookQueryCommand {
            hook_name: Some(PluginHookName::BeforeToolCall),
            plugin_id: None,
            include_inactive: false,
            trace: TraceContext::new("trace-plugin-test-query-hooks"),
        })
        .await
        .unwrap();
    assert_eq!(
        registrations[0].plugin_id,
        PluginId::new("plugin.test.hook_observer_a")
    );

    let observer_result = bus
        .invoke(PluginHookInvokeCommand {
            hook_name: PluginHookName::BeforeToolCall,
            expected_kind: Some(PluginHookKind::Observer),
            context: PluginHookContext::global(),
            payload: serde_json::json!({ "payload": "bounded-metadata-only" }),
            trace: TraceContext::new("trace-plugin-test-invoke-observer"),
            metadata: Default::default(),
        })
        .await
        .unwrap();
    assert_eq!(observer_result.decision, PluginHookDecision::Noop);
    assert_eq!(observer_result.outcomes.len(), 2);

    bus.register(PluginHookRegisterCommand {
        plugin_id: PluginId::new("plugin.test.hook_blocking"),
        descriptors: vec![blocking_hook("plugin.test.hook_blocking")],
        trace: TraceContext::new("trace-plugin-test-register-blocking"),
        metadata: Default::default(),
    })
    .await
    .unwrap();

    let blocking_result = bus
        .invoke(PluginHookInvokeCommand {
            hook_name: PluginHookName::BeforeToolCall,
            expected_kind: Some(PluginHookKind::Blocking),
            context: PluginHookContext::global(),
            payload: serde_json::json!({ "payload": "bounded-metadata-only" }),
            trace: TraceContext::new("trace-plugin-test-invoke-blocking"),
            metadata: Default::default(),
        })
        .await
        .unwrap();
    assert_eq!(blocking_result.outcomes[0].status, "unavailable");
    assert_eq!(
        blocking_result.outcomes[0].error_code.as_deref(),
        Some("execution_host_unavailable")
    );
}

#[tokio::test]
async fn host_plugins_are_traceable_and_unavailable_safe_by_runtime_kind() {
    let supervisor = PluginHostLifecycleSupervisor::new();
    let descriptor_result = supervisor
        .execute_operation(
            DescriptorToolPlugin.host_context().host_descriptor(),
            PluginHostOperation::Start,
            serde_json::json!({}),
            TraceContext::new("trace-plugin-test-descriptor-host"),
        )
        .await
        .unwrap();
    assert_eq!(descriptor_result.status, PluginHostStatus::Accepted);

    let built_in_result = supervisor
        .execute_operation(
            BuiltInAdapterPlugin.host_context().host_descriptor(),
            PluginHostOperation::Start,
            serde_json::json!({}),
            TraceContext::new("trace-plugin-test-builtin-host"),
        )
        .await
        .unwrap();
    assert_eq!(built_in_result.status, PluginHostStatus::Accepted);

    for plugin in [
        Box::new(WasmSkeletonPlugin) as Box<dyn TestPluginFixture>,
        Box::new(ProcessSkeletonPlugin),
        Box::new(RemoteProxySkeletonPlugin),
    ] {
        let result = supervisor
            .execute_operation(
                plugin.host_context().host_descriptor(),
                PluginHostOperation::Start,
                serde_json::json!({ "entry": "ignored-by-skeleton" }),
                TraceContext::new(format!("trace-plugin-test-{}", plugin.plugin_id())),
            )
            .await
            .unwrap();
        assert_eq!(result.status, PluginHostStatus::Unavailable);
        assert_eq!(
            result.error_code.as_deref(),
            Some("plugin_host_unavailable")
        );

        let health = supervisor
            .health(
                plugin.host_context().host_descriptor(),
                TraceContext::new(format!("trace-plugin-health-{}", plugin.plugin_id())),
            )
            .await
            .unwrap();
        assert!(matches!(health.health, PluginHealth::Unavailable { .. }));
        assert!(!health.trace.trace_id.is_empty());
    }
}
