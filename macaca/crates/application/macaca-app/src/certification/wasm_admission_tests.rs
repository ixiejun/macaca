use macaca_proto::{
    AbilityImplementationKind, ApplicationAbiDeclaration, ApplicationAbilityDescriptor,
    ApplicationAbilityKind, ApplicationImport, ApplicationRuntimeProfile, DeveloperId, PackageId,
    PackageRuntimeKind, WasmAbiRequirement, WasmArtifactDigest, WasmComponentArtifactDescriptor,
    WasmEngineCapabilities, WasmImportRequirement, WasmRuntimeArtifactRef,
};

use crate::certification::{
    ApplicationCertificationContext, ApplicationCertificationFixture, ApplicationCertificationKit,
    WasmPackageAdmissionSpec, WasmPackageAdmissionStatus,
};

fn wasm_fixture() -> ApplicationCertificationFixture {
    let manifest = macaca_proto::ApplicationManifestV1::new(
        PackageId::new("application.fixture.wasm.admission"),
        DeveloperId::new("developer.fixture"),
        "WASM Admission Fixture",
        "1.0.0",
        ApplicationRuntimeProfile::new(PackageRuntimeKind::WasmComponent, "0"),
        macaca_proto::ApplicationCompatibilityDeclaration::new("0.1.0"),
    )
    .ability(ApplicationAbilityDescriptor::new(
        "ability.fixture.wasm",
        ApplicationAbilityKind::Agent,
        AbilityImplementationKind::WasmComponent,
    ))
    .permission(macaca_proto::ApplicationPermissionDeclaration::required(
        "storage.read",
        "Read application storage",
    ));
    ApplicationCertificationFixture::new("fixture.wasm.admission", manifest).abi(
        ApplicationAbiDeclaration::v0("application.fixture.wasm.admission"),
    )
}

fn artifact() -> WasmComponentArtifactDescriptor {
    WasmComponentArtifactDescriptor::new(
        "artifact.fixture",
        WasmRuntimeArtifactRef::new("pkg://fixture/component.wasm"),
        WasmArtifactDigest::sha256("abc123"),
    )
    .abi(WasmAbiRequirement::new("0"))
    .required_import(WasmImportRequirement::permissioned(
        ApplicationImport::StorageGet,
        "storage.read",
    ))
}

fn executable_capabilities() -> WasmEngineCapabilities {
    let mut capabilities = WasmEngineCapabilities::unavailable();
    capabilities.can_execute = true;
    capabilities.supports_component_model = true;
    capabilities.supports_host_import_bridge = true;
    capabilities
}

#[test]
fn wasm_admission_accepts_matching_artifact_and_abi() {
    let context = ApplicationCertificationContext::new()
        .trace_id("trace-wasm-admission")
        .runtime(PackageRuntimeKind::WasmComponent)
        .wasm_runtime_capabilities(executable_capabilities());
    let fixture = wasm_fixture().wasm_artifact(artifact());

    let report = WasmPackageAdmissionSpec::default().evaluate(&fixture, &context);

    assert_eq!(report.status, WasmPackageAdmissionStatus::Accepted);
    assert!(report.reason_codes.is_empty(), "{report:?}");
}

#[test]
fn wasm_admission_rejects_missing_artifact_digest() {
    let context = ApplicationCertificationContext::new()
        .trace_id("trace-wasm-admission")
        .runtime(PackageRuntimeKind::WasmComponent)
        .wasm_runtime_capabilities(executable_capabilities());
    let fixture = wasm_fixture().wasm_artifact(WasmComponentArtifactDescriptor::new(
        "artifact.fixture",
        WasmRuntimeArtifactRef::new("pkg://fixture/component.wasm"),
        WasmArtifactDigest {
            algorithm: "sha256".into(),
            value: "".into(),
        },
    ));

    let report = WasmPackageAdmissionSpec::default().evaluate(&fixture, &context);

    assert_eq!(report.status, WasmPackageAdmissionStatus::Rejected);
    assert!(report
        .reason_codes
        .iter()
        .any(|code| code == "artifact_digest_missing"));
}

#[test]
fn wasm_admission_rejects_abi_mismatch() {
    let context = ApplicationCertificationContext::new()
        .trace_id("trace-wasm-admission")
        .runtime(PackageRuntimeKind::WasmComponent)
        .wasm_runtime_capabilities(executable_capabilities());
    let fixture = wasm_fixture().wasm_artifact(artifact().abi(WasmAbiRequirement::new("99")));

    let report = WasmPackageAdmissionSpec::default().evaluate(&fixture, &context);

    assert!(report
        .reason_codes
        .iter()
        .any(|code| code == "abi_version_mismatch"));
}

#[test]
fn wasm_admission_rejects_missing_import_permission() {
    let context = ApplicationCertificationContext::new()
        .trace_id("trace-wasm-admission")
        .runtime(PackageRuntimeKind::WasmComponent)
        .wasm_runtime_capabilities(executable_capabilities());
    let fixture = wasm_fixture().wasm_artifact(artifact().required_import(
        WasmImportRequirement::permissioned(ApplicationImport::StorageSet, "storage.write"),
    ));

    let report = WasmPackageAdmissionSpec::default().evaluate(&fixture, &context);

    assert!(report
        .reason_codes
        .iter()
        .any(|code| code == "import_permission_missing"));
}

#[test]
fn wasm_admission_report_is_sanitized() {
    let context = ApplicationCertificationContext::new()
        .trace_id("trace-wasm-admission")
        .runtime(PackageRuntimeKind::WasmComponent);
    let mut unsafe_artifact = artifact();
    unsafe_artifact
        .metadata
        .insert("raw_manifest".into(), "raw wasm bytes with API_KEY".into());
    let fixture = wasm_fixture().wasm_artifact(unsafe_artifact);

    let report = WasmPackageAdmissionSpec::default().evaluate(&fixture, &context);
    let encoded = serde_json::to_string(&report).unwrap().to_lowercase();

    assert!(report.is_sanitized());
    assert!(!encoded.contains("raw wasm"));
    assert!(!encoded.contains("api_key"));
}

#[test]
fn certification_kit_includes_wasm_admission_report() {
    let context = ApplicationCertificationContext::new()
        .trace_id("trace-wasm-admission")
        .runtime(PackageRuntimeKind::WasmComponent)
        .wasm_runtime_capabilities(executable_capabilities());
    let fixture = wasm_fixture().wasm_artifact(artifact());

    let report = ApplicationCertificationKit.certify(&fixture, &context);

    assert!(report.wasm_admission.is_some());
    assert!(report.is_success(), "{report:?}");
}
