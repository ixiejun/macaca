use crate::{
    plugin_hook_bus_service_id, PluginHookContext, PluginHookDecision, PluginHookDescriptor,
    PluginHookInvokeCommand, PluginHookKind, PluginHookName, PluginHookTimeoutPolicy, PluginId,
    TraceContext, TraceSchemaRef, PLUGIN_HOOK_BUS_SERVICE_ID,
};

#[test]
fn hook_descriptor_round_trips_through_serde() {
    let mut descriptor = PluginHookDescriptor::observer(
        PluginId::new("plugin.fixture"),
        PluginHookName::BeforeToolCall,
        10,
        TraceSchemaRef::new("trace.plugin.hook.fixture"),
    );
    descriptor.required_permissions.push("tool.call".into());
    descriptor.timeout_policy = PluginHookTimeoutPolicy::Millis(25);

    let encoded = serde_json::to_string(&descriptor).unwrap();
    let decoded: PluginHookDescriptor = serde_json::from_str(&encoded).unwrap();

    assert_eq!(decoded.plugin_id, PluginId::new("plugin.fixture"));
    assert_eq!(decoded.hook_name, PluginHookName::BeforeToolCall);
    assert_eq!(decoded.kind, PluginHookKind::Observer);
    assert_eq!(decoded.timeout_policy.as_millis(100), Some(25));
}

#[test]
fn hook_invocation_context_is_bounded_metadata() {
    let command = PluginHookInvokeCommand {
        hook_name: PluginHookName::BeforeLlmCall,
        expected_kind: Some(PluginHookKind::Blocking),
        context: PluginHookContext::global(),
        payload: serde_json::json!({
            "model_family": "generic",
            "message_count": 2,
            "prompt_chars": 120
        }),
        trace: TraceContext::new("trace-hook-invoke"),
        metadata: Default::default(),
    };

    let encoded = serde_json::to_value(&command).unwrap();
    assert_eq!(encoded["hook_name"], "before_llm_call");
    assert_eq!(encoded["expected_kind"], "blocking");
    assert_eq!(
        plugin_hook_bus_service_id().as_str(),
        PLUGIN_HOOK_BUS_SERVICE_ID
    );
}

#[test]
fn hook_decisions_are_explicit() {
    let decision = PluginHookDecision::RequireApproval;
    let encoded = serde_json::to_string(&decision).unwrap();
    assert_eq!(encoded, "\"require_approval\"");
}
