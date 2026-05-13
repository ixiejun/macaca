use macaca_proto::{ApplicationExport, ApplicationImport, MACACA_APPLICATION_WIT};

use super::{
    generate_wasm_guest_bindings, RustWasmBindgenBackend, WasmBindgenBackend, WasmBindgenInput,
};

fn input() -> WasmBindgenInput {
    WasmBindgenInput::canonical(
        "fixture.application.wasm.bindgen",
        "developer.fixture",
        "WASM Bindgen Fixture",
        "1.0.0",
    )
}

#[test]
fn wasm_bindgen_rejects_wit_missing_required_export() {
    let broken_wit = MACACA_APPLICATION_WIT.replace("canonical-export: app:start", "");

    let output = generate_wasm_guest_bindings(input().wit_source(broken_wit));

    assert!(output
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "missing_canonical_export"
            && diagnostic.subject == "app:start"));
}

#[test]
fn wasm_bindgen_generates_binding_plan_from_wit() {
    let output = generate_wasm_guest_bindings(input());

    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(
        output.binding_plan.exports.len(),
        ApplicationExport::required_v0().len()
    );
    assert_eq!(
        output.binding_plan.imports.len(),
        ApplicationImport::v0().len()
    );
    assert_eq!(output.binding_plan.runtime.abi_version, "0");
}

#[test]
fn wasm_bindgen_generates_rust_guest_scaffold() {
    let output = generate_wasm_guest_bindings(input().component_entry("guest/component.wasm"));

    assert!(output
        .rust_source
        .contains("Generated Macaca WASM guest scaffold"));
    assert!(output
        .rust_source
        .contains("pub const PACKAGE_ID: &str = \"fixture.application.wasm.bindgen\""));
    assert!(output
        .rust_source
        .contains("pub const COMPONENT_ENTRY: &str = \"guest/component.wasm\""));
    assert!(output.rust_source.contains("guest_entrypoint"));
}

#[test]
fn wasm_bindgen_registers_mock_host_imports() {
    let output = generate_wasm_guest_bindings(input());

    assert!(output
        .binding_plan
        .mock_host_imports
        .iter()
        .any(|binding| binding.import_name == "macaca:trace/emit"
            && binding.permission == "host.import.trace.emit"
            && binding.trace_required));
}

#[test]
fn wasm_bindgen_fixture_descriptor_is_contract_valid() {
    let output = generate_wasm_guest_bindings(input());

    assert_eq!(
        output.descriptor.manifest.runtime.kind,
        macaca_proto::PackageRuntimeKind::WasmComponent
    );
    assert_eq!(
        output.descriptor.abi.application_id,
        "fixture.application.wasm.bindgen"
    );
    assert_eq!(
        output.descriptor.abi.metadata.get("execution.status"),
        Some(&"runtime_unavailable".to_string())
    );
}

#[test]
fn wasm_bindgen_local_certification_report_is_successful() {
    let backend = RustWasmBindgenBackend;

    let output = backend.generate(input());

    assert!(output.local_contract_report.is_success());
    assert!(output.is_success(), "{output:?}");
}
