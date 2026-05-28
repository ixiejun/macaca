//! Focused tests for `service.config`.
//!
//! These tests prove that configuration is service-owned, layered, redacted,
//! hot-reload observable, schema-backed, requirement-aware, and explicit when
//! unavailable.  They avoid application workflow semantics and exercise only
//! generic OS policy/configuration surfaces.

use std::collections::BTreeMap;

use macaca_kernel::SystemService;
use macaca_proto::workbench::config::*;
use macaca_proto::{
    ServiceCommand, ServiceCommandName, TraceContext, WorkbenchCommandResult,
    WorkbenchCommandStatus,
};
use serde_json::json;

use crate::ConfigSystemServiceProvider;

fn command<T: serde::Serialize>(name: &str, payload: T) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        serde_json::to_value(payload).unwrap(),
        TraceContext::new(format!("trace-{name}")),
    )
}

fn scope() -> ConfigScope {
    ConfigScope {
        application_id: Some("app.generic".into()),
        session_id: Some("session-1".into()),
        project_ref: Some("project://workspace".into()),
        workspace_root: Some("/workspace".into()),
    }
}

fn write(layer: ConfigLayer, key_path: &str, value: serde_json::Value) -> ConfigValueWrite {
    ConfigValueWrite {
        layer,
        key_path: key_path.into(),
        value,
        writer_ref: "writer://test".into(),
        reload: false,
        metadata: BTreeMap::new(),
    }
}

fn decode(value: serde_json::Value) -> WorkbenchCommandResult<ConfigServiceResponse> {
    serde_json::from_value(value).unwrap()
}

#[tokio::test]
async fn layered_read_uses_managed_precedence() {
    let provider = ConfigSystemServiceProvider::mock();
    let _ = decode(
        provider
            .call(command(
                VALUE_WRITE_COMMAND,
                write(ConfigLayer::Default, "runtime.mode", json!("default")),
            ))
            .await
            .unwrap()
            .output,
    );
    let _ = decode(
        provider
            .call(command(
                VALUE_WRITE_COMMAND,
                write(ConfigLayer::Managed, "runtime.mode", json!("managed")),
            ))
            .await
            .unwrap()
            .output,
    );

    let result = decode(
        provider
            .call(command(
                READ_COMMAND,
                ConfigReadRequest {
                    scope: scope(),
                    key_prefix: Some("runtime".into()),
                    include_sources: true,
                },
            ))
            .await
            .unwrap()
            .output,
    );

    let Some(ConfigServiceResponse::Read(effective)) = result.output else {
        panic!("expected effective config");
    };
    assert_eq!(effective.values["runtime.mode"].value, json!("managed"));
    assert_eq!(effective.values["runtime.mode"].layer, ConfigLayer::Managed);
}

#[tokio::test]
async fn secret_values_are_redacted_and_batch_writes_are_atomic() {
    let provider = ConfigSystemServiceProvider::mock();
    let secret = decode(
        provider
            .call(command(
                VALUE_WRITE_COMMAND,
                write(
                    ConfigLayer::User,
                    "provider.secret_token",
                    json!("raw-token"),
                ),
            ))
            .await
            .unwrap()
            .output,
    );
    let Some(ConfigServiceResponse::Written { values, .. }) = secret.output else {
        panic!("expected written secret");
    };
    assert_eq!(values[0].value, json!("<redacted>"));
    assert!(values[0].redacted);

    let failed = provider
        .call(command(
            BATCH_WRITE_COMMAND,
            ConfigBatchWrite {
                writes: vec![
                    write(ConfigLayer::Project, "valid.key", json!(true)),
                    write(ConfigLayer::Project, "invalid key", json!(true)),
                ],
                reload: false,
                metadata: BTreeMap::new(),
            },
        ))
        .await;
    assert!(failed.is_err());

    let result = decode(
        provider
            .call(command(
                READ_COMMAND,
                ConfigReadRequest {
                    scope: scope(),
                    key_prefix: Some("valid".into()),
                    include_sources: false,
                },
            ))
            .await
            .unwrap()
            .output,
    );
    let Some(ConfigServiceResponse::Read(effective)) = result.output else {
        panic!("expected effective config");
    };
    assert!(effective.values.is_empty());
}

