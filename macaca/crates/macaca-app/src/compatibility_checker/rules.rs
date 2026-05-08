//! Compatibility checker rule specifications.
//!
//! Each rule owns one ecosystem invariant.  The facade can reorder or extend
//! rules without changing callers, and tests can identify diagnostics through
//! stable rule-specific codes.

use macaca_proto::{PackageRuntimeKind, PackageType};

use super::{CompatibilityDiagnostic, CompatibilitySeverity, CompatibilityVisitor};

/// Rule contract used by Specification-style checker steps.
pub(crate) trait CompatibilityRule: Send + Sync {
    fn name(&self) -> &'static str;
    fn evaluate(&self, visitor: &mut CompatibilityVisitor<'_>);
}

pub(crate) struct ManifestVersionRule;

impl CompatibilityRule for ManifestVersionRule {
    fn name(&self) -> &'static str {
        "manifest_version"
    }

    fn evaluate(&self, visitor: &mut CompatibilityVisitor<'_>) {
        let version = visitor
            .descriptor
            .manifest
            .metadata
            .get("package.manifest.version")
            .cloned()
            .unwrap_or_else(|| "1".into());
        if visitor
            .context
            .supported_manifest_versions
            .contains(&version)
        {
            visitor.trace(self.name(), "passed", "manifest version is supported");
        } else if version.parse::<u64>().ok().unwrap_or(0) > 1 {
            visitor.diagnostic(CompatibilityDiagnostic::new(
                "manifest.future_version",
                CompatibilitySeverity::Warning,
                "metadata.package.manifest.version",
                format!("manifest version {version} is newer than this host understands"),
            ));
        } else {
            visitor.diagnostic(CompatibilityDiagnostic::new(
                "manifest.unsupported_version",
                CompatibilitySeverity::Error,
                "metadata.package.manifest.version",
                format!("manifest version {version} is not supported"),
            ));
        }
    }
}

pub(crate) struct RuntimeRule;

impl CompatibilityRule for RuntimeRule {
    fn name(&self) -> &'static str {
        "runtime"
    }

    fn evaluate(&self, visitor: &mut CompatibilityVisitor<'_>) {
        if visitor.descriptor.manifest.runtime.kind.is_none() {
            visitor.diagnostic(CompatibilityDiagnostic::new(
                "runtime.missing_kind",
                CompatibilitySeverity::Error,
                "runtime.kind",
                "package runtime kind is required",
            ));
        } else {
            visitor.trace(self.name(), "passed", "runtime kind is declared");
        }
    }
}

pub(crate) struct AbiRule;

impl CompatibilityRule for AbiRule {
    fn name(&self) -> &'static str {
        "abi"
    }

    fn evaluate(&self, visitor: &mut CompatibilityVisitor<'_>) {
        let abi = &visitor.descriptor.manifest.runtime.abi_version;
        if visitor.context.supported_abi_versions.contains(abi) {
            visitor.trace(self.name(), "passed", "ABI version is supported");
        } else if abi.parse::<u64>().ok().unwrap_or(0) > 1 {
            visitor.diagnostic(CompatibilityDiagnostic::new(
                "abi.future_version",
                CompatibilitySeverity::Warning,
                "runtime.abi_version",
                format!("ABI version {abi} requires forward-compatible handling"),
            ));
        } else {
            visitor.diagnostic(CompatibilityDiagnostic::new(
                "abi.unsupported_version",
                CompatibilitySeverity::Error,
                "runtime.abi_version",
                format!("ABI version {abi} is not supported"),
            ));
        }
    }
}

pub(crate) struct PackageTypeRule;

impl CompatibilityRule for PackageTypeRule {
    fn name(&self) -> &'static str {
        "package_type"
    }

    fn evaluate(&self, visitor: &mut CompatibilityVisitor<'_>) {
        match visitor.descriptor.manifest.package_type {
            PackageType::Custom(_) => visitor.diagnostic(CompatibilityDiagnostic::new(
                "package_type.custom",
                CompatibilitySeverity::Warning,
                "package_type",
                "custom package type is preserved as structured metadata",
            )),
            _ => visitor.trace(self.name(), "passed", "package type is known"),
        }
    }
}

pub(crate) struct PermissionRule;

impl CompatibilityRule for PermissionRule {
    fn name(&self) -> &'static str {
        "permission"
    }

    fn evaluate(&self, visitor: &mut CompatibilityVisitor<'_>) {
        for (index, permission) in visitor.descriptor.manifest.permissions.iter().enumerate() {
            if permission.name.trim().is_empty() || permission.reason.trim().is_empty() {
                visitor.diagnostic(CompatibilityDiagnostic::new(
                    "permission.invalid",
                    CompatibilitySeverity::Error,
                    format!("permissions[{index}]"),
                    "permission name and reason must be present",
                ));
            }
        }
        visitor.trace(self.name(), "passed", "permission metadata checked");
    }
}

