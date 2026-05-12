//! Data contracts for Application Platform certification.
//!
//! These types are deliberately data-only.  They can be serialized, logged in
//! redacted form, and consumed by future Store tooling without constructing a
//! runtime, kernel, service runtime, plugin host, or WASM engine.

use std::collections::BTreeSet;

use macaca_proto::{
    ApplicationAbiDeclaration, ApplicationManifestV1, KernelServiceId, PackageRuntimeKind,
};

/// Status for an application platform certification report.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplicationCertificationStatus {
    /// All enabled rules passed and the fixture is contract-ready.
    Passed,
    /// At least one fail-closed rule rejected the fixture before execution.
    Failed,
    /// The fixture is structurally valid but depends on an unavailable optional
    /// runtime or module.  This status is safe: callers can display the report
    /// without attempting execution.
    Unavailable,
}

/// Safe diagnostic emitted by a certification rule.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ApplicationCertificationDiagnostic {
    pub code: String,
    pub subject: String,
    pub message: String,
}

impl ApplicationCertificationDiagnostic {
    pub(crate) fn new(
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

/// Serializable, redacted certification report.
///
/// The report intentionally stores identifiers and reason codes only.  It does
/// not store raw manifest JSON/YAML, prompt bodies, raw agent config, WASM
/// bytes, secrets, environment values, API keys, private keys, raw signatures,
/// raw host payloads, or unbounded user input.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ApplicationCertificationReport {
    pub fixture_id: String,
    pub app_id: String,
    pub ability_ids: Vec<String>,
    pub service_ids: Vec<String>,
    pub capability_ids: Vec<String>,
    pub operation: String,
    pub status: ApplicationCertificationStatus,
    pub reason_codes: Vec<String>,
    pub trace_id: Option<String>,
    pub diagnostics: Vec<ApplicationCertificationDiagnostic>,
}

impl ApplicationCertificationReport {
    pub(crate) fn new(fixture_id: impl Into<String>, manifest: &ApplicationManifestV1) -> Self {
        Self {
            fixture_id: fixture_id.into(),
            app_id: manifest.package_id.to_string(),
            ability_ids: Vec::new(),
            service_ids: Vec::new(),
            capability_ids: Vec::new(),
            operation: "application.certify".into(),
            status: ApplicationCertificationStatus::Passed,
            reason_codes: Vec::new(),
            trace_id: None,
            diagnostics: Vec::new(),
        }
    }

    /// Report whether certification completed without fail-closed diagnostics.
    pub fn is_success(&self) -> bool {
        self.status == ApplicationCertificationStatus::Passed && self.diagnostics.is_empty()
    }

    pub(crate) fn fail(
        &mut self,
        code: impl Into<String>,
        subject: impl Into<String>,
        message: impl Into<String>,
    ) {
        let code = code.into();
        self.reason_codes.push(code.clone());
        self.status = ApplicationCertificationStatus::Failed;
        self.diagnostics
            .push(ApplicationCertificationDiagnostic::new(
                code, subject, message,
            ));
    }

    pub(crate) fn unavailable(
        &mut self,
        code: impl Into<String>,
        subject: impl Into<String>,
        message: impl Into<String>,
    ) {
        let code = code.into();
        self.reason_codes.push(code.clone());
        if self.status == ApplicationCertificationStatus::Passed {
            self.status = ApplicationCertificationStatus::Unavailable;
        }
        self.diagnostics
            .push(ApplicationCertificationDiagnostic::new(
                code, subject, message,
            ));
    }
}

/// Certification context supplied by the caller.
///
/// The context models currently available services, plugins, and runtimes as
/// identifiers.  Certification never opens a network connection, store session,
/// plugin host, or WASM runtime.  Missing required entries fail closed; missing
/// optional entries become structured unavailable diagnostics.
#[derive(Debug, Clone, Default)]
pub struct ApplicationCertificationContext {
    pub(crate) trace_id: Option<String>,
    pub(crate) available_services: BTreeSet<KernelServiceId>,
    pub(crate) available_plugins: BTreeSet<String>,
    pub(crate) available_runtimes: BTreeSet<PackageRuntimeKind>,
}

impl ApplicationCertificationContext {
    /// Create an empty context.  Empty means no concrete runtime or service is
    /// assumed to exist, which keeps certification fail-closed by default.
    pub fn new() -> Self {
        Self::default()
    }

    /// Attach trace context to the certification operation.
    pub fn trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    /// Declare one available service by id.
    pub fn service(mut self, service: KernelServiceId) -> Self {
        self.available_services.insert(service);
        self
    }

    /// Declare one available plugin by id.
    pub fn plugin(mut self, plugin_id: impl Into<String>) -> Self {
        self.available_plugins.insert(plugin_id.into());
        self
    }

    /// Declare one available runtime kind.
    pub fn runtime(mut self, runtime: PackageRuntimeKind) -> Self {
        self.available_runtimes.insert(runtime);
        self
    }
}

/// Data-only certification fixture consumed by the certification facade.
#[derive(Debug, Clone)]
pub struct ApplicationCertificationFixture {
    pub fixture_id: String,
    pub manifest: ApplicationManifestV1,
    pub abi: Option<ApplicationAbiDeclaration>,
}

impl ApplicationCertificationFixture {
    /// Build a fixture from Manifest v1 data and optional ABI metadata.
    pub fn new(fixture_id: impl Into<String>, manifest: ApplicationManifestV1) -> Self {
        Self {
            fixture_id: fixture_id.into(),
            manifest,
            abi: None,
        }
    }

    /// Attach ABI metadata for WASM or host-import certification checks.
    pub fn abi(mut self, abi: ApplicationAbiDeclaration) -> Self {
        self.abi = Some(abi);
        self
    }
}
