//! Runtime Store/Entitlement facade for Route C Phase 08.
//!
//! The runtime host owns this facade because entitlement policy is replaceable
//! service behavior, not a kernel invariant. The facade deliberately exposes a
//! small set of operations (`install`, `start`, and capability `call`) so app,
//! skill, plugin, MCP, and future store adapters can share one auditable
//! decision pipeline.

use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::Utc;
use macaca_persist::{AppendEventCommand, EntitlementStore, EventLog};
use macaca_proto::{
    CapabilityId, CommerceError, EntitlementDecision, EntitlementRecord, EntitlementState,
    MeteringEvent, MeteringEventId, PackageManifest,
};
use tracing::{info, warn};

/// Runtime operation protected by the entitlement facade.
///
/// A closed operation enum keeps the public facade readable while the emitted
/// audit payload remains string-based and forward-compatible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntitlementOperation {
    Install,
    Start,
    CapabilityCall,
}

impl EntitlementOperation {
    /// Return the canonical operation label used in logs and persisted audits.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Install => "install",
            Self::Start => "start",
            Self::CapabilityCall => "call",
        }
    }
}

/// Additional identity context for capability-call authorization.
///
/// The fields are optional because install/start paths often happen before a
/// user session exists. Capability calls should pass a session id whenever one
/// is available so metering events can be replayed through the existing
/// session-scoped EventLog.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CapabilityCallContext {
    pub app_id: Option<String>,
    pub session_id: Option<String>,
    pub capability_id: Option<CapabilityId>,
    pub quantity: u64,
    pub unit: String,
}

impl CapabilityCallContext {
    /// Build context for one capability call.
    pub fn new(capability_id: CapabilityId) -> Self {
        Self {
            app_id: None,
            session_id: None,
            capability_id: Some(capability_id),
            quantity: 1,
            unit: "call".into(),
        }
    }

    /// Attach the application id used by EventLog and audit consumers.
    pub fn with_app_id(mut self, app_id: impl Into<String>) -> Self {
        self.app_id = Some(app_id.into());
        self
    }

    /// Attach the session id used by EventLog replay.
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }
}

/// Facade for runtime entitlement decisions and metering emission.
///
/// The facade composes Repository + Specification + Observer patterns:
/// entitlement state is read through a repository trait, decision rules are
/// isolated in small predicate methods, and every decision is persisted as an
/// audit record. When an EventLog is supplied, paid capability calls also emit
/// a session-scoped metering event compatible with existing trace replay.
pub struct EntitlementRuntimeFacade {
    store: Arc<dyn EntitlementStore>,
    event_log: Option<Arc<EventLog>>,
}

impl EntitlementRuntimeFacade {
    /// Create a facade with an entitlement store and no EventLog bridge.
    pub fn new(store: Arc<dyn EntitlementStore>) -> Self {
        Self {
            store,
            event_log: None,
        }
    }

    /// Create a facade that also writes metering events into EventLog.
    pub fn with_event_log(store: Arc<dyn EntitlementStore>, event_log: Arc<EventLog>) -> Self {
        Self {
            store,
            event_log: Some(event_log),
        }
    }

    /// Authorize package installation.
    pub async fn authorize_install(
        &self,
        manifest: &PackageManifest,
    ) -> Result<EntitlementDecision, CommerceError> {
        self.authorize_operation(manifest, EntitlementOperation::Install, None)
            .await
    }

    /// Authorize package start or runtime activation.
    pub async fn authorize_start(
        &self,
        manifest: &PackageManifest,
    ) -> Result<EntitlementDecision, CommerceError> {
        self.authorize_operation(manifest, EntitlementOperation::Start, None)
            .await
    }

    /// Authorize a paid capability call and emit metering when configured.
    pub async fn authorize_capability_call(
        &self,
        manifest: &PackageManifest,
        context: CapabilityCallContext,
    ) -> Result<EntitlementDecision, CommerceError> {
        let decision = self
            .authorize_operation(
                manifest,
                EntitlementOperation::CapabilityCall,
                Some(&context),
            )
            .await?;
        if decision.allowed && manifest.commerce.metering_enabled {
            self.record_metering(manifest, &decision, context).await?;
        }
        Ok(decision)
    }

    async fn authorize_operation(
        &self,
        manifest: &PackageManifest,
        operation: EntitlementOperation,
        context: Option<&CapabilityCallContext>,
    ) -> Result<EntitlementDecision, CommerceError> {
        info!(
            package_id = %manifest.id,
            developer_id = %manifest.developer,
            operation = operation.as_str(),
            license_type = %manifest.commerce.license_type,
            "entitlement validation started"
        );

        let mut decision = if !manifest.commerce.license_type.is_paid_family()
            && !manifest.commerce.store_required
        {
            EntitlementDecision::allow(
                manifest.id.clone(),
                manifest.developer.clone(),
                operation.as_str(),
                EntitlementState::valid(),
            )
        } else {
            let record = self.resolve_record(manifest).await?;
            self.decision_from_record(manifest, operation, record.as_ref())
        };

        self.attach_context(&mut decision, context);
        self.store.append_decision_audit(decision.clone()).await?;

        if decision.allowed {
            info!(
                package_id = %decision.package_id,
                developer_id = %decision.developer_id,
                operation = %decision.operation,
                state = %decision.state,
                "entitlement validation passed"
            );
        } else {
            warn!(
                package_id = %decision.package_id,
                developer_id = %decision.developer_id,
                operation = %decision.operation,
                state = %decision.state,
                reason = %decision.reason,
                "entitlement validation rejected"
            );
            return Err(CommerceError::EntitlementRejected(
                decision.state.to_string(),
            ));
        }

        Ok(decision)
    }

