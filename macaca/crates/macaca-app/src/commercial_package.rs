//! Application-level commercial package guard wiring.
//!
//! This module is intentionally thin: application code should not duplicate
//! entitlement policy. The guard delegates paid install/start/call decisions
//! to `macaca-runtime-host` while preserving free/open package compatibility.

use macaca_proto::{CommerceError, EntitlementDecision, EntitlementState, PackageManifest};
use macaca_runtime_host::{CapabilityCallContext, EntitlementRuntimeFacade};
use tracing::info;

/// Guard facade used by application/package loaders before commercial actions.
pub struct CommercialPackageGuard<'a> {
    entitlement: &'a EntitlementRuntimeFacade,
}

impl<'a> CommercialPackageGuard<'a> {
    /// Create a guard around the canonical runtime-host entitlement facade.
    pub fn new(entitlement: &'a EntitlementRuntimeFacade) -> Self {
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
        context: CapabilityCallContext,
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
    use std::sync::Arc;

    use chrono::Utc;
    use macaca_persist::{EntitlementStore, InMemoryEntitlementStore};
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

    #[tokio::test]
    async fn free_package_start_does_not_require_store_record() {
        let store = Arc::new(InMemoryEntitlementStore::new());
        let facade = EntitlementRuntimeFacade::new(store);
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
        let store = Arc::new(InMemoryEntitlementStore::new());
        store.upsert_record(record()).await.unwrap();
        let facade = EntitlementRuntimeFacade::new(store);
        let guard = CommercialPackageGuard::new(&facade);

        let decision = guard
            .authorize_install(&manifest(LicenseType::subscription()))
            .await
            .unwrap();

        assert!(decision.allowed);
        assert_eq!(decision.entitlement_id.as_ref().unwrap().as_str(), "entitlement.app");
    }
}
