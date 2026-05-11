//! Contract test utilities for Plugin SDK output.
//!
//! The test kit implements Specification-style checks over DTOs. It is safe to
//! run in plugin author tests because it performs no network, filesystem,
//! process, WASM, or runtime-host operations.

use std::collections::BTreeSet;

use macaca_proto::{PluginHostResult, PluginHostStatus, PluginManifest, PluginRuntimeKind};

use super::builders::PluginRegistration;

/// One structured contract diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginContractDiagnostic {
    pub code: String,
    pub message: String,
}

impl PluginContractDiagnostic {
    fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

/// Contract report returned by the test kit.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PluginContractReport {
    pub diagnostics: Vec<PluginContractDiagnostic>,
}

impl PluginContractReport {
    /// Return true when no contract diagnostics were produced.
    pub fn is_success(&self) -> bool {
        self.diagnostics.is_empty()
    }

    fn push(&mut self, code: impl Into<String>, message: impl Into<String>) {
        self.diagnostics
            .push(PluginContractDiagnostic::new(code, message));
    }
}

/// Stateless contract test kit for manifest, capability, hook, config, and host checks.
#[derive(Debug, Clone, Default)]
pub struct PluginContractTestKit;

impl PluginContractTestKit {
    /// Validate the composite SDK registration.
    pub fn validate_registration(&self, registration: &PluginRegistration) -> PluginContractReport {
        let mut report = self.validate_manifest(&registration.manifest);
        for capability in &registration.capabilities {
            if capability.plugin_id != registration.manifest.plugin_id {
                report.push(
                    "capability_plugin_mismatch",
                    "capability descriptor plugin id must match manifest plugin id",
                );
            }
            for hint in &capability.permission_hints {
                if hint.required
                    && !registration
                        .manifest
                        .permissions
                        .iter()
                        .any(|permission| permission.id == hint.permission)
                {
                    report.push(
                        "missing_capability_permission",
                        format!("missing permission declaration for {}", hint.permission),
                    );
                }
            }
        }
        for hook in &registration.hooks {
            if hook.plugin_id != registration.manifest.plugin_id {
                report.push(
                    "hook_plugin_mismatch",
                    "hook descriptor plugin id must match manifest plugin id",
                );
            }
        }
        for config in &registration.config_requirements {
            if config.required && config.key.is_empty() {
                report.push("invalid_config_requirement", "required config key is empty");
            }
        }
        for secret in &registration.secret_requirements {
            if secret.required && secret.name.is_empty() {
                report.push(
                    "invalid_secret_requirement",
                    "required secret name is empty",
                );
            }
        }
        report
    }

    /// Validate a manifest's identity, permission, resource, and ABI shape.
    pub fn validate_manifest(&self, manifest: &PluginManifest) -> PluginContractReport {
        let mut report = PluginContractReport::default();
        if manifest.plugin_id.as_str().is_empty() {
            report.push("missing_plugin_id", "plugin id must not be empty");
        }
        if manifest.version.as_str().is_empty() {
            report.push("missing_plugin_version", "plugin version must not be empty");
        }
        if manifest.developer_id.as_str().is_empty() {
            report.push("missing_developer_id", "developer id must not be empty");
        }
        if manifest.runtime.abi_version.trim().is_empty() {
            report.push(
                "missing_abi_version",
                "runtime ABI version must not be empty",
            );
        }
        self.validate_privileged_contracts(manifest, &mut report);
        report
    }

    /// Validate structured unavailable-safe behavior from host skeletons.
    pub fn validate_unavailable_result(&self, result: &PluginHostResult) -> PluginContractReport {
        let mut report = PluginContractReport::default();
        if result.status != PluginHostStatus::Unavailable {
            report.push(
                "expected_unavailable_status",
                "unavailable-safe host result must use unavailable status",
            );
        }
        if result.error_code.as_deref() != Some("plugin_host_unavailable") {
            report.push(
                "missing_unavailable_error_code",
                "unavailable-safe host result must include plugin_host_unavailable",
            );
        }
        if result.trace.trace_id.is_empty() {
            report.push("missing_trace", "host result must preserve trace context");
        }
        report
    }

    /// Validate that executable skeleton runtimes are declared but not treated as v0-supported.
    pub fn validate_boundary_compliance(&self, manifest: &PluginManifest) -> PluginContractReport {
        let mut report = PluginContractReport::default();
        if matches!(
            manifest.runtime.kind,
            PluginRuntimeKind::Wasm | PluginRuntimeKind::Process | PluginRuntimeKind::RemoteProxy
        ) && manifest.runtime.kind.is_supported_v0()
        {
            report.push(
                "unsafe_runtime_boundary",
                "execution skeleton runtimes must not be marked v0 executable",
            );
        }
        report
    }

    fn validate_privileged_contracts(
        &self,
        manifest: &PluginManifest,
        report: &mut PluginContractReport,
    ) {
        let declared_permissions: BTreeSet<_> =
            manifest.permissions.iter().map(|p| p.id.as_str()).collect();
        for service in &manifest.provides_services {
            for permission in service
                .required_permissions
                .iter()
                .chain(service.service.required_permissions.iter())
            {
                if !declared_permissions.contains(permission.as_str()) {
                    report.push(
                        "missing_permission_declaration",
                        format!("missing permission declaration for {permission}"),
                    );
                }
            }
        }
        if (!manifest.provides_services.is_empty() || !manifest.provides_capabilities.is_empty())
            && manifest.resources.is_empty()
        {
            report.push(
                "missing_resource_declaration",
                "privileged plugin manifests must declare at least one resource",
            );
        }
    }
}
