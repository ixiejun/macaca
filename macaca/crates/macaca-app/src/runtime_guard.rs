//! Runtime guard chain for package load decisions.
//!
//! The guard uses the Specification + Chain of Responsibility patterns.  Each
//! step validates one invariant and emits traceable decision data.  The guard
//! never executes package code; it only decides whether metadata can proceed
//! to a loader or whether activation must stop with structured errors.

use std::collections::BTreeSet;

use macaca_proto::{
    KernelServiceId, PackageDescriptor, PackageGuardDecision, PackageGuardError, PackageResult,
    PackageRuntimeKind, PackageServiceAvailability,
};
use tracing::{info, warn};

/// Trace event emitted by the package guard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageGuardTraceEvent {
    pub package_id: String,
    pub step: String,
    pub decision: String,
    pub reason: Option<String>,
}

/// Immutable input for one guard run.
#[derive(Debug, Clone)]
pub struct PackageGuardContext {
    pub supported_abi_version: String,
    pub available_services: BTreeSet<KernelServiceId>,
}

impl PackageGuardContext {
    /// Create a guard context for a host ABI and service inventory.
    pub fn new(supported_abi_version: impl Into<String>) -> Self {
        Self {
            supported_abi_version: supported_abi_version.into(),
            available_services: BTreeSet::new(),
        }
    }

    /// Add one available service to the context.
    pub fn with_service(mut self, service: KernelServiceId) -> Self {
        self.available_services.insert(service);
        self
    }
}

/// Guard step contract.
pub trait PackageGuardStep: Send + Sync {
    /// Human-readable step name used in trace and logs.
    fn name(&self) -> &'static str;

    /// Validate and possibly mutate descriptor diagnostics.
    fn evaluate(
        &self,
        descriptor: &mut PackageDescriptor,
        context: &PackageGuardContext,
        trace: &mut Vec<PackageGuardTraceEvent>,
    ) -> PackageResult<()>;
}

/// Default ordered package runtime guard.
pub struct PackageRuntimeGuard {
    steps: Vec<Box<dyn PackageGuardStep>>,
}

impl Default for PackageRuntimeGuard {
    fn default() -> Self {
        Self {
            steps: vec![
                Box::new(SchemaStep),
                Box::new(SignatureMetadataStep),
                Box::new(CompatibilityStep),
                Box::new(PermissionStep),
                Box::new(RequiredServiceStep),
                Box::new(OptionalServiceStep),
                Box::new(CommerceInertStep),
            ],
        }
    }
}

impl PackageRuntimeGuard {
    /// Create the default guard chain.
    pub fn new() -> Self {
        Self::default()
    }

    /// Validate one descriptor through every guard step.
    pub fn evaluate(
        &self,
        mut descriptor: PackageDescriptor,
        context: &PackageGuardContext,
    ) -> PackageDescriptor {
        let mut trace = Vec::new();
        for step in &self.steps {
            emit_step(&descriptor, step.name(), "started", None, &mut trace);
            match step.evaluate(&mut descriptor, context, &mut trace) {
                Ok(()) => emit_step(&descriptor, step.name(), "passed", None, &mut trace),
                Err(error) => {
                    let reason = error.to_string();
                    emit_step(
                        &descriptor,
                        step.name(),
                        "rejected",
                        Some(reason),
                        &mut trace,
                    );
                    descriptor.decision = PackageGuardDecision::Rejected { error };
                    descriptor.trace_events = trace
                        .iter()
                        .map(|event| format!("{}:{}", event.step, event.decision))
                        .collect();
                    return descriptor;
                }
            }
        }
        if descriptor
            .optional_service_status
            .iter()
            .any(|status| matches!(status, PackageServiceAvailability::Unavailable { .. }))
        {
            descriptor.decision = PackageGuardDecision::AcceptedWithUnavailableOptionalServices;
        } else {
            descriptor.decision = PackageGuardDecision::Accepted;
        }
        descriptor.trace_events = trace
            .iter()
            .map(|event| format!("{}:{}", event.step, event.decision))
            .collect();
        descriptor
    }
}

