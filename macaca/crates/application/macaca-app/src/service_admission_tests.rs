use macaca_proto::{ApplicationLifecycleState, ApplicationManifestV1, TraceContext};

use crate::model::{AppLayer, AppManifest, AppStatus};
use crate::service_admission::{
    app_status_from_lifecycle, lifecycle_from_app_status, ApplicationManifestSpec,
    ApplicationManifestV1Spec, ApplicationTraceSpec,
};
use crate::service_capability::AppServiceContractConfig;

#[test]
fn lifecycle_projection_preserves_running_status() {
    let state = lifecycle_from_app_status(AppStatus::Running);
    assert_eq!(state, ApplicationLifecycleState::Started);
    assert_eq!(app_status_from_lifecycle(&state), AppStatus::Running);
}

#[test]
fn trace_spec_rejects_blank_trace() {
    let error = ApplicationTraceSpec
        .validate(&TraceContext::new(" "))
        .expect_err("blank trace id must be rejected");
    assert!(error.to_string().contains("trace"));
}

#[test]
fn manifest_v1_spec_rejects_missing_ability() {
    let manifest = ApplicationManifestV1::new(
        macaca_proto::PackageId::new("application.invalid"),
        macaca_proto::DeveloperId::new("developer.invalid"),
        "Invalid",
        "1.0.0",
        macaca_proto::ApplicationRuntimeProfile::new(macaca_proto::PackageRuntimeKind::Yaml, "1"),
        macaca_proto::ApplicationHostRequirementDeclaration::new("0.1.0"),
    );

    let report = ApplicationManifestV1Spec.validate(&manifest);
    assert!(!report.is_success());
    assert!(report
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "missing_ability"));
}

#[test]
fn manifest_spec_rejects_empty_declared_service_identifier() {
    let manifest = AppManifest {
        id: macaca_proto::ApplicationId::new(),
        name: "service-invalid".into(),
        description: None,
        version: "1.0.0".into(),
        layer: AppLayer::L2Wasm,
        ui_type: None,
        agents: vec![],
        llm_config: None,
        entry_agent: None,
        entrypoint: None,
        workflows: None,
        resources: None,
        context: None,
        service_contract: Some(AppServiceContractConfig {
            use_packs: vec![],
            required_services: vec!["".into()],
            optional_services: vec![],
            service_policy_overrides: Default::default(),
            ..Default::default()
        }),
        execution_profile: None,
        workbench: None,
        autonomy: None,
        ui: None,
        execution_control: None,
    };
    let error = ApplicationManifestSpec.validate(&manifest).unwrap_err();
    assert!(error.to_string().contains("required_services"));
}

#[test]
fn manifest_spec_rejects_invalid_pack_declaration() {
    let manifest = AppManifest {
        id: macaca_proto::ApplicationId::new(),
        name: "pack-invalid".into(),
        description: None,
        version: "1.0.0".into(),
        layer: AppLayer::L2Wasm,
        ui_type: None,
        agents: vec![],
        llm_config: None,
        entry_agent: None,
        entrypoint: None,
        workflows: None,
        resources: None,
        context: None,
        service_contract: Some(AppServiceContractConfig {
            required_packs: vec!["invalid.pack.v1".into()],
            ..Default::default()
        }),
        execution_profile: None,
        workbench: None,
        autonomy: None,
        ui: None,
        execution_control: None,
    };
    let error = ApplicationManifestSpec.validate(&manifest).unwrap_err();
    assert!(error.to_string().contains("domain pack id"));
}
