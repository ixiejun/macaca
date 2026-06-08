//! Contract tests for `entitlement.rs` (extracted to satisfy OS file-size gate).


    use std::collections::BTreeMap;
    use std::sync::Arc;

    use chrono::Utc;
    use macaca_persist::{InMemoryEntitlementStore, RedbStore};
    use macaca_proto::{
        DeveloperId, EntitlementId, EntitlementRecord, LicenseType, PackageId, PackageManifest,
        PackageRuntime, PackageRuntimeKind, PackageType,
    };

    use super::*;

    fn manifest(license_type: LicenseType) -> PackageManifest {
        let mut manifest = PackageManifest::new(
            PackageId::new("package.entitlement"),
            PackageType::Skill,
            "1.0.0",
            DeveloperId::new("developer.entitlement"),
            PackageRuntime::new(PackageRuntimeKind::Custom("descriptor".into()), "1"),
        );
        manifest.commerce.license_type = license_type;
        manifest.commerce.store_required = manifest.commerce.license_type.is_paid_family();
        manifest.commerce.metering_enabled = true;
        manifest
    }

    fn record(state: EntitlementState) -> EntitlementRecord {
        let now = Utc::now();
        EntitlementRecord {
            entitlement_id: EntitlementId::new("entitlement.valid"),
            package_id: PackageId::new("package.entitlement"),
            developer_id: DeveloperId::new("developer.entitlement"),
            state,
            granted_at: now,
            updated_at: now,
            expires_at: None,
            metadata: BTreeMap::new(),
        }
    }

    #[tokio::test]
    async fn free_package_authorizes_without_store_record() {
        let store = Arc::new(InMemoryEntitlementStore::new());
        let facade = EntitlementRuntimeFacade::new(store);

        let decision = facade
            .authorize_install(&manifest(LicenseType::free()))
            .await
            .unwrap();

        assert!(decision.allowed);
        assert_eq!(decision.state, EntitlementState::valid());
    }

    #[tokio::test]
    async fn paid_package_without_entitlement_is_denied() {
        let store = Arc::new(InMemoryEntitlementStore::new());
        let facade = EntitlementRuntimeFacade::new(store);

        let error = facade
            .authorize_start(&manifest(LicenseType::paid()))
            .await
            .unwrap_err();

        assert!(matches!(error, CommerceError::EntitlementRejected(_)));
    }

    #[tokio::test]
    async fn paid_package_with_valid_entitlement_is_allowed() {
        let store = Arc::new(InMemoryEntitlementStore::new());
        store
            .upsert_record(record(EntitlementState::valid()))
            .await
            .unwrap();
        let facade = EntitlementRuntimeFacade::new(store);

        let decision = facade
            .authorize_start(&manifest(LicenseType::paid()))
            .await
            .unwrap();

        assert!(decision.allowed);
        assert_eq!(
            decision.entitlement_id.as_ref().unwrap().as_str(),
            "entitlement.valid"
        );
    }

    #[tokio::test]
    async fn paid_capability_call_emits_metering_event_to_event_log() {
        let store = Arc::new(InMemoryEntitlementStore::new());
        store
            .upsert_record(record(EntitlementState::valid()))
            .await
            .unwrap();
        let dir = tempfile::tempdir().unwrap();
        let event_log = Arc::new(EventLog::new(Arc::new(
            RedbStore::open(dir.path().join("events.db")).unwrap(),
        )));
        let facade = EntitlementRuntimeFacade::with_event_log(store, Arc::clone(&event_log));

        let decision = facade
            .authorize_capability_call(
                &manifest(LicenseType::metered()),
                CapabilityCallContext::new(CapabilityId::new("capability.call"))
                    .with_app_id("app.entitlement")
                    .with_session_id("session.entitlement"),
            )
            .await
            .unwrap();

        assert!(decision.allowed);
        let entries = event_log.query("session.entitlement", 0, 10).await;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].event_type, "metering_event");
        assert_eq!(entries[0].payload["metadata"]["app_id"], "app.entitlement");
    }