#[tokio::test]
async fn hot_reload_emits_bounded_change_event() {
    let provider = ConfigSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    let mut request = write(ConfigLayer::Session, "runtime.reloadable", json!(true));
    request.reload = true;

    let result = decode(
        provider
            .call(command(VALUE_WRITE_COMMAND, request))
            .await
            .unwrap()
            .output,
    );

    assert!(matches!(
        result.output,
        Some(ConfigServiceResponse::Written {
            reload_emitted: true,
            ..
        })
    ));
    let event = events.recv().await.unwrap();
    assert_eq!(event.key_paths, vec!["runtime.reloadable".to_string()]);
    assert!(event.reload);
}

#[tokio::test]
async fn schema_requirements_profiles_and_features_are_generic() {
    let provider = ConfigSystemServiceProvider::local();
    let schema = decode(
        provider
            .call(command(
                SCHEMA_READ_COMMAND,
                ConfigSchemaRead {
                    include_internal: false,
                },
            ))
            .await
            .unwrap()
            .output,
    );
    assert!(matches!(
        schema.output,
        Some(ConfigServiceResponse::Schema(schema))
            if schema.supported_commands.iter().any(|command| command == READ_COMMAND)
                && schema.reload_supported
    ));

    let requirements = decode(
        provider
            .call(command(
                REQUIREMENTS_READ_COMMAND,
                ConfigRequirementsRead { scope: scope() },
            ))
            .await
            .unwrap()
            .output,
    );
    assert!(matches!(
        requirements.output,
        Some(ConfigServiceResponse::Requirements(requirements))
            if requirements.managed_hooks_only
                && requirements.allowed_permissions.iter().any(|item| item == "hook.managed")
    ));

    let profiles = decode(
        provider
            .call(command(
                PERMISSION_PROFILE_LIST_COMMAND,
                PermissionProfileListRequest {
                    scope: scope(),
                    runtime_kind: None,
                },
            ))
            .await
            .unwrap()
            .output,
    );
    assert!(matches!(
        profiles.output,
        Some(ConfigServiceResponse::PermissionProfiles(profiles)) if !profiles.is_empty()
    ));

    let feature = decode(
        provider
            .call(command(
                FEATURE_ENABLEMENT_SET_COMMAND,
                FeatureEnablementSet {
                    feature_key: "service.generic.flag".into(),
                    enabled: true,
                    layer: ConfigLayer::Managed,
                    writer_ref: "writer://test".into(),
                    reload: false,
                    metadata: BTreeMap::new(),
                },
            ))
            .await
            .unwrap()
            .output,
    );
    assert!(matches!(feature.status, WorkbenchCommandStatus::Completed));

    let features = decode(
        provider
            .call(command(
                FEATURE_LIST_COMMAND,
                FeatureListRequest {
                    include_disabled: false,
                },
            ))
            .await
            .unwrap()
            .output,
    );
    assert!(matches!(
        features.output,
        Some(ConfigServiceResponse::Features(features))
            if features.iter().any(|feature| feature.feature_key == "service.generic.flag")
    ));
}

#[tokio::test]
async fn unavailable_provider_returns_structured_unavailable() {
    let provider = ConfigSystemServiceProvider::unavailable("config disabled");
    let result = decode(
        provider
            .call(command(
                FEATURE_LIST_COMMAND,
                FeatureListRequest {
                    include_disabled: true,
                },
            ))
            .await
            .unwrap()
            .output,
    );
    assert!(matches!(
        result.status,
        WorkbenchCommandStatus::Unavailable { .. }
    ));
}
