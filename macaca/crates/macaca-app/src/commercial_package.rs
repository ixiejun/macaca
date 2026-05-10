//! Application-level commercial package guard wiring.
//!
//! This module is intentionally thin: application code should not duplicate
//! entitlement policy. The guard delegates paid install/start/call decisions
//! to `macaca-runtime-host` while preserving free/open package compatibility.

use macaca_proto::{CommerceError, EntitlementDecision, EntitlementState, PackageManifest};
use serde::{Deserialize, Serialize};
use tracing::info;

/// Provider-neutral capability-call context used by application package guards.
///
/// The struct intentionally mirrors the runtime-host entitlement facade context
/// without depending on `macaca-runtime-host`.  This keeps `macaca-app` as the
/// owner of application/package semantics and allows runtime-host to depend on
/// `macaca-app` for Application Service providers without a Cargo cycle.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppCapabilityCallContext {
    pub app_id: Option<String>,
    pub session_id: Option<String>,
    pub capability_id: Option<String>,
    pub quantity: u64,
    pub unit: String,
}

/// Minimal entitlement facade required by application package guards.
///
/// This trait is a Dependency Inversion seam.  Runtime-host can implement it
/// for its entitlement facade, tests can provide a mock, and `macaca-app` no
/// longer needs to know which concrete Store/Entitlement service is installed.
#[async_trait::async_trait]
pub trait ApplicationEntitlementAuthorizer: Send + Sync {
    async fn authorize_install(
        &self,
        manifest: &PackageManifest,
    ) -> Result<EntitlementDecision, CommerceError>;

    async fn authorize_start(
        &self,
        manifest: &PackageManifest,
    ) -> Result<EntitlementDecision, CommerceError>;

    async fn authorize_capability_call(
        &self,
        manifest: &PackageManifest,
        context: AppCapabilityCallContext,
    ) -> Result<EntitlementDecision, CommerceError>;
}

/// Guard facade used by application/package loaders before commercial actions.
pub struct CommercialPackageGuard<'a> {
    entitlement: &'a dyn ApplicationEntitlementAuthorizer,
}

impl<'a> CommercialPackageGuard<'a> {
    /// Create a guard around any entitlement authorizer implementation.
    pub fn new(entitlement: &'a dyn ApplicationEntitlementAuthorizer) -> Self {
        Self { entitlement }
    }

    /// Authorize package installation through the canonical facade.
    pub async fn authorize_install(
        &self,
        manifest: &PackageManifest,
    ) -> Result<EntitlementDecision, CommerceError> {
        self.authorize_free_or_delegate(manifest, "install").await
    }

    /// Authorize package runtime start through the canonical facade.
    pub async fn authorize_start(
        &self,
        manifest: &PackageManifest,
    ) -> Result<EntitlementDecision, CommerceError> {
        self.authorize_free_or_delegate(manifest, "start").await
    }

    /// Authorize a paid capability call and emit metering when configured.
    pub async fn authorize_capability_call(
        &self,
        manifest: &PackageManifest,
        context: AppCapabilityCallContext,
    ) -> Result<EntitlementDecision, CommerceError> {
        if !is_commercial_package(manifest) {
            return Ok(free_decision(manifest, "call"));
        }
        self.entitlement
            .authorize_capability_call(manifest, context)
            .await
    }

    async fn authorize_free_or_delegate(
        &self,
        manifest: &PackageManifest,
        operation: &str,
    ) -> Result<EntitlementDecision, CommerceError> {
        if !is_commercial_package(manifest) {
            info!(
                package_id = %manifest.id,
                operation,
                "commercial package guard allowed free/open package"
            );
            return Ok(free_decision(manifest, operation));
        }
        match operation {
            "install" => self.entitlement.authorize_install(manifest).await,
            "start" => self.entitlement.authorize_start(manifest).await,
            _ => Err(CommerceError::PolicyUnavailable(format!(
                "unsupported commercial package operation: {operation}"
            ))),
        }
    }
}

