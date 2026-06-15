//! Admission chain for Plugin Control Plane installs.
//!
//! The chain uses the Chain of Responsibility pattern: each check validates one
//! security or runtime-version concern and returns a structured decision.  This
//! keeps policy growth auditable and avoids a single install function becoming
//! a long sequence of unrelated conditionals.

use async_trait::async_trait;
use macaca_proto::{PluginError, PluginInstallSourceKind, PluginManifest, PluginPackageLocation};
use tracing::{info, warn};

/// Immutable facts evaluated by admission checks.
pub struct AdmissionContext<'a> {
    pub manifest: &'a PluginManifest,
    pub location: &'a PluginPackageLocation,
    pub project_local_enabled: bool,
}

/// Structured result for one admission check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionDecision {
    pub check: &'static str,
    pub accepted: bool,
    pub reason: Option<String>,
}

impl AdmissionDecision {
    /// Build an accepted decision for one check name.
    pub fn accepted(check: &'static str) -> Self {
        Self {
            check,
            accepted: true,
            reason: None,
        }
    }

    /// Build a rejected decision with a human-readable reason.
    pub fn rejected(check: &'static str, reason: impl Into<String>) -> Self {
        Self {
            check,
            accepted: false,
            reason: Some(reason.into()),
        }
    }
}

/// Replaceable admission policy check.
#[async_trait]
pub trait AdmissionCheck: Send + Sync {
    /// Evaluate one policy concern without mutating repository or registry
    /// state.  Checks must fail closed and must not read secret values.
    async fn evaluate(&self, context: &AdmissionContext<'_>) -> AdmissionDecision;
}

/// Validates basic manifest identity and version shape.
#[derive(Debug, Default)]
pub struct ManifestShapeAdmissionCheck;

#[async_trait]
impl AdmissionCheck for ManifestShapeAdmissionCheck {
    async fn evaluate(&self, context: &AdmissionContext<'_>) -> AdmissionDecision {
        if context.manifest.plugin_id.as_str().is_empty() {
            return AdmissionDecision::rejected("manifest_shape", "missing plugin id");
        }
        if context.manifest.version.as_str().is_empty() {
            return AdmissionDecision::rejected("manifest_shape", "missing plugin version");
        }
        if context.manifest.developer_id.as_str().is_empty() {
            return AdmissionDecision::rejected("manifest_shape", "missing developer id");
        }
        AdmissionDecision::accepted("manifest_shape")
    }
}

/// Validates runtime version support before runtime registration is attempted.
#[derive(Debug, Default)]
pub struct RuntimeVersionAdmissionCheck;

#[async_trait]
impl AdmissionCheck for RuntimeVersionAdmissionCheck {
    async fn evaluate(&self, context: &AdmissionContext<'_>) -> AdmissionDecision {
        if context.manifest.runtime.kind.is_supported_v0() {
            AdmissionDecision::accepted("runtime_version")
        } else {
            AdmissionDecision::rejected(
                "runtime_version",
                format!("unsupported runtime {}", context.manifest.runtime.kind),
            )
        }
    }
}

/// Validates source policy for local and future remote sources.
#[derive(Debug, Default)]
pub struct SourcePolicyAdmissionCheck;

#[async_trait]
impl AdmissionCheck for SourcePolicyAdmissionCheck {
    async fn evaluate(&self, context: &AdmissionContext<'_>) -> AdmissionDecision {
        if context.location.uri.trim().is_empty() {
            return AdmissionDecision::rejected("source_policy", "empty package location");
        }
        if context.location.uri.contains("..") {
            return AdmissionDecision::rejected("source_policy", "path traversal is not allowed");
        }
        if matches!(
            context.location.source_kind,
            PluginInstallSourceKind::ProjectLocal
        ) && !context.project_local_enabled
        {
            return AdmissionDecision::rejected(
                "source_policy",
                "project-local plugin installs are disabled by policy",
            );
        }
        AdmissionDecision::accepted("source_policy")
    }
}

/// Ordered admission chain used by the control-plane service.
pub struct PluginAdmissionChain {
    checks: Vec<Box<dyn AdmissionCheck>>,
}

impl Default for PluginAdmissionChain {
    fn default() -> Self {
        Self {
            checks: vec![
                Box::<ManifestShapeAdmissionCheck>::default(),
                Box::<RuntimeVersionAdmissionCheck>::default(),
                Box::<SourcePolicyAdmissionCheck>::default(),
            ],
        }
    }
}

impl PluginAdmissionChain {
    /// Build the default fail-closed chain.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a custom admission strategy after the built-in checks.
    pub fn with_check(mut self, check: Box<dyn AdmissionCheck>) -> Self {
        self.checks.push(check);
        self
    }

    /// Run every check in order and stop at the first rejection.
    pub async fn authorize(&self, context: &AdmissionContext<'_>) -> Result<(), PluginError> {
        for check in &self.checks {
            let decision = check.evaluate(context).await;
            if decision.accepted {
                info!(
                    check = decision.check,
                    plugin_id = %context.manifest.plugin_id,
                    "plugin admission check accepted"
                );
                continue;
            }
            let reason = decision
                .reason
                .unwrap_or_else(|| "plugin admission rejected".into());
            warn!(
                check = decision.check,
                plugin_id = %context.manifest.plugin_id,
                reason = %reason,
                "plugin admission check rejected"
            );
            return Err(PluginError::Unavailable(reason));
        }
        Ok(())
    }
}