struct SchemaStep;

impl PackageGuardStep for SchemaStep {
    fn name(&self) -> &'static str {
        "schema"
    }

    fn evaluate(
        &self,
        descriptor: &mut PackageDescriptor,
        _context: &PackageGuardContext,
        _trace: &mut Vec<PackageGuardTraceEvent>,
    ) -> PackageResult<()> {
        if descriptor.manifest.runtime.kind.is_none() {
            return Err(PackageGuardError::MissingRuntimeKind);
        }
        if descriptor.manifest.id.as_str().is_empty() {
            return Err(PackageGuardError::InvalidManifest(
                "package id must not be empty".into(),
            ));
        }
        Ok(())
    }
}

struct SignatureMetadataStep;

impl PackageGuardStep for SignatureMetadataStep {
    fn name(&self) -> &'static str {
        "signature_metadata"
    }

    fn evaluate(
        &self,
        descriptor: &mut PackageDescriptor,
        _context: &PackageGuardContext,
        _trace: &mut Vec<PackageGuardTraceEvent>,
    ) -> PackageResult<()> {
        if let Some(signature) = &descriptor.manifest.signature {
            if signature.algorithm.trim().is_empty() || signature.digest.trim().is_empty() {
                return Err(PackageGuardError::InvalidSignatureMetadata(
                    "signature algorithm and digest must be present".into(),
                ));
            }
        }
        Ok(())
    }
}

struct CompatibilityStep;

impl PackageGuardStep for CompatibilityStep {
    fn name(&self) -> &'static str {
        "compatibility"
    }

    fn evaluate(
        &self,
        descriptor: &mut PackageDescriptor,
        context: &PackageGuardContext,
        _trace: &mut Vec<PackageGuardTraceEvent>,
    ) -> PackageResult<()> {
        let runtime_abi = descriptor.manifest.runtime.abi_version.clone();
        if runtime_abi != context.supported_abi_version {
            return Err(PackageGuardError::AbiIncompatible {
                required: runtime_abi,
                supported: context.supported_abi_version.clone(),
            });
        }
        Ok(())
    }
}

struct PermissionStep;

impl PackageGuardStep for PermissionStep {
    fn name(&self) -> &'static str {
        "permission"
    }

    fn evaluate(
        &self,
        _descriptor: &mut PackageDescriptor,
        _context: &PackageGuardContext,
        _trace: &mut Vec<PackageGuardTraceEvent>,
    ) -> PackageResult<()> {
        Ok(())
    }
}

struct RequiredServiceStep;

impl PackageGuardStep for RequiredServiceStep {
    fn name(&self) -> &'static str {
        "required_service"
    }

    fn evaluate(
        &self,
        descriptor: &mut PackageDescriptor,
        context: &PackageGuardContext,
        _trace: &mut Vec<PackageGuardTraceEvent>,
    ) -> PackageResult<()> {
        for requirement in &descriptor.manifest.required_services {
            if !context.available_services.contains(&requirement.service) {
                return Err(PackageGuardError::MissingRequiredService(
                    requirement.service.to_string(),
                ));
            }
        }
        Ok(())
    }
}

struct OptionalServiceStep;

impl PackageGuardStep for OptionalServiceStep {
    fn name(&self) -> &'static str {
        "optional_service"
    }

    fn evaluate(
        &self,
        descriptor: &mut PackageDescriptor,
        context: &PackageGuardContext,
        _trace: &mut Vec<PackageGuardTraceEvent>,
    ) -> PackageResult<()> {
        descriptor.optional_service_status.clear();
        for requirement in &descriptor.manifest.optional_services {
            if context.available_services.contains(&requirement.service) {
                descriptor
                    .optional_service_status
                    .push(PackageServiceAvailability::Available {
                        service: requirement.service.clone(),
                    });
            } else {
                descriptor
                    .optional_service_status
                    .push(PackageServiceAvailability::Unavailable {
                        service: requirement.service.clone(),
                        reason: "optional service is not available".into(),
                    });
            }
        }
        Ok(())
    }
}