/// Return whether a package requires Store/Entitlement checks.
pub fn is_commercial_package(manifest: &PackageManifest) -> bool {
    manifest.commerce.store_required || manifest.commerce.license_type.is_paid_family()
}

fn free_decision(manifest: &PackageManifest, operation: &str) -> EntitlementDecision {
    EntitlementDecision::allow(
        manifest.id.clone(),
        manifest.developer.clone(),
        operation,
        EntitlementState::valid(),
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use async_trait::async_trait;
    use chrono::Utc;
    use macaca_proto::{
        DeveloperId, EntitlementId, EntitlementRecord, LicenseType, PackageId, PackageManifest,
        PackageRuntime, PackageRuntimeKind, PackageType,
    };

    use super::*;

    fn manifest(license_type: LicenseType) -> PackageManifest {
        let mut manifest = PackageManifest::new(
            PackageId::new("app.package"),
            PackageType::Application,
            "1.0.0",
            DeveloperId::new("developer.app"),
            PackageRuntime::new(PackageRuntimeKind::Yaml, "1"),
        );
        manifest.commerce.license_type = license_type;
        manifest.commerce.store_required = manifest.commerce.license_type.is_paid_family();
        manifest
    }

    fn record() -> EntitlementRecord {
        let now = Utc::now();
        EntitlementRecord {
            entitlement_id: EntitlementId::new("entitlement.app"),
            package_id: PackageId::new("app.package"),
            developer_id: DeveloperId::new("developer.app"),
            state: EntitlementState::valid(),
            granted_at: now,
            updated_at: now,
            expires_at: None,
            metadata: BTreeMap::new(),
        }
    }

    struct MockEntitlementAuthorizer {
        record: Option<EntitlementRecord>,
    }

    #[async_trait]
    impl ApplicationEntitlementAuthorizer for MockEntitlementAuthorizer {
        async fn authorize_install(
            &self,
            manifest: &PackageManifest,
        ) -> Result<EntitlementDecision, CommerceError> {
            self.decision(manifest, "install")
        }

        async fn authorize_start(
            &self,
            manifest: &PackageManifest,
        ) -> Result<EntitlementDecision, CommerceError> {
            self.decision(manifest, "start")
        }

        async fn authorize_capability_call(
            &self,
            manifest: &PackageManifest,
            _context: AppCapabilityCallContext,
        ) -> Result<EntitlementDecision, CommerceError> {
            self.decision(manifest, "call")
        }
    }

    impl MockEntitlementAuthorizer {
        fn decision(
            &self,
            manifest: &PackageManifest,
            operation: &str,
        ) -> Result<EntitlementDecision, CommerceError> {
            if let Some(record) = &self.record {
                let mut decision = EntitlementDecision::allow(
                    manifest.id.clone(),
                    manifest.developer.clone(),
                    operation,
                    record.state.clone(),
                );
                decision.entitlement_id = Some(record.entitlement_id.clone());
                Ok(decision)
            } else {
                Err(CommerceError::EntitlementRejected("missing".into()))
            }
        }
    }

    #[tokio::test]
    async fn free_package_start_does_not_require_store_record() {
        let facade = MockEntitlementAuthorizer { record: None };
        let guard = CommercialPackageGuard::new(&facade);

        let decision = guard
            .authorize_start(&manifest(LicenseType::open_source()))
            .await
            .unwrap();

        assert!(decision.allowed);
        assert_eq!(decision.state, EntitlementState::valid());
    }

    #[tokio::test]
    async fn paid_package_install_uses_entitlement_facade() {
        let facade = MockEntitlementAuthorizer {
            record: Some(record()),
        };
        let guard = CommercialPackageGuard::new(&facade);

        let decision = guard
            .authorize_install(&manifest(LicenseType::subscription()))
            .await
            .unwrap();

        assert!(decision.allowed);
        assert_eq!(
            decision.entitlement_id.as_ref().unwrap().as_str(),
            "entitlement.app"
        );
    }
}
