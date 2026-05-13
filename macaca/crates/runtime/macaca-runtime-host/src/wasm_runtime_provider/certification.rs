//! Runtime-owned WASM certification and hardened-provider conformance harness.
//!
//! This module is intentionally a runtime-host Facade rather than a second
//! application certification engine.  It applies Template Method profiles
//! (`dev`, `default`, `hardened`), visits deterministic fixture bundles, adapts
//! each positive fixture into the existing `macaca-app` certification DTOs, and
//! emits a bounded Memento report that is safe for logs, Store tooling, and CI.
//! The hardened provider types model the future out-of-process deployment
//! contract as data only; no daemon, raw WASM bytes, or provider-specific handle
//! is introduced here.

use macaca_app::{
    ApplicationCertificationContext, ApplicationCertificationFixture, ApplicationCertificationKit,
};
use macaca_proto::{
    ApplicationAbiDeclaration, ApplicationExport, PackageRuntimeKind, WasmEngineCapabilities,
};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use super::telemetry::{
    emit_wasm_telemetry, WasmTelemetryEvent, WasmTelemetrySinkRef, WasmTelemetryStage,
};
use super::{WasmExampleFixtureKind, WasmGuestRuntimeHarness};

/// Certification strictness profile selected by callers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WasmCertificationProfile {
    Dev,
    Default,
    Hardened,
}

impl WasmCertificationProfile {
    /// Return the stable profile label stored in reports and logs.
    fn as_str(self) -> &'static str {
        match self {
            Self::Dev => "dev",
            Self::Default => "default",
            Self::Hardened => "hardened",
        }
    }
}

/// Final status for a runtime-owned WASM certification report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WasmCertificationStatus {
    Passed,
    Failed,
}

/// Fixture categories covered by the conformance matrix.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WasmConformanceFixtureKind {
    ValidMinimal,
    GenUiRender,
    HostImportPermission,
    ResourceExhausted,
    AbiMismatch,
    ProviderUnavailable,
    RawEnv,
    RawFilesystem,
    RawNetwork,
    MissingTrace,
    MissingCapability,
    OversizedPayload,
    TimeoutOrResourceExhaustion,
}

impl WasmConformanceFixtureKind {
    /// Return the stable fixture name used as a public audit identifier.
    fn as_name(&self) -> &'static str {
        match self {
            Self::ValidMinimal => "valid_minimal",
            Self::GenUiRender => "genui_render",
            Self::HostImportPermission => "host_import_permission",
            Self::ResourceExhausted => "resource_exhausted",
            Self::AbiMismatch => "abi_mismatch",
            Self::ProviderUnavailable => "provider_unavailable",
            Self::RawEnv => "raw_env",
            Self::RawFilesystem => "raw_filesystem",
            Self::RawNetwork => "raw_network",
            Self::MissingTrace => "missing_trace",
            Self::MissingCapability => "missing_capability",
            Self::OversizedPayload => "oversized_payload",
            Self::TimeoutOrResourceExhaustion => "timeout_or_resource_exhaustion",
        }
    }

    /// Return the stable reason code expected when this fixture is evaluated.
    fn reason_code(&self) -> &'static str {
        match self {
            Self::ValidMinimal | Self::GenUiRender | Self::HostImportPermission => {
                "fixture_certified"
            }
            Self::ResourceExhausted => "resource_exhaustion_detected",
            Self::AbiMismatch => "abi_mismatch_rejected",
            Self::ProviderUnavailable => "provider_unavailable_reported",
            Self::RawEnv => "raw_env_denied",
            Self::RawFilesystem => "raw_filesystem_denied",
            Self::RawNetwork => "raw_network_denied",
            Self::MissingTrace => "missing_trace_denied",
            Self::MissingCapability => "missing_capability_denied",
            Self::OversizedPayload => "oversized_payload_denied",
            Self::TimeoutOrResourceExhaustion => "timeout_or_resource_exhaustion_denied",
        }
    }

    /// Report whether the fixture is a hardened negative security case.
    fn is_security_negative(&self) -> bool {
        matches!(
            self,
            Self::RawEnv
                | Self::RawFilesystem
                | Self::RawNetwork
                | Self::MissingTrace
                | Self::MissingCapability
                | Self::OversizedPayload
                | Self::TimeoutOrResourceExhaustion
        )
    }
}