struct CommerceInertStep;

impl PackageGuardStep for CommerceInertStep {
    fn name(&self) -> &'static str {
        "commerce_inert"
    }

    fn evaluate(
        &self,
        descriptor: &mut PackageDescriptor,
        _context: &PackageGuardContext,
        _trace: &mut Vec<PackageGuardTraceEvent>,
    ) -> PackageResult<()> {
        descriptor
            .manifest
            .metadata
            .insert("commerce.precheck".into(), "inert".into());
        Ok(())
    }
}

fn emit_step(
    descriptor: &PackageDescriptor,
    step: &str,
    decision: &str,
    reason: Option<String>,
    trace: &mut Vec<PackageGuardTraceEvent>,
) {
    if decision == "rejected" {
        warn!(
            package_id = %descriptor.manifest.id,
            package_type = %descriptor.manifest.package_type,
            runtime_kind = ?descriptor.manifest.runtime.kind,
            step,
            reason = reason.as_deref().unwrap_or(""),
            "package runtime guard rejected package"
        );
    } else {
        info!(
            package_id = %descriptor.manifest.id,
            package_type = %descriptor.manifest.package_type,
            runtime_kind = ?descriptor.manifest.runtime.kind,
            step,
            decision,
            "package runtime guard step"
        );
    }
    trace.push(PackageGuardTraceEvent {
        package_id: descriptor.manifest.id.to_string(),
        step: step.into(),
        decision: decision.into(),
        reason,
    });
}

/// Return whether a descriptor requests WASM execution.
pub fn package_requires_wasm_runtime(descriptor: &PackageDescriptor) -> bool {
    matches!(
        descriptor.manifest.runtime.kind,
        Some(PackageRuntimeKind::WasmComponent)
    )
}

#[cfg(test)]
mod tests {
    use macaca_proto::{
        DeveloperId, PackageId, PackageManifest, PackageRuntime, PackageRuntimeKind, PackageType,
    };

    use super::*;

    fn descriptor_with_runtime(kind: Option<PackageRuntimeKind>, abi: &str) -> PackageDescriptor {
        PackageDescriptor::new(PackageManifest::new(
            PackageId::new("pkg.guard"),
            PackageType::Application,
            "1.0.0",
            DeveloperId::new("dev.guard"),
            PackageRuntime {
                kind,
                abi_version: abi.into(),
            },
        ))
    }

    #[test]
    fn guard_rejects_missing_runtime_kind() {
        let guard = PackageRuntimeGuard::new();
        let result = guard.evaluate(
            descriptor_with_runtime(None, "1"),
            &PackageGuardContext::new("1"),
        );

        assert!(matches!(
            result.decision,
            PackageGuardDecision::Rejected {
                error: PackageGuardError::MissingRuntimeKind
            }
        ));
    }

    #[test]
    fn guard_rejects_incompatible_abi() {
        let guard = PackageRuntimeGuard::new();
        let result = guard.evaluate(
            descriptor_with_runtime(Some(PackageRuntimeKind::Yaml), "99"),
            &PackageGuardContext::new("1"),
        );

        assert!(matches!(
            result.decision,
            PackageGuardDecision::Rejected {
                error: PackageGuardError::AbiIncompatible { .. }
            }
        ));
    }

    #[test]
    fn guard_marks_optional_service_unavailable_without_rejection() {
        let mut descriptor = descriptor_with_runtime(Some(PackageRuntimeKind::Yaml), "1");
        descriptor
            .manifest
            .optional_services
            .push(macaca_proto::PackageServiceRequirement {
                service: KernelServiceId::new("service.optional"),
                capability: None,
                reason: "Optional".into(),
            });
        let guard = PackageRuntimeGuard::new();
        let result = guard.evaluate(descriptor, &PackageGuardContext::new("1"));

        assert_eq!(
            result.decision,
            PackageGuardDecision::AcceptedWithUnavailableOptionalServices
        );
        assert!(matches!(
            result.optional_service_status.first(),
            Some(PackageServiceAvailability::Unavailable { .. })
        ));
    }
}