pub(crate) struct ServiceRule;

impl CompatibilityRule for ServiceRule {
    fn name(&self) -> &'static str {
        "required_service"
    }

    fn evaluate(&self, visitor: &mut CompatibilityVisitor<'_>) {
        let requirements = visitor.descriptor.manifest.required_services.clone();
        for requirement in requirements {
            if !visitor
                .context
                .available_services
                .contains(&requirement.service)
            {
                visitor.diagnostic(CompatibilityDiagnostic::new(
                    "service.required_missing",
                    CompatibilitySeverity::Error,
                    "required_services",
                    format!("required service {} is unavailable", requirement.service),
                ));
            }
        }
        visitor.trace(self.name(), "passed", "required services checked");
    }
}

pub(crate) struct OptionalModuleRule;

impl CompatibilityRule for OptionalModuleRule {
    fn name(&self) -> &'static str {
        "optional_module"
    }

    fn evaluate(&self, visitor: &mut CompatibilityVisitor<'_>) {
        let requirements = visitor.descriptor.manifest.optional_services.clone();
        for requirement in requirements {
            if !visitor
                .context
                .available_optional_modules
                .contains(&requirement.service)
            {
                visitor.diagnostic(CompatibilityDiagnostic::new(
                    "optional_module.unavailable",
                    CompatibilitySeverity::Warning,
                    "optional_services",
                    format!("optional service {} is unavailable", requirement.service),
                ));
            }
        }
        visitor.trace(self.name(), "passed", "optional modules checked");
    }
}

pub(crate) struct CommerceRule;

impl CompatibilityRule for CommerceRule {
    fn name(&self) -> &'static str {
        "commerce"
    }

    fn evaluate(&self, visitor: &mut CompatibilityVisitor<'_>) {
        let commerce = &visitor.descriptor.manifest.commerce;
        if commerce.store_required || commerce.license_type.is_paid_family() {
            if visitor.context.entitlement_states.contains("valid") {
                visitor.trace(self.name(), "passed", "paid package entitlement is valid");
            } else {
                visitor.diagnostic(CompatibilityDiagnostic::new(
                    "commerce.entitlement_missing",
                    CompatibilitySeverity::Warning,
                    "commerce.entitlement_id",
                    "paid package requires entitlement before runtime start",
                ));
            }
        } else {
            visitor.trace(self.name(), "passed", "package is free or open");
        }
    }
}

pub(crate) struct TraceRule;

impl CompatibilityRule for TraceRule {
    fn name(&self) -> &'static str {
        "trace"
    }

    fn evaluate(&self, visitor: &mut CompatibilityVisitor<'_>) {
        match visitor
            .descriptor
            .manifest
            .metadata
            .get("trace.required")
            .map(String::as_str)
        {
            Some("true") => visitor.trace(self.name(), "passed", "trace metadata is declared"),
            _ => visitor.diagnostic(CompatibilityDiagnostic::new(
                "trace.missing",
                CompatibilitySeverity::Warning,
                "metadata.trace.required",
                "ecosystem packages should declare trace requirements",
            )),
        }
    }
}

pub(crate) struct UpgradeRule;

impl CompatibilityRule for UpgradeRule {
    fn name(&self) -> &'static str {
        "upgrade"
    }

    fn evaluate(&self, visitor: &mut CompatibilityVisitor<'_>) {
        if let Some(min_os) = &visitor.descriptor.manifest.compatibility.min_os_version {
            if min_os > &visitor.context.os_version {
                visitor.diagnostic(CompatibilityDiagnostic::new(
                    "upgrade.os_too_old",
                    CompatibilitySeverity::Error,
                    "compatibility.min_os_version",
                    format!("package requires OS {min_os}"),
                ));
            } else {
                visitor.upgrade_note(format!(
                    "minimum OS {min_os} is compatible with host {}",
                    visitor.context.os_version
                ));
            }
        }
        if matches!(
            visitor.descriptor.manifest.runtime.kind,
            Some(PackageRuntimeKind::Custom(_))
        ) {
            visitor.diagnostic(CompatibilityDiagnostic::new(
                "runtime.custom_kind",
                CompatibilitySeverity::Warning,
                "runtime.kind",
                "custom runtime kind is preserved for future hosts",
            ));
        }
        visitor.trace(self.name(), "passed", "upgrade compatibility checked");
    }
}
