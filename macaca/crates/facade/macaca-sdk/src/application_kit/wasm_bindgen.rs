//! WASM guest SDK bindgen planning helpers.
//!
//! This module is a provider-neutral toolchain facade for developers.  It
//! turns checked WIT metadata into a binding plan, a Rust guest scaffold, mock
//! host-import registrations, and a local contract report.  It never compiles
//! WASM, opens component bytes, constructs runtime-host providers, or performs
//! host calls.  Concrete code generators can be added later through the
//! `WasmBindgenBackend` trait without changing SDK callers.

use macaca_proto::{
    ApplicationExport, ApplicationImport, ApplicationRuntimeProfile, PackageRuntimeKind,
    MACACA_APPLICATION_WIT,
};

use crate::application_kit::{
    WasmComponentApplicationDescriptor, WasmComponentApplicationScaffold,
};
use crate::application_testkit::{ApplicationContractReport, ApplicationContractTestKit};

const IMPORT_MARKER: &str = "canonical-import:";
const EXPORT_MARKER: &str = "canonical-export:";

/// Safe diagnostic emitted by bindgen planning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasmBindgenDiagnostic {
    pub code: String,
    pub subject: String,
    pub message: String,
}

impl WasmBindgenDiagnostic {
    fn new(
        code: impl Into<String>,
        subject: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            subject: subject.into(),
            message: message.into(),
        }
    }
}

/// Input supplied to the bindgen backend.
///
/// The WIT source is accepted as text so SDK tests can validate local schemas
/// without a filesystem dependency.  The generated output reports only
/// canonical labels and sanitized diagnostics; callers should avoid logging the
/// full WIT body in production telemetry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasmBindgenInput {
    pub package_id: String,
    pub developer_id: String,
    pub name: String,
    pub version: String,
    pub component_entry: String,
    pub wit_source: String,
}

impl WasmBindgenInput {
    /// Create input with the canonical Macaca Application WIT schema.
    pub fn canonical(
        package_id: impl Into<String>,
        developer_id: impl Into<String>,
        name: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            package_id: package_id.into().trim().to_string(),
            developer_id: developer_id.into().trim().to_string(),
            name: name.into().trim().to_string(),
            version: version.into().trim().to_string(),
            component_entry: "component.wasm".into(),
            wit_source: MACACA_APPLICATION_WIT.into(),
        }
    }

    /// Override the component entry recorded in the generated fixture.
    pub fn component_entry(mut self, component_entry: impl Into<String>) -> Self {
        self.component_entry = component_entry.into().trim().to_string();
        self
    }

    /// Override WIT text for drift and validation tests.
    pub fn wit_source(mut self, wit_source: impl Into<String>) -> Self {
        self.wit_source = wit_source.into();
        self
    }
}

/// Mock host-import registration emitted by bindgen.
///
/// The registration is data-only.  It describes the host ABI import, its mock
/// permission label, and the trace requirement so test harnesses can register
/// deterministic mocks without importing runtime-host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasmMockHostImportBinding {
    pub import_name: String,
    pub permission: String,
    pub trace_required: bool,
}

/// Provider-neutral binding plan produced from WIT markers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasmGuestBindingPlan {
    pub package_id: String,
    pub runtime: ApplicationRuntimeProfile,
    pub exports: Vec<String>,
    pub imports: Vec<String>,
    pub mock_host_imports: Vec<WasmMockHostImportBinding>,
    pub diagnostics: Vec<WasmBindgenDiagnostic>,
}

/// Full bindgen output for SDK workflows.
#[derive(Debug, Clone, PartialEq)]
pub struct WasmBindgenOutput {
    pub descriptor: WasmComponentApplicationDescriptor,
    pub binding_plan: WasmGuestBindingPlan,
    pub rust_source: String,
    pub local_contract_report: ApplicationContractReport,
    pub diagnostics: Vec<WasmBindgenDiagnostic>,
}

impl WasmBindgenOutput {
    /// Return whether bindgen produced a contract-valid scaffold.
    pub fn is_success(&self) -> bool {
        self.diagnostics.is_empty() && self.local_contract_report.is_success()
    }
}

/// Extensible backend boundary for guest SDK generation.
pub trait WasmBindgenBackend {
    /// Generate one provider-neutral output bundle from bindgen input.
    fn generate(&self, input: WasmBindgenInput) -> WasmBindgenOutput;
}

/// Default Rust guest scaffold generator.
///
/// The backend currently emits deterministic source text and fixture metadata.
/// This is enough for SDK examples, package fixtures, and certification tests.
/// A later production backend can shell out to a real WIT bindgen or component
/// toolchain behind this trait while preserving the same audited output shape.
#[derive(Debug, Clone, Copy, Default)]
pub struct RustWasmBindgenBackend;

