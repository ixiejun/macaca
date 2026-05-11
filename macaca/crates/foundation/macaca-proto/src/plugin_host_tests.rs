use super::*;

use crate::{PluginId, PluginRuntimeKind, PluginVersion, TraceContext};

fn descriptor(runtime_kind: PluginRuntimeKind) -> PluginHostDescriptor {
    let mut descriptor = PluginHostDescriptor::new(
        PluginId::new("plugin.host.fixture"),
        PluginVersion::new("1.0.0"),
        runtime_kind,
        PluginAbiVersion::default(),
    );
    descriptor.entry_point = Some(PluginHostEntryPoint::new("descriptor", "manifest"));
    descriptor
}

#[test]
fn host_descriptor_round_trips_without_runtime_internals() {
    let descriptor = descriptor(PluginRuntimeKind::Wasm);
    let encoded = serde_json::to_string(&descriptor).unwrap();
    let decoded: PluginHostDescriptor = serde_json::from_str(&encoded).unwrap();

    assert_eq!(decoded.plugin_id, descriptor.plugin_id);
    assert_eq!(decoded.runtime_kind, PluginRuntimeKind::Wasm);
    assert_eq!(decoded.abi_version.as_str(), "plugin_abi_v1");
}

#[test]
fn unavailable_result_is_structured_and_traceable() {
    let command = PluginHostCommand::new(
        descriptor(PluginRuntimeKind::Process),
        PluginHostOperation::Start,
        serde_json::json!({ "bounded": true }),
        TraceContext::new("plugin-host-trace"),
    );
    let result = PluginHostResult::unavailable(&command, "process host disabled");

    assert_eq!(result.status, PluginHostStatus::Unavailable);
    assert_eq!(
        result.error_code.as_deref(),
        Some("plugin_host_unavailable")
    );
    assert_eq!(result.trace.trace_id, "plugin-host-trace");
}

#[test]
fn timeout_policy_resolves_to_bounded_value() {
    assert_eq!(PluginHostTimeoutPolicy::Default.as_millis(500), Some(500));
    assert_eq!(
        PluginHostTimeoutPolicy::Millis(100).as_millis(500),
        Some(100)
    );
    assert_eq!(PluginHostTimeoutPolicy::Disabled.as_millis(500), None);
}
