use macaca_proto::{
    AbilityImplementationKind, ApplicationAbiDeclaration, ApplicationAbilityDescriptor,
    ApplicationAbilityKind, ApplicationRuntimeProfile, DeveloperId, PackageId, PackageRuntimeKind,
    WasmAbiRequirement, WasmArtifactDigest, WasmArtifactSignature, WasmBuildProvenance,
    WasmComponentArtifactDescriptor, WasmEngineCapabilities, WasmRuntimeArtifactRef,
    WasmSupplyChainAttestation, WasmSupplyChainTrustPolicy,
};

use crate::certification::{
    ApplicationCertificationContext, ApplicationCertificationFixture, WasmPackageAdmissionSpec,
    WasmPackageAdmissionStatus,
};

fn wasm_fixture() -> ApplicationCertificationFixture {
    let manifest = macaca_proto::ApplicationManifestV1::new(
        PackageId::new("application.fixture.wasm.supply_chain"),
        DeveloperId::new("developer.fixture"),
        "WASM Supply Chain Fixture",
        "1.0.0",
        ApplicationRuntimeProfile::new(PackageRuntimeKind::WasmComponent, "0"),
        macaca_proto::ApplicationHostRequirementDeclaration::new("0.1.0"),
    )
    .ability(ApplicationAbilityDescriptor::new(
        "ability.fixture.wasm",
        ApplicationAbilityKind::Agent,
        AbilityImplementationKind::WasmComponent,
    ));
    ApplicationCertificationFixture::new("fixture.wasm.supply_chain", manifest).abi(
        ApplicationAbiDeclaration::v0("application.fixture.wasm.supply_chain"),
    )
}

fn executable_capabilities() -> WasmEngineCapabilities {
    let mut capabilities = WasmEngineCapabilities::unavailable();
    capabilities.can_execute = true;
    capabilities.supports_component_model = true;
    capabilities.supports_host_import_bridge = true;
    capabilities
}

fn digest() -> WasmArtifactDigest {
    WasmArtifactDigest::sha256("abc123")
}

fn signed_artifact() -> WasmComponentArtifactDescriptor {
    WasmComponentArtifactDescriptor::new(
        "artifact.fixture",
        WasmRuntimeArtifactRef::new("pkg://fixture/component.wasm"),
        digest(),
    )
    .abi(WasmAbiRequirement::new("0"))
    .supply_chain(
        WasmSupplyChainAttestation::new(
            digest(),
            WasmArtifactSignature::detached("sigstore", "signer.fixture", "sig://fixture"),
            WasmBuildProvenance::new("builder.fixture", "origin.fixture", "build.fixture", 100)
                .expires_at(200),
        )
        .certification_report("cert.fixture"),
    )
}

fn industrial_policy() -> WasmSupplyChainTrustPolicy {
    WasmSupplyChainTrustPolicy::default()
        .trusted_signer("signer.fixture")
        .accepted_origin("origin.fixture")
        .max_age(50, 120)
}

fn context(policy: WasmSupplyChainTrustPolicy) -> ApplicationCertificationContext {
    ApplicationCertificationContext::new()
        .trace_id("trace-wasm-supply-chain")
        .runtime(PackageRuntimeKind::WasmComponent)
        .wasm_runtime_capabilities(executable_capabilities())
        .wasm_supply_chain_policy(policy)
}

fn evaluate(
    artifact: WasmComponentArtifactDescriptor,
    policy: WasmSupplyChainTrustPolicy,
) -> crate::certification::WasmPackageAdmissionReport {
    let fixture = wasm_fixture().wasm_artifact(artifact);
    WasmPackageAdmissionSpec::default().evaluate(&fixture, &context(policy))
}

#[test]
fn wasm_supply_chain_accepts_signed_artifact() {
    let report = evaluate(signed_artifact(), industrial_policy());

    assert_eq!(report.status, WasmPackageAdmissionStatus::Accepted);
    assert!(report.reason_codes.is_empty(), "{report:?}");
    assert!(report.supply_chain.unwrap().is_sanitized());
}

#[test]
fn wasm_supply_chain_rejects_missing_signature() {
    let unsigned = WasmComponentArtifactDescriptor::new(
        "artifact.fixture",
        WasmRuntimeArtifactRef::new("pkg://fixture/component.wasm"),
        digest(),
    )
    .abi(WasmAbiRequirement::new("0"));

    let report = evaluate(unsigned, industrial_policy());

    assert_eq!(report.status, WasmPackageAdmissionStatus::Rejected);
    assert!(report
        .reason_codes
        .iter()
        .any(|code| code == "missing_signature"));
}

#[test]
fn wasm_supply_chain_rejects_digest_mismatch() {
    let artifact = WasmComponentArtifactDescriptor::new(
        "artifact.fixture",
        WasmRuntimeArtifactRef::new("pkg://fixture/component.wasm"),
        digest(),
    )
    .abi(WasmAbiRequirement::new("0"))
    .supply_chain(
        WasmSupplyChainAttestation::new(
            WasmArtifactDigest::sha256("different"),
            WasmArtifactSignature::detached("sigstore", "signer.fixture", "sig://fixture"),
            WasmBuildProvenance::new("builder.fixture", "origin.fixture", "build.fixture", 100),
        )
        .certification_report("cert.fixture"),
    );

    let report = evaluate(artifact, industrial_policy());

    assert!(report
        .reason_codes
        .iter()
        .any(|code| code == "digest_mismatch"));
}

#[test]
fn wasm_supply_chain_rejects_untrusted_signer() {
    let report = evaluate(
        signed_artifact(),
        WasmSupplyChainTrustPolicy::default()
            .trusted_signer("signer.other")
            .accepted_origin("origin.fixture")
            .max_age(50, 120),
    );

    assert!(report
        .reason_codes
        .iter()
        .any(|code| code == "untrusted_signer"));
}

#[test]
fn wasm_supply_chain_rejects_stale_provenance() {
    let report = evaluate(signed_artifact(), industrial_policy().max_age(10, 180));

    assert!(report
        .reason_codes
        .iter()
        .any(|code| code == "stale_provenance"));
}

#[test]
fn wasm_supply_chain_rejects_origin_mismatch() {
    let report = evaluate(
        signed_artifact(),
        WasmSupplyChainTrustPolicy::default()
            .trusted_signer("signer.fixture")
            .accepted_origin("origin.other")
            .max_age(50, 120),
    );

    assert!(report
        .reason_codes
        .iter()
        .any(|code| code == "origin_mismatch"));
}

#[test]
fn wasm_supply_chain_rejects_certification_missing() {
    let artifact = WasmComponentArtifactDescriptor::new(
        "artifact.fixture",
        WasmRuntimeArtifactRef::new("pkg://fixture/component.wasm"),
        digest(),
    )
    .abi(WasmAbiRequirement::new("0"))
    .supply_chain(WasmSupplyChainAttestation::new(
        digest(),
        WasmArtifactSignature::detached("sigstore", "signer.fixture", "sig://fixture"),
        WasmBuildProvenance::new("builder.fixture", "origin.fixture", "build.fixture", 100),
    ));

    let report = evaluate(artifact, industrial_policy());

    assert!(report
        .reason_codes
        .iter()
        .any(|code| code == "certification_missing"));
}
