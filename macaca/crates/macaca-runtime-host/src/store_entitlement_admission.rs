//! Admission specifications shared by Store and Entitlement service providers.
//!
//! These small Specification objects keep validation out of Web, CLI, app, and
//! skill code.  Providers call them before delegating to Phase 08 facades so
//! service commands fail as structured data rather than panicking or silently
//! bypassing trace/policy requirements.

use macaca_proto::{CommerceMetadata, PackageId, ServiceError, TraceContext};

/// Validates that a service command carries a non-empty trace id.
pub struct StoreTraceSpec;

impl StoreTraceSpec {
    /// Return an error when trace correlation is missing or blank.
    pub fn check(trace: &TraceContext) -> Result<(), ServiceError> {
        if trace.trace_id.trim().is_empty() {
            return Err(ServiceError::MissingTraceContext);
        }
        Ok(())
    }
}

/// Validates that package-scoped commands include enough identity to audit.
pub struct PackageScopeSpec;

impl PackageScopeSpec {
    /// Require package id for mutating or authorization paths.
    pub fn require_package(
        package_id: Option<&PackageId>,
        operation: &str,
    ) -> Result<(), ServiceError> {
        if package_id.is_none() {
            return Err(ServiceError::AdapterFailure(format!(
                "{operation} requires package_id"
            )));
        }
        Ok(())
    }
}

/// Validates closed operation labels used by the service providers.
pub struct EntitlementOperationSpec;

impl EntitlementOperationSpec {
    /// Ensure operation names stay in the S9 authorization vocabulary.
    pub fn check(operation: &str) -> Result<(), ServiceError> {
        match operation {
            "install" | "start" | "call" | "query" | "upsert" | "revoke" | "audit" | "metering" => {
                Ok(())
            }
            other => Err(ServiceError::UnsupportedCommand(format!(
                "unsupported Store/Entitlement operation '{other}'"
            ))),
        }
    }
}

/// Validates metering quantities before they reach the audit bridge.
pub struct MeteringScopeSpec;

impl MeteringScopeSpec {
    /// Quantity must be non-zero so metering cannot emit meaningless usage.
    pub fn check(quantity: u64) -> Result<(), ServiceError> {
        if quantity == 0 {
            return Err(ServiceError::AdapterFailure(
                "metering quantity must be greater than zero".into(),
            ));
        }
        Ok(())
    }
}

/// Classifies free/open package commerce metadata.
pub struct FreeOpenFastPathSpec;

impl FreeOpenFastPathSpec {
    /// Return true when Store unavailability must not block local execution.
    pub fn allows_without_store(commerce: &CommerceMetadata) -> bool {
        !commerce.store_required && !commerce.license_type.is_paid_family()
    }
}