    async fn resolve_record(
        &self,
        manifest: &PackageManifest,
    ) -> Result<Option<EntitlementRecord>, CommerceError> {
        if let Some(entitlement_id) = &manifest.commerce.entitlement_id {
            let record = self.store.get_record(entitlement_id).await?;
            info!(
                package_id = %manifest.id,
                entitlement_id = %entitlement_id,
                found = record.is_some(),
                "entitlement resolved by manifest entitlement id"
            );
            return Ok(record);
        }
        self.store.resolve_for_package(&manifest.id).await
    }

    fn decision_from_record(
        &self,
        manifest: &PackageManifest,
        operation: EntitlementOperation,
        record: Option<&EntitlementRecord>,
    ) -> EntitlementDecision {
        let Some(record) = record else {
            return EntitlementDecision::deny(
                manifest.id.clone(),
                manifest.developer.clone(),
                operation.as_str(),
                EntitlementState::missing(),
                "paid package requires a valid entitlement",
            );
        };

        if record.developer_id != manifest.developer {
            return EntitlementDecision::deny(
                manifest.id.clone(),
                manifest.developer.clone(),
                operation.as_str(),
                EntitlementState::missing(),
                "entitlement developer scope does not match package developer",
            );
        }
        if record.package_id != manifest.id {
            return EntitlementDecision::deny(
                manifest.id.clone(),
                manifest.developer.clone(),
                operation.as_str(),
                EntitlementState::missing(),
                "entitlement package scope does not match package id",
            );
        }

        let mut decision = if self.state_allows_operation(&record.state, operation, manifest) {
            EntitlementDecision::allow(
                manifest.id.clone(),
                manifest.developer.clone(),
                operation.as_str(),
                record.state.clone(),
            )
        } else {
            EntitlementDecision::deny(
                manifest.id.clone(),
                manifest.developer.clone(),
                operation.as_str(),
                record.state.clone(),
                format!(
                    "entitlement state {} does not allow operation",
                    record.state
                ),
            )
        };
        decision.entitlement_id = Some(record.entitlement_id.clone());
        decision
    }

    fn state_allows_operation(
        &self,
        state: &EntitlementState,
        operation: EntitlementOperation,
        manifest: &PackageManifest,
    ) -> bool {
        match state.as_str() {
            "valid" => true,
            "unknown_offline" => {
                operation == EntitlementOperation::Start
                    && manifest.commerce.offline_grace_period_seconds.is_some()
            }
            _ => false,
        }
    }

    fn attach_context(
        &self,
        decision: &mut EntitlementDecision,
        context: Option<&CapabilityCallContext>,
    ) {
        let Some(context) = context else {
            return;
        };
        if let Some(app_id) = &context.app_id {
            decision.metadata.insert("app_id".into(), app_id.clone());
        }
        if let Some(session_id) = &context.session_id {
            decision
                .metadata
                .insert("session_id".into(), session_id.clone());
        }
        if let Some(capability_id) = &context.capability_id {
            decision
                .metadata
                .insert("capability_id".into(), capability_id.to_string());
        }
    }

    /// Emit a structured metering event for an already-authorized paid call.
    pub async fn record_metering(
        &self,
        manifest: &PackageManifest,
        decision: &EntitlementDecision,
        context: CapabilityCallContext,
    ) -> Result<MeteringEvent, CommerceError> {
        let event = MeteringEvent {
            event_id: MeteringEventId::new(format!(
                "metering.{}.{}",
                manifest.id,
                Utc::now().timestamp_micros()
            )),
            entitlement_id: decision.entitlement_id.clone(),
            package_id: manifest.id.clone(),
            developer_id: manifest.developer.clone(),
            capability_id: context.capability_id.clone(),
            session_id: context.session_id.clone(),
            operation: decision.operation.clone(),
            quantity: context.quantity.max(1),
            unit: if context.unit.is_empty() {
                "call".into()
            } else {
                context.unit.clone()
            },
            status: "authorized".into(),
            timestamp: Utc::now(),
            metadata: metering_metadata(manifest, decision, &context),
        };

        info!(
            package_id = %event.package_id,
            developer_id = %event.developer_id,
            operation = %event.operation,
            status = %event.status,
            "metering event emitted"
        );

        if let (Some(event_log), Some(session_id)) = (&self.event_log, &event.session_id) {
            let payload = serde_json::to_value(&event)
                .map_err(|error| CommerceError::MeteringRejected(error.to_string()))?;
            let mut command = AppendEventCommand::new(
                session_id.clone(),
                "metering_event",
                "entitlement",
                payload,
            );
            if let Some(app_id) = context.app_id {
                command = command.with_app_id(app_id);
            }
            event_log.append_command(command).await;
        }

        Ok(event)
    }
}

fn metering_metadata(
    manifest: &PackageManifest,
    decision: &EntitlementDecision,
    context: &CapabilityCallContext,
) -> BTreeMap<String, String> {
    let mut metadata = BTreeMap::new();
    metadata.insert("package_version".into(), manifest.version.clone());
    metadata.insert("decision_state".into(), decision.state.to_string());
    metadata.insert("decision_allowed".into(), decision.allowed.to_string());
    if let Some(app_id) = &context.app_id {
        metadata.insert("app_id".into(), app_id.clone());
    }
    if let Some(capability_id) = &context.capability_id {
        metadata.insert("capability_id".into(), capability_id.to_string());
    }
    metadata
}

#[cfg(test)]
mod tests {
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
}