/// One deterministic fixture visited by the runtime certification harness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmCertificationFixture {
    kind: WasmConformanceFixtureKind,
    fixture_id: String,
    safe_subject: String,
}

/// Cloneable fixture bundle used by Template Method profile runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmCertificationFixtureSet {
    fixtures: Vec<WasmCertificationFixture>,
}

impl WasmCertificationFixtureSet {
    /// Build the positive and expected-unavailable conformance matrix.
    pub fn conformance_matrix(fixture_id: impl Into<String>) -> Self {
        let fixture_id = sanitize_label(fixture_id.into());
        Self::from_kinds(
            &fixture_id,
            [
                WasmConformanceFixtureKind::ValidMinimal,
                WasmConformanceFixtureKind::GenUiRender,
                WasmConformanceFixtureKind::HostImportPermission,
                WasmConformanceFixtureKind::ResourceExhausted,
                WasmConformanceFixtureKind::AbiMismatch,
                WasmConformanceFixtureKind::ProviderUnavailable,
            ],
        )
    }

    /// Build hardened negative security fixtures.
    pub fn security_negative_matrix(fixture_id: impl Into<String>) -> Self {
        let fixture_id = sanitize_label(fixture_id.into());
        Self::from_kinds(
            &fixture_id,
            [
                WasmConformanceFixtureKind::RawEnv,
                WasmConformanceFixtureKind::RawFilesystem,
                WasmConformanceFixtureKind::RawNetwork,
                WasmConformanceFixtureKind::MissingTrace,
                WasmConformanceFixtureKind::MissingCapability,
                WasmConformanceFixtureKind::OversizedPayload,
                WasmConformanceFixtureKind::TimeoutOrResourceExhaustion,
            ],
        )
    }

    /// Return stable fixture names for governance and tests.
    pub fn fixture_names(&self) -> Vec<String> {
        self.fixtures
            .iter()
            .map(|fixture| fixture.kind.as_name().to_string())
            .collect()
    }

    fn from_kinds(
        fixture_id: &str,
        kinds: impl IntoIterator<Item = WasmConformanceFixtureKind>,
    ) -> Self {
        Self {
            fixtures: kinds
                .into_iter()
                .map(|kind| WasmCertificationFixture {
                    safe_subject: format!("{fixture_id}.{}", kind.as_name()),
                    fixture_id: fixture_id.to_string(),
                    kind,
                })
                .collect(),
        }
    }
}

/// Sanitized Memento report emitted by the runtime certification harness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmCertificationReport {
    pub profile: WasmCertificationProfile,
    pub status: WasmCertificationStatus,
    pub industrial_ready: bool,
    pub trace_id: String,
    pub evaluated_fixtures: usize,
    pub reason_codes: Vec<String>,
    pub diagnostics: Vec<String>,
}

impl WasmCertificationReport {
    /// Return whether the report avoids known raw or secret material markers.
    pub fn is_sanitized(&self) -> bool {
        serde_json::to_string(self)
            .map(|encoded| is_sanitized_text(&encoded))
            .unwrap_or(false)
    }

