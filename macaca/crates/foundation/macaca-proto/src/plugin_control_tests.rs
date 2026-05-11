use super::*;
use crate::{
    DeveloperId, PluginId, PluginRuntimeDeclaration, PluginRuntimeKind, PluginVersion, TraceContext,
};

fn fixture_manifest() -> PluginManifest {
    PluginManifest::new(
        PluginId::new("plugin.control.fixture"),
        PluginVersion::new("1.0.0"),
        DeveloperId::new("developer.fixture"),
        PluginRuntimeDeclaration::new(PluginRuntimeKind::DescriptorOnly, "1"),
    )
}

#[test]
fn control_records_round_trip_without_secret_values() {
    let manifest = fixture_manifest();
    let record = PluginControlRecord {
        plugin_id: manifest.plugin_id.clone(),
        version: manifest.version.clone(),
        repository_id: "user-local".into(),
        source_kind: PluginInstallSourceKind::UserLocal,
        activation_state: PluginActivationState::Enabled,
        lifecycle_state: PluginLifecycleState::Registered,
        health: PluginHealth::Unknown,
        enabled: true,
        installed_at: Utc::now(),
        updated_at: Utc::now(),
        manifest,
        config_requirements: vec![PluginConfigRequirement {
            key: "PLUGIN_MODE".into(),
            required: true,
            present: false,
            description: "Mode selection".into(),
        }],
        secret_requirements: vec![PluginSecretRequirement {
            name: "PLUGIN_TOKEN".into(),
            required: true,
            present: false,
            source_hint: Some("env".into()),
        }],
        metadata: BTreeMap::new(),
    };

    let encoded = serde_json::to_string(&record).unwrap();
    assert!(encoded.contains("PLUGIN_TOKEN"));
    assert!(!encoded.contains("secret-value"));
    let decoded: PluginControlRecord = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded.plugin_id, record.plugin_id);
    assert_eq!(decoded.secret_requirements[0].present, false);
}

#[test]
fn state_changing_commands_carry_trace_context() {
    let command = PluginTargetCommand::new(
        PluginId::new("plugin.control.fixture"),
        TraceContext::new("trace-plugin-control"),
    );

    assert_eq!(command.trace.trace_id, "trace-plugin-control");
    assert_eq!(command.plugin_id.as_str(), "plugin.control.fixture");
}
