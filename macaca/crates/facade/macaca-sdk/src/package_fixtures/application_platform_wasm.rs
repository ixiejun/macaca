//! WASM Application Platform fixture construction.
//!
//! This child module keeps the WASM artifact-admission fixture separate from
//! the generic package fixtures so `package_fixtures.rs` stays below the
//! project file-size limit while preserving the same public SDK function.

use macaca_proto::{
    ApplicationExport, ApplicationImport, WasmAbiRequirement, WasmArtifactDigest,
    WasmArtifactSignature, WasmBuildProvenance, WasmComponentArtifactDescriptor,
    WasmExportDeclaration, WasmImportRequirement, WasmRuntimeArtifactRef,
    WasmSupplyChainAttestation,
};

use crate::application_kit::WasmComponentApplicationScaffold;
use crate::package_fixtures::ApplicationPlatformFixture;

/// Return a Manifest v1 WASM skeleton fixture with ABI and artifact metadata.
pub fn application_platform_wasm_skeleton_fixture() -> ApplicationPlatformFixture {
    let fixture = WasmComponentApplicationScaffold::new(
        "fixture.application.platform.wasm",
        "developer.fixture",
        "Application Platform WASM Fixture",
        "1.0.0",
    )
    .build();
    let ability_id = format!("{}.component", fixture.manifest.package_id);
    let digest = WasmArtifactDigest::sha256("fixture-platform-wasm-digest");
    let artifact = WasmComponentArtifactDescriptor::new(
        format!("{}.artifact", fixture.manifest.package_id),
        WasmRuntimeArtifactRef::new(format!(
            "pkg://{}/component.wasm",
            fixture.manifest.package_id
        )),
        digest.clone(),
    )
    .supply_chain(
        WasmSupplyChainAttestation::new(
            digest,
            WasmArtifactSignature::detached(
                "sigstore",
                "fixture.application-platform.signer",
                "sig://fixture/application-platform-wasm",
            ),
            WasmBuildProvenance::new(
                "fixture.application-platform.builder",
                "fixture.application-platform.origin",
                "fixture.application-platform.build",
                1_776_800_000,
            )
            .expires_at(1_780_000_000),
        )
        .certification_report("fixture.application-platform.certification"),
    )
    .abi(WasmAbiRequirement::new("0"))
    .required_import(WasmImportRequirement::permissioned(
        ApplicationImport::TraceEmit,
        "trace.emit",
    ))
    .required_import(WasmImportRequirement::permissioned(
        ApplicationImport::ServiceCall,
        "service.call",
    ))
    .export(WasmExportDeclaration::ability(
        ApplicationExport::Start,
        ability_id,
    ));
    ApplicationPlatformFixture::new("fixture.platform.wasm", fixture.manifest)
        .abi(fixture.abi)
        .wasm_artifact(artifact)
}