impl WasmBindgenBackend for RustWasmBindgenBackend {
    fn generate(&self, input: WasmBindgenInput) -> WasmBindgenOutput {
        tracing::info!(
            package_id = %input.package_id,
            component_entry = %input.component_entry,
            "WASM bindgen planning started"
        );
        let mut diagnostics = validate_input(&input);
        let exports = marker_values(&input.wit_source, EXPORT_MARKER);
        let imports = marker_values(&input.wit_source, IMPORT_MARKER);
        validate_canonical_exports(&exports, &mut diagnostics);
        validate_canonical_imports(&imports, &mut diagnostics);

        let descriptor = WasmComponentApplicationScaffold::new(
            input.package_id.clone(),
            input.developer_id.clone(),
            input.name.clone(),
            input.version.clone(),
        )
        .component_entry(input.component_entry.clone())
        .build();
        let mock_host_imports = imports
            .iter()
            .map(|import| WasmMockHostImportBinding {
                import_name: import.clone(),
                permission: permission_for_import(import),
                trace_required: true,
            })
            .collect::<Vec<_>>();
        let binding_plan = WasmGuestBindingPlan {
            package_id: input.package_id.clone(),
            runtime: ApplicationRuntimeProfile::new(PackageRuntimeKind::WasmComponent, "0"),
            exports,
            imports,
            mock_host_imports,
            diagnostics: diagnostics.clone(),
        };
        let rust_source = render_rust_scaffold(&input, &binding_plan);
        let testkit = ApplicationContractTestKit;
        let local_contract_report = merge_reports(
            testkit.validate_manifest(&descriptor.manifest),
            testkit.validate_abi_declaration(&descriptor.abi),
        );
        tracing::info!(
            package_id = %input.package_id,
            export_count = binding_plan.exports.len(),
            import_count = binding_plan.imports.len(),
            diagnostic_count = diagnostics.len(),
            "WASM bindgen planning completed"
        );
        WasmBindgenOutput {
            descriptor,
            binding_plan,
            rust_source,
            local_contract_report,
            diagnostics,
        }
    }
}

/// Generate a Rust scaffold using the default backend.
pub fn generate_wasm_guest_bindings(input: WasmBindgenInput) -> WasmBindgenOutput {
    RustWasmBindgenBackend.generate(input)
}

fn validate_input(input: &WasmBindgenInput) -> Vec<WasmBindgenDiagnostic> {
    let mut diagnostics = Vec::new();
    for (field, value) in [
        ("package_id", input.package_id.as_str()),
        ("developer_id", input.developer_id.as_str()),
        ("name", input.name.as_str()),
        ("version", input.version.as_str()),
        ("component_entry", input.component_entry.as_str()),
    ] {
        if value.trim().is_empty() {
            diagnostics.push(WasmBindgenDiagnostic::new(
                "missing_bindgen_input",
                field,
                "WASM bindgen input field is required",
            ));
        }
    }
    if marker_values(&input.wit_source, EXPORT_MARKER).is_empty() {
        diagnostics.push(WasmBindgenDiagnostic::new(
            "missing_wit_exports",
            "wit",
            "WIT source must declare canonical exports",
        ));
    }
    if marker_values(&input.wit_source, IMPORT_MARKER).is_empty() {
        diagnostics.push(WasmBindgenDiagnostic::new(
            "missing_wit_imports",
            "wit",
            "WIT source must declare canonical imports",
        ));
    }
    diagnostics
}

fn validate_canonical_exports(exports: &[String], diagnostics: &mut Vec<WasmBindgenDiagnostic>) {
    for export in ApplicationExport::required_v0() {
        if !exports.iter().any(|value| value == export.as_name()) {
            diagnostics.push(WasmBindgenDiagnostic::new(
                "missing_canonical_export",
                export.as_name(),
                "WIT source is missing a required Application ABI export",
            ));
        }
    }
}

fn validate_canonical_imports(imports: &[String], diagnostics: &mut Vec<WasmBindgenDiagnostic>) {
    for import in ApplicationImport::v0() {
        if !imports.iter().any(|value| value == import.as_name()) {
            diagnostics.push(WasmBindgenDiagnostic::new(
                "missing_canonical_import",
                import.as_name(),
                "WIT source is missing a required Application ABI import",
            ));
        }
    }
}

fn marker_values(schema: &str, marker: &str) -> Vec<String> {
    schema
        .lines()
        .filter_map(|line| line.split_once(marker).map(|(_, value)| value.trim()))
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

fn permission_for_import(import: &str) -> String {
    format!(
        "host.import.{}",
        import
            .trim()
            .replace("macaca:", "")
            .replace(['/', ':'], ".")
            .replace('_', ".")
    )
}

fn render_rust_scaffold(input: &WasmBindgenInput, plan: &WasmGuestBindingPlan) -> String {
    let exports = plan
        .exports
        .iter()
        .map(|export| format!("// export: {export}\n"))
        .collect::<String>();
    let imports = plan
        .mock_host_imports
        .iter()
        .map(|import| {
            format!(
                "// import: {} permission={} trace_required={}\n",
                import.import_name, import.permission, import.trace_required
            )
        })
        .collect::<String>();
    format!(
        "//! Generated Macaca WASM guest scaffold.\n\
         //! This source is provider-neutral and contains no runtime-host dependency.\n\
         \n\
         pub const PACKAGE_ID: &str = \"{}\";\n\
         pub const COMPONENT_ENTRY: &str = \"{}\";\n\
         \n\
         {}\
         {}\
         pub fn guest_entrypoint(trace_id: &str) -> &'static str {{\n\
         \tlet _ = trace_id;\n\
         \t\"macaca_guest_scaffold_ready\"\n\
         }}\n",
        escape_rust_literal(&input.package_id),
        escape_rust_literal(&input.component_entry),
        exports,
        imports
    )
}

fn escape_rust_literal(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn merge_reports(
    mut left: ApplicationContractReport,
    right: ApplicationContractReport,
) -> ApplicationContractReport {
    left.diagnostics.extend(right.diagnostics);
    left
}

#[cfg(test)]
#[path = "wasm_bindgen_tests.rs"]
mod wasm_bindgen_tests;
