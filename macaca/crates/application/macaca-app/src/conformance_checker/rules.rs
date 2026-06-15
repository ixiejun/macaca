//! Conformance checker rule specifications.
//!
//! Each rule owns one ecosystem invariant.  The facade can reorder or extend
//! rules without changing callers, and tests can identify diagnostics through
//! stable rule-specific codes.

use macaca_proto::{PackageRuntimeKind, PackageType};

use super::{ConformanceDiagnostic, ConformanceSeverity, ConformanceVisitor};

/// Rule contract used by Specification-style checker steps.
pub(crate) trait ConformanceRule: Send + Sync {
    fn name(&self) -> &'static str;
    fn evaluate(&self, visitor: &mut ConformanceVisitor<'_>);
}

pub(crate) struct ManifestVersionRule;

impl ConformanceRule for ManifestVersionRule {
    fn name(&self) -> &'static str {
        "manifest_version"
    }

    fn evaluate(&self, visitor: &mut ConformanceVisitor<'_>) {
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
            visitor.diagnostic(ConformanceDiagnostic::new(
                "manifest.future_version",
                ConformanceSeverity::Warning,
                "metadata.package.manifest.version",
                format!("manifest version {version} is newer than this host understands"),
            ));
        } else {
            visitor.diagnostic(ConformanceDiagnostic::new(
                "manifest.unsupported_version",
                ConformanceSeverity::Error,
                "metadata.package.manifest.version",
                format!("manifest version {version} is not supported"),
            ));
        }
    }
}

pub(crate) struct RuntimeRule;

impl ConformanceRule for RuntimeRule {
    fn name(&self) -> &'static str {
        "runtime"
    }

    fn evaluate(&self, visitor: &mut ConformanceVisitor<'_>) {
        if visitor.descriptor.manifest.runtime.kind.is_none() {
            visitor.diagnostic(ConformanceDiagnostic::new(
                "runtime.missing_kind",
                ConformanceSeverity::Error,
                "runtime.kind",
                "package runtime kind is required",
            ));
        } else {
            visitor.trace(self.name(), "passed", "runtime kind is declared");
        }
    }
}

pub(crate) struct AbiRule;

impl ConformanceRule for AbiRule {
    fn name(&self) -> &'static str {
        "abi"
    }

    fn evaluate(&self, visitor: &mut ConformanceVisitor<'_>) {
        let abi = &visitor.descriptor.manifest.runtime.abi_version;
        if visitor.context.supported_abi_versions.contains(abi) {
            visitor.trace(self.name(), "passed", "ABI version is supported");
        } else if abi.parse::<u64>().ok().unwrap_or(0) > 1 {
            visitor.diagnostic(ConformanceDiagnostic::new(
                "abi.future_version",
                ConformanceSeverity::Warning,
                "runtime.abi_version",
                format!("ABI version {abi} requires forward-conformant handling"),
            ));
        } else {
            visitor.diagnostic(ConformanceDiagnostic::new(
                "abi.unsupported_version",
                ConformanceSeverity::Error,
                "runtime.abi_version",
                format!("ABI version {abi} is not supported"),
            ));
        }
    }
}

pub(crate) struct PackageTypeRule;

impl ConformanceRule for PackageTypeRule {
    fn name(&self) -> &'static str {
        "package_type"
    }

    fn evaluate(&self, visitor: &mut ConformanceVisitor<'_>) {
        match visitor.descriptor.manifest.package_type {
            PackageType::Custom(_) => visitor.diagnostic(ConformanceDiagnostic::new(
                "package_type.custom",
                ConformanceSeverity::Warning,
                "package_type",
                "custom package type is preserved as structured metadata",
            )),
            _ => visitor.trace(self.name(), "passed", "package type is known"),
        }
    }
}

pub(crate) struct PermissionRule;

impl ConformanceRule for PermissionRule {
    fn name(&self) -> &'static str {
        "permission"
    }

    fn evaluate(&self, visitor: &mut ConformanceVisitor<'_>) {
        for (index, permission) in visitor.descriptor.manifest.permissions.iter().enumerate() {
            if permission.name.trim().is_empty() || permission.reason.trim().is_empty() {
                visitor.diagnostic(ConformanceDiagnostic::new(
                    "permission.invalid",
                    ConformanceSeverity::Error,
                    format!("permissions[{index}]"),
                    "permission name and reason must be present",
                ));
            }
        }
        visitor.trace(self.name(), "passed", "permission metadata checked");
    }
}