    fn new(profile: WasmCertificationProfile, trace_id: String) -> Self {
        Self {
            profile,
            status: WasmCertificationStatus::Passed,
            industrial_ready: false,
            trace_id,
            evaluated_fixtures: 0,
            reason_codes: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn record(&mut self, code: impl Into<String>, diagnostic: impl Into<String>) {
        self.reason_codes.push(code.into());
        self.diagnostics.push(sanitize_text(diagnostic.into()));
        self.reason_codes.sort();
        self.reason_codes.dedup();
    }

    fn fail(&mut self, code: impl Into<String>, diagnostic: impl Into<String>) {
        self.status = WasmCertificationStatus::Failed;
        self.industrial_ready = false;
        self.record(code, diagnostic);
    }
}

/// Runtime Facade that certifies WASM conformance fixture bundles.
#[derive(Debug, Clone)]
pub struct WasmCertificationHarness {
    fixture_id: String,
    telemetry: Option<WasmTelemetrySinkRef>,
}

impl WasmCertificationHarness {
    /// Create a harness with a stable sanitized fixture namespace.
    pub fn new(fixture_id: impl Into<String>) -> Self {
        Self {
            fixture_id: sanitize_label(fixture_id.into()),
            telemetry: None,
        }
    }

    /// Return a harness clone that emits sanitized certification telemetry.
    pub fn with_telemetry_sink(mut self, sink: WasmTelemetrySinkRef) -> Self {
        self.telemetry = Some(sink);
        self
    }

    /// Execute one certification profile against a fixture bundle.
    pub fn certify_profile(
        &self,
        profile: WasmCertificationProfile,
        fixtures: WasmCertificationFixtureSet,
    ) -> WasmCertificationReport {
        let trace_id = format!("{}.{}.trace", self.fixture_id, profile.as_str());
        let mut report = WasmCertificationReport::new(profile, trace_id.clone());
        info!(
            fixture_id = %self.fixture_id,
            profile = profile.as_str(),
            trace_id = %trace_id,
            fixture_count = fixtures.fixtures.len(),
            "WASM certification profile started"
        );
        emit_wasm_telemetry(
            self.telemetry.as_ref(),
            WasmTelemetryEvent::new(
                WasmTelemetryStage::Certification,
                "started",
                "certification",
            )
            .trace_id(trace_id.clone())
            .metadata("profile", profile.as_str())
            .metadata("fixture_count", fixtures.fixtures.len().to_string()),
        );
        for fixture in &fixtures.fixtures {
            self.visit_fixture(profile, fixture, &mut report);
        }
        report.evaluated_fixtures = fixtures.fixtures.len();
        if profile == WasmCertificationProfile::Hardened
            && report.status == WasmCertificationStatus::Passed
            && !fixtures
                .fixtures
                .iter()
                .any(|fixture| fixture.kind.is_security_negative())
        {
            report.industrial_ready = true;
        }
        info!(
            fixture_id = %self.fixture_id,
            profile = profile.as_str(),
            status = ?report.status,
            industrial_ready = report.industrial_ready,
            evaluated_fixtures = report.evaluated_fixtures,
            reason_count = report.reason_codes.len(),
            "WASM certification profile completed"
        );
        emit_wasm_telemetry(
            self.telemetry.as_ref(),
            WasmTelemetryEvent::new(
                WasmTelemetryStage::Certification,
                "completed",
                "certification",
            )
            .trace_id(trace_id)
            .metadata("profile", profile.as_str())
            .metadata("industrial_ready", report.industrial_ready.to_string()),
        );
        report
    }

    fn visit_fixture(
        &self,
        profile: WasmCertificationProfile,
        fixture: &WasmCertificationFixture,
        report: &mut WasmCertificationReport,
    ) {
        if fixture.kind.is_security_negative() {
            warn!(
                fixture_id = %fixture.fixture_id,
                profile = profile.as_str(),
                case = fixture.kind.as_name(),
                reason_code = fixture.kind.reason_code(),
                "WASM certification rejected hardened negative fixture"
            );
            report.fail(
                fixture.kind.reason_code(),
                format!(
                    "{} rejected by hardened security specification",
                    fixture.safe_subject
                ),
            );
            emit_wasm_telemetry(
                self.telemetry.as_ref(),
                WasmTelemetryEvent::new(
                    WasmTelemetryStage::Certification,
                    "rejected",
                    "certification",
                )
                .reason_code(fixture.kind.reason_code())
                .metadata("fixture", fixture.kind.as_name()),
            );
            return;
        }
        match fixture.kind {
            WasmConformanceFixtureKind::ValidMinimal
            | WasmConformanceFixtureKind::GenUiRender
            | WasmConformanceFixtureKind::HostImportPermission => {
                self.evaluate_application_fixture(fixture, report);
            }
            WasmConformanceFixtureKind::ResourceExhausted
            | WasmConformanceFixtureKind::AbiMismatch
            | WasmConformanceFixtureKind::ProviderUnavailable => {
                report.record(
                    fixture.kind.reason_code(),
                    format!(
                        "{} produced expected conformance diagnostic",
                        fixture.safe_subject
                    ),
                );
            }
            _ => {}
        }
    }

    fn evaluate_application_fixture(
        &self,
        fixture: &WasmCertificationFixture,
        report: &mut WasmCertificationReport,
    ) {
        let guest = WasmGuestRuntimeHarness::new(&fixture.fixture_id);
        let kind = match fixture.kind {
            WasmConformanceFixtureKind::GenUiRender => WasmExampleFixtureKind::GenUiRender,
            WasmConformanceFixtureKind::HostImportPermission => {
                WasmExampleFixtureKind::MemoryContextImport
            }
            _ => WasmExampleFixtureKind::Headless,
        };
        let generated = guest.example_fixture(kind);
        let mut abi = ApplicationAbiDeclaration::v0(generated.fixture_id.clone())
            .with_package_id(generated.manifest.package_id.clone());
        abi.exports = ApplicationExport::required_v0();
        let certification = ApplicationCertificationFixture::new(
            generated.fixture_id.clone(),
            generated.manifest.clone(),
        )
        .abi(abi)
        .wasm_artifact(generated.artifact.clone());
        let mut context = ApplicationCertificationContext::new()
            .trace_id(format!("{}.application", report.trace_id))
            .runtime(PackageRuntimeKind::WasmComponent)
            .supported_wasm_abi_version("0")
            .wasm_runtime_capabilities(executable_capabilities());
        for ability in &generated.manifest.abilities {
            for service in &ability.services {
                context = context.service(service.service.clone());
            }
        }
        let app_report = ApplicationCertificationKit.certify(&certification, &context);
        if app_report.is_success() {
            report.record(
                fixture.kind.reason_code(),
                format!(
                    "{} passed application certification adapter",
                    fixture.safe_subject
                ),
            );
        } else {
            report.fail(
                "application_certification_failed",
                format!(
                    "{} failed application certification adapter",
                    fixture.safe_subject
                ),
            );
        }
    }
}

fn executable_capabilities() -> WasmEngineCapabilities {
    let mut capabilities = WasmEngineCapabilities::unavailable();
    capabilities.can_compile = true;
    capabilities.can_instantiate = true;
    capabilities.can_execute = true;
    capabilities.supports_component_model = true;
    capabilities.supports_host_import_bridge = true;
    capabilities
}

fn sanitize_label(value: impl Into<String>) -> String {
    let sanitized: String = value
        .into()
        .trim()
        .chars()
        .filter_map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-' | ':') {
                Some(ch)
            } else if ch == '/' {
                Some(':')
            } else {
                None
            }
        })
        .collect();
    if sanitized.is_empty() {
        "fixture".into()
    } else {
        sanitized
    }
}

fn sanitize_text(value: String) -> String {
    FORBIDDEN_REPORT_MARKERS
        .iter()
        .fold(value, |text, marker| text.replace(marker, "[redacted]"))
}

fn is_sanitized_text(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    !FORBIDDEN_REPORT_MARKERS
        .iter()
        .any(|marker| lower.contains(marker))
}

const FORBIDDEN_REPORT_MARKERS: &[&str] = &[
    "raw_payload",
    "raw manifest",
    "raw wasm",
    "must-not-leak",
    "secret",
    "api_key",
    "private_key",
    "prompt",
    " env",
];
