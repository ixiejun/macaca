use std::collections::BTreeMap;

use macaca_proto::{
    ApplicationAbiDeclaration, ApplicationAbiError, ApplicationAbiVersion, ApplicationCheckpoint,
    ApplicationExport, ApplicationHostCommand, ApplicationHostCommandResult,
    ApplicationHostCommandStatus, ApplicationImport, ApplicationLifecycleState,
    ApplicationRenderRequest, PackageId, TraceContext,
};
use serde_json::json;

#[test]
fn application_abi_declaration_round_trips() {
    let mut declaration =
        ApplicationAbiDeclaration::v0("app.sample").with_package_id(PackageId::new("pkg.app"));
    declaration.permissions.push("storage.scoped".into());
    declaration
        .metadata
        .insert("source".into(), "unit-test".into());

    let encoded = serde_json::to_string(&declaration).unwrap();
    let decoded: ApplicationAbiDeclaration = serde_json::from_str(&encoded).unwrap();

    assert_eq!(decoded, declaration);
    decoded.validate_required_exports().unwrap();
}

#[test]
fn application_abi_missing_required_export_is_structured_error() {
    let mut declaration = ApplicationAbiDeclaration::v0("app.sample");
    declaration
        .exports
        .retain(|export| export != &ApplicationExport::Start);

    let error = declaration.validate_required_exports().unwrap_err();

    assert_eq!(
        error,
        ApplicationAbiError::MissingExport("app:start".into())
    );
}

#[test]
fn application_abi_host_command_render_checkpoint_and_error_round_trip() {
    let trace = TraceContext::new("trace-abi");
    let command = ApplicationHostCommand::with_trace(
        ApplicationImport::TraceEmit,
        json!({"message": "hello"}),
        trace.clone(),
    );
    let render = ApplicationRenderRequest {
        application_id: "app.sample".into(),
        surface: "main".into(),
        input: json!({"kind": "panel"}),
        trace: Some(trace.clone()),
        metadata: BTreeMap::new(),
    };
    let checkpoint = ApplicationCheckpoint::new(
        "app.sample",
        ApplicationLifecycleState::Paused,
        json!({"opaque": true}),
    );
    let error = ApplicationAbiError::RuntimeUnavailable("wasm runtime missing".into());

    let payload = json!({
        "command": command,
        "render": render,
        "checkpoint": checkpoint,
        "error": error,
    });
    let decoded: serde_json::Value = serde_json::from_str(&payload.to_string()).unwrap();
    let decoded_error: ApplicationAbiError =
        serde_json::from_value(decoded["error"].clone()).unwrap();

    assert_eq!(decoded["command"]["import"], "trace_emit");
    assert_eq!(decoded["render"]["surface"], "main");
    assert_eq!(decoded["checkpoint"]["application_id"], "app.sample");
    assert_eq!(
        decoded_error,
        ApplicationAbiError::RuntimeUnavailable("wasm runtime missing".into())
    );
}

#[test]
fn application_abi_custom_import_and_export_remain_structured() {
    let declaration = ApplicationAbiDeclaration {
        application_id: "app.future".into(),
        package_id: None,
        version: ApplicationAbiVersion::v0(),
        exports: vec![ApplicationExport::Custom("app:future".into())],
        imports: vec![ApplicationImport::Custom("macaca:future/call".into())],
        permissions: Vec::new(),
        metadata: BTreeMap::new(),
    };

    let decoded: ApplicationAbiDeclaration =
        serde_json::from_str(&serde_json::to_string(&declaration).unwrap()).unwrap();

    assert_eq!(decoded.exports[0].to_string(), "app:future");
    assert_eq!(decoded.imports[0].to_string(), "macaca:future/call");
    assert!(matches!(
        ApplicationHostCommandResult::runtime_unavailable("future runtime", None).status,
        ApplicationHostCommandStatus::RuntimeUnavailable { .. }
    ));
}
