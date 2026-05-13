//! WASM Application Platform fixture construction.
//!
//! This child module keeps the WASM artifact-admission fixture separate from
//! the generic package fixtures so `package_fixtures.rs` stays below the
//! project file-size limit while preserving the same public SDK function.

use macaca_proto::{
    ApplicationExport, ApplicationImport, WasmAbiRequirement, WasmArtifactDigest,
    WasmComponentArtifactDescriptor, WasmExportDeclaration, WasmImportRequirement,
    WasmRuntimeArtifactRef,
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
    let artifact = WasmComponentArtifactDescriptor::new(
        format!("{}.artifact", fixture.manifest.package_id),
        WasmRuntimeArtifactRef::new(format!(
            "pkg://{}/component.wasm",
            fixture.manifest.package_id
        )),
        WasmArtifactDigest::sha256("fixture-platform-wasm-digest"),
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
