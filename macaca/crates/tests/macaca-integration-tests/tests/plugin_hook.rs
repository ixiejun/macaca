//! Plugin Hook Bus service-boundary integration tests.
//!
//! This test keeps plugin hooks behind `ServiceRuntime` so the integration path
//! mirrors Web/CLI/SDK usage: callers submit typed commands through a generic
//! service envelope and never access runtime-host registry internals directly.

use std::sync::Arc;

use macaca_proto::{
    plugin_hook_bus_service_id, PluginHookContext, PluginHookDecision, PluginHookDescriptor,
    PluginHookInvokeCommand, PluginHookName, PluginHookRegisterCommand, PluginId, ServiceBusSource,
    TraceContext, TraceSchemaRef, PLUGIN_HOOK_INVOKE_COMMAND, PLUGIN_HOOK_REGISTER_COMMAND,
};
use macaca_runtime_host::{
    plugin_hook_service_command, plugin_hook_service_descriptor, PluginHookSystemServiceProvider,
    ServiceProviderFactoryContext, ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig,
    StaticServiceProviderFactory,
};

#[tokio::test]
async fn plugin_hook_bus_invokes_through_service_runtime() {
    let runtime = ServiceRuntime::new(ServiceRuntimeConfig::default());
    let service_id = runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(
                plugin_hook_service_descriptor(),
                Arc::new(PluginHookSystemServiceProvider::in_memory()),
            )),
            ServiceProviderFactoryContext::new(),
        )
        .await
        .unwrap();
    assert_eq!(service_id, plugin_hook_bus_service_id());
    runtime
        .start(&service_id, TraceContext::new("trace-hook-start"))
        .await
        .unwrap();

    let descriptor = PluginHookDescriptor::observer(
        PluginId::new("plugin.integration"),
        PluginHookName::BeforeToolCall,
        0,
        TraceSchemaRef::new("trace.plugin.integration.hook"),
    );
    let register = plugin_hook_service_command(
        PLUGIN_HOOK_REGISTER_COMMAND,
        PluginHookRegisterCommand {
            plugin_id: PluginId::new("plugin.integration"),
            descriptors: vec![descriptor],
            trace: TraceContext::new("trace-hook-register"),
            metadata: Default::default(),
        },
        TraceContext::new("trace-hook-register-envelope"),
    )
    .unwrap();
    runtime
        .call(
            &service_id,
            ServiceBusSource::new("integration.plugin_hook"),
            register,
        )
        .await
        .unwrap();

    let invoke = plugin_hook_service_command(
        PLUGIN_HOOK_INVOKE_COMMAND,
        PluginHookInvokeCommand {
            hook_name: PluginHookName::BeforeToolCall,
            expected_kind: None,
            context: PluginHookContext::global(),
            payload: serde_json::json!({"tool_name": "generic"}),
            trace: TraceContext::new("trace-hook-invoke"),
            metadata: Default::default(),
        },
        TraceContext::new("trace-hook-invoke-envelope"),
    )
    .unwrap();
    let reply = runtime
        .call(
            &service_id,
            ServiceBusSource::new("integration.plugin_hook"),
            invoke,
        )
        .await
        .unwrap();

    let output = reply.output.expect("hook service should return output");
    let result: macaca_proto::PluginHookInvokeResult = serde_json::from_value(output).unwrap();
    assert_eq!(result.decision, PluginHookDecision::Noop);
    assert_eq!(result.outcomes.len(), 1);
    assert_eq!(
        result.outcomes[0].plugin_id,
        PluginId::new("plugin.integration")
    );
}