pub(crate) struct ServiceRule;

impl ConformanceRule for ServiceRule {
    fn name(&self) -> &'static str {
        "required_service"
    }

    fn evaluate(&self, visitor: &mut ConformanceVisitor<'_>) {
        let requirements = visitor.descriptor.manifest.required_services.clone();
        for requirement in requirements {
            if !visitor
                .context
                .available_services
                .contains(&requirement.service)
            {
                visitor.diagnostic(ConformanceDiagnostic::new(
                    "service.required_missing",
                    ConformanceSeverity::Error,
                    "required_services",
                    format!("required service {} is unavailable", requirement.service),
                ));
            }
        }
        visitor.trace(self.name(), "passed", "required services checked");
    }
}

pub(crate) struct OptionalModuleRule;

impl ConformanceRule for OptionalModuleRule {
    fn name(&self) -> &'static str {
        "optional_module"
    }

    fn evaluate(&self, visitor: &mut ConformanceVisitor<'_>) {
        let requirements = visitor.descriptor.manifest.optional_services.clone();
        for requirement in requirements {
            if !visitor
                .context
                .available_optional_modules
                .contains(&requirement.service)
            {
                visitor.diagnostic(ConformanceDiagnostic::new(
                    "optional_module.unavailable",
                    ConformanceSeverity::Warning,
                    "optional_services",
                    format!("optional service {} is unavailable", requirement.service),
                ));
            }
        }
        visitor.trace(self.name(), "passed", "optional modules checked");
    }
}

pub(crate) struct CommerceRule;

impl ConformanceRule for CommerceRule {
    fn name(&self) -> &'static str {
        "commerce"
    }

    fn evaluate(&self, visitor: &mut ConformanceVisitor<'_>) {
        let commerce = &visitor.descriptor.manifest.commerce;
        if commerce.store_required || commerce.license_type.is_paid_family() {
            if visitor.context.entitlement_states.contains("valid") {
                visitor.trace(self.name(), "passed", "paid package entitlement is valid");
            } else {
                visitor.diagnostic(ConformanceDiagnostic::new(
                    "commerce.entitlement_missing",
                    ConformanceSeverity::Warning,
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

impl ConformanceRule for TraceRule {
    fn name(&self) -> &'static str {
        "trace"
    }

    fn evaluate(&self, visitor: &mut ConformanceVisitor<'_>) {
        match visitor
            .descriptor
            .manifest
            .metadata
            .get("trace.required")
            .map(String::as_str)
        {
            Some("true") => visitor.trace(self.name(), "passed", "trace metadata is declared"),
            _ => visitor.diagnostic(ConformanceDiagnostic::new(
                "trace.missing",
                ConformanceSeverity::Warning,
                "metadata.trace.required",
                "ecosystem packages should declare trace requirements",
            )),
        }
    }
}

pub(crate) struct UpgradeRule;

impl ConformanceRule for UpgradeRule {
    fn name(&self) -> &'static str {
        "upgrade"
    }

    fn evaluate(&self, visitor: &mut ConformanceVisitor<'_>) {
        if let Some(min_os) = &visitor.descriptor.manifest.host_requirements.min_os_version {
            if min_os > &visitor.context.os_version {
                visitor.diagnostic(ConformanceDiagnostic::new(
                    "upgrade.os_too_old",
                    ConformanceSeverity::Error,
                    "manifest.host.min_os_version",
                    format!("package requires OS {min_os}"),
                ));
            } else {
                visitor.upgrade_note(format!(
                    "minimum OS {min_os} is conformant with host {}",
                    visitor.context.os_version
                ));
            }
        }
        if matches!(
            visitor.descriptor.manifest.runtime.kind,
            Some(PackageRuntimeKind::Custom(_))
        ) {
            visitor.diagnostic(ConformanceDiagnostic::new(
                "runtime.custom_kind",
                ConformanceSeverity::Warning,
                "runtime.kind",
                "custom runtime kind is preserved for future hosts",
            ));
        }
        visitor.trace(self.name(), "passed", "upgrade conformance checked");
    }
}
