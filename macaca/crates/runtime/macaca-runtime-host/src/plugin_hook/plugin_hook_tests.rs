use macaca_proto::{
    PluginHookContext, PluginHookDeactivateCommand, PluginHookDecision, PluginHookDescriptor,
    PluginHookFailurePolicy, PluginHookInvokeCommand, PluginHookKind, PluginHookName,
    PluginHookQueryCommand, PluginHookRegisterCommand, PluginId, TraceContext, TraceSchemaRef,
};

use crate::plugin_hook::PluginHookBus;

fn observer(plugin_id: &str, priority: i32) -> PluginHookDescriptor {
    PluginHookDescriptor::observer(
        PluginId::new(plugin_id),
        PluginHookName::BeforeToolCall,
        priority,
        TraceSchemaRef::new("trace.plugin.hook.fixture"),
    )
}

#[tokio::test]
async fn registry_orders_hooks_by_priority_then_plugin_id() {
    let bus = PluginHookBus::in_memory();
    bus.register(PluginHookRegisterCommand {
        plugin_id: PluginId::new("plugin.b"),
        descriptors: vec![observer("plugin.b", 20)],
        trace: TraceContext::new("trace-register-b"),
        metadata: Default::default(),
    })
    .await
    .unwrap();
    bus.register(PluginHookRegisterCommand {
        plugin_id: PluginId::new("plugin.a"),
        descriptors: vec![observer("plugin.a", 10)],
        trace: TraceContext::new("trace-register-a"),
        metadata: Default::default(),
    })
    .await
    .unwrap();

    let hooks = bus
        .query(PluginHookQueryCommand {
            hook_name: Some(PluginHookName::BeforeToolCall),
            plugin_id: None,
            include_inactive: false,
            trace: TraceContext::new("trace-query"),
        })
        .await
        .unwrap();

    assert_eq!(hooks[0].plugin_id, PluginId::new("plugin.a"));
    assert_eq!(hooks[1].plugin_id, PluginId::new("plugin.b"));
}

#[tokio::test]
async fn observer_invocation_returns_noop_outcome() {
    let bus = PluginHookBus::in_memory();
    bus.register(PluginHookRegisterCommand {
        plugin_id: PluginId::new("plugin.observer"),
        descriptors: vec![observer("plugin.observer", 0)],
        trace: TraceContext::new("trace-register"),
        metadata: Default::default(),
    })
    .await
    .unwrap();

    let result = bus
        .invoke(PluginHookInvokeCommand {
            hook_name: PluginHookName::BeforeToolCall,
            expected_kind: Some(PluginHookKind::Observer),
            context: PluginHookContext::global(),
            payload: serde_json::json!({"tool": "generic"}),
            trace: TraceContext::new("trace-invoke"),
            metadata: Default::default(),
        })
        .await
        .unwrap();

    assert_eq!(result.decision, PluginHookDecision::Noop);
    assert_eq!(result.outcomes.len(), 1);
    assert_eq!(result.outcomes[0].status, "ok");
}

#[tokio::test]
async fn blocking_unavailable_hook_uses_descriptor_safe_result() {
    let bus = PluginHookBus::in_memory();
    let mut descriptor = observer("plugin.blocker", 0);
    descriptor.kind = PluginHookKind::Blocking;
    descriptor.failure_policy = PluginHookFailurePolicy::FailClosed;
    bus.register(PluginHookRegisterCommand {
        plugin_id: PluginId::new("plugin.blocker"),
        descriptors: vec![descriptor],
        trace: TraceContext::new("trace-register"),
        metadata: Default::default(),
    })
    .await
    .unwrap();

    let result = bus
        .invoke(PluginHookInvokeCommand {
            hook_name: PluginHookName::BeforeToolCall,
            expected_kind: Some(PluginHookKind::Blocking),
            context: PluginHookContext::global(),
            payload: serde_json::json!({"tool": "generic"}),
            trace: TraceContext::new("trace-invoke"),
            metadata: Default::default(),
        })
        .await
        .unwrap();

    assert_eq!(result.outcomes[0].status, "unavailable");
    assert_eq!(
        result.outcomes[0].error_code.as_deref(),
        Some("execution_host_unavailable")
    );
}

#[tokio::test]
async fn deactivation_removes_active_invocation_targets() {
    let bus = PluginHookBus::in_memory();
    bus.register(PluginHookRegisterCommand {
        plugin_id: PluginId::new("plugin.observer"),
        descriptors: vec![observer("plugin.observer", 0)],
        trace: TraceContext::new("trace-register"),
        metadata: Default::default(),
    })
    .await
    .unwrap();
    bus.deactivate(PluginHookDeactivateCommand {
        plugin_id: PluginId::new("plugin.observer"),
        trace: TraceContext::new("trace-deactivate"),
        reason: "test cleanup".into(),
    })
    .await
    .unwrap();

    let result = bus
        .invoke(PluginHookInvokeCommand {
            hook_name: PluginHookName::BeforeToolCall,
            expected_kind: None,
            context: PluginHookContext::global(),
            payload: serde_json::json!({}),
            trace: TraceContext::new("trace-invoke"),
            metadata: Default::default(),
        })
        .await
        .unwrap();

    assert!(result.outcomes.is_empty());
}
