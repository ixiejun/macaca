//! Memento-style report types and adapter state for YAML → Manifest v1 projection.
//!
//! These types capture **safe, auditable facts** about the conversion without
//! serializing raw manifest bodies, prompt templates, secrets, or environment
//! values into logs or Manifest v1 metadata.

use macaca_proto::ApplicationManifestV1;
use macaca_proto::AgentConfig;

use crate::model::AppManifest;

/// Safe diagnostic emitted while projecting YAML data into Manifest v1.
///
/// Each diagnostic records a machine-readable `code`, a stable `subject` anchor
/// (typically the application id), and a human-readable `message` that avoids
/// leaking sensitive manifest content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlProjectionDiagnostic {
    pub code: String,
    pub subject: String,
    pub message: String,
}

impl YamlProjectionDiagnostic {
    /// Construct a diagnostic with normalized string ownership.
    pub(super) fn new(
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

/// Conversion report that records safe facts about the YAML projection.
///
/// The report acts as a **Memento**: callers can persist or trace projection
/// decisions later without re-parsing the original YAML input.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct YamlToApplicationManifestV1Report {
    pub application_id: String,
    pub package_id: String,
    pub ability_count: usize,
    pub inferred_defaults: Vec<YamlProjectionDiagnostic>,
    pub compatibility_warnings: Vec<YamlProjectionDiagnostic>,
    pub legacy_only_fields: Vec<YamlProjectionDiagnostic>,
}

impl YamlToApplicationManifestV1Report {
    /// Record an inferred default that legacy runtime will apply implicitly.
    pub(super) fn push_default(&mut self, subject: impl Into<String>, message: impl Into<String>) {
        self.inferred_defaults.push(YamlProjectionDiagnostic::new(
            "inferred_default",
            subject,
            message,
        ));
    }

    /// Record a YAML-only field that remains owned by legacy compatibility.
    pub(super) fn push_legacy_only(&mut self, subject: impl Into<String>, message: impl Into<String>) {
        self.legacy_only_fields.push(YamlProjectionDiagnostic::new(
            "legacy_only_field",
            subject,
            message,
        ));
    }
}

/// Projection result used by package and ABI adapters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyAppManifestProjection {
    pub manifest: ApplicationManifestV1,
    pub report: YamlToApplicationManifestV1Report,
}

/// Adapter that converts legacy YAML app manifests into Manifest v1.
///
/// The adapter owns the parsed legacy manifest plus any pre-resolved file-based
/// agents supplied by the caller.  Fields are `pub(super)` so sibling modules
/// (`projection`, `abilities`) can implement the Adapter pattern without
/// exposing internal state on the public API surface.
pub struct YamlApplicationManifestAdapter {
    pub(super) manifest: AppManifest,
    pub(super) resolved_agents: Vec<AgentConfig>,
}
