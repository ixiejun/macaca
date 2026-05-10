//! Entitlement persistence contracts for Store/Entitlement Runtime v0.
//!
//! This module provides a repository-style boundary for storing and querying
//! entitlement state and audit artifacts. The in-memory implementation is
//! intentionally deterministic and test-friendly while preserving the same
//! contract expected by future persistent backends.

use std::collections::{BTreeMap, HashMap};

use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::RwLock;
use tracing::{info, warn};

use macaca_proto::{
    CommerceError, EntitlementDecision, EntitlementId, EntitlementRecord, EntitlementState,
    PackageId,
};

/// Repository contract for entitlement persistence.
///
/// Store/Entitlement services and runtime guards depend on this abstraction to
/// avoid coupling policy decisions to a concrete database engine.
#[async_trait]
pub trait EntitlementStore: Send + Sync {
    /// Upsert one entitlement record by `entitlement_id`.
    async fn upsert_record(&self, record: EntitlementRecord) -> Result<(), CommerceError>;

    /// Get one entitlement record by id.
    async fn get_record(
        &self,
        entitlement_id: &EntitlementId,
    ) -> Result<Option<EntitlementRecord>, CommerceError>;

    /// Resolve effective entitlement by package id with deterministic precedence.
    async fn resolve_for_package(
        &self,
        package_id: &PackageId,
    ) -> Result<Option<EntitlementRecord>, CommerceError>;

    /// Append a decision audit record.
    async fn append_decision_audit(
        &self,
        decision: EntitlementDecision,
    ) -> Result<(), CommerceError>;

    /// Return audit records associated with one package.
    async fn decision_audit_by_package(
        &self,
        package_id: &PackageId,
    ) -> Result<Vec<EntitlementDecision>, CommerceError>;
}

/// In-memory entitlement repository used for tests and local runtime scaffolds.
///
/// The implementation keeps deterministic precedence:
/// `revoked > usage_exceeded > region_blocked > expired > unknown_offline > valid > missing`.
#[derive(Default)]
pub struct InMemoryEntitlementStore {
    records: RwLock<HashMap<EntitlementId, EntitlementRecord>>,
    package_index: RwLock<BTreeMap<PackageId, Vec<EntitlementId>>>,
    decision_audit: RwLock<Vec<EntitlementDecision>>,
}

impl InMemoryEntitlementStore {
    /// Create an empty in-memory entitlement store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Precedence ranking used to resolve one effective entitlement from
    /// multiple records linked to the same package.
    fn state_rank(state: &EntitlementState) -> i32 {
        match state.as_str() {
            "revoked" => 100,
            "usage_exceeded" => 90,
            "region_blocked" => 80,
            "expired" => 70,
            "unknown_offline" => 60,
            "valid" => 50,
            "missing" => 10,
            _ => 40,
        }
    }
}

#[async_trait]
impl EntitlementStore for InMemoryEntitlementStore {
    async fn upsert_record(&self, mut record: EntitlementRecord) -> Result<(), CommerceError> {
        record.updated_at = Utc::now();
        info!(
            entitlement_id = %record.entitlement_id,
            package_id = %record.package_id,
            state = %record.state,
            "entitlement record upserted"
        );

        self.records
            .write()
            .await
            .insert(record.entitlement_id.clone(), record.clone());
        let mut index = self.package_index.write().await;
        let ids = index.entry(record.package_id.clone()).or_default();
        if !ids.contains(&record.entitlement_id) {
            ids.push(record.entitlement_id);
        }
        Ok(())
    }

    async fn get_record(
        &self,
        entitlement_id: &EntitlementId,
    ) -> Result<Option<EntitlementRecord>, CommerceError> {
        let found = self.records.read().await.get(entitlement_id).cloned();
        info!(
            entitlement_id = %entitlement_id,
            found = found.is_some(),
            "entitlement record queried by id"
        );
        Ok(found)
    }

    async fn resolve_for_package(
        &self,
        package_id: &PackageId,
    ) -> Result<Option<EntitlementRecord>, CommerceError> {
        let index = self.package_index.read().await;
        let Some(ids) = index.get(package_id) else {
            info!(package_id = %package_id, "no entitlement record found for package");
            return Ok(None);
        };
        let records = self.records.read().await;
        let mut selected: Option<EntitlementRecord> = None;
        for id in ids {
            if let Some(candidate) = records.get(id) {
                selected = match selected.take() {
                    None => Some(candidate.clone()),
                    Some(current) => {
                        let candidate_rank = Self::state_rank(&candidate.state);
                        let current_rank = Self::state_rank(&current.state);
                        if candidate_rank > current_rank
                            || (candidate_rank == current_rank
                                && candidate.updated_at > current.updated_at)
                        {
                            Some(candidate.clone())
                        } else {
                            Some(current)
                        }
                    }
                };
            }
        }
        if let Some(ref resolved) = selected {
            if resolved.state.as_str() == "revoked" {
                warn!(
                    package_id = %package_id,
                    entitlement_id = %resolved.entitlement_id,
                    "revoked entitlement resolved with highest precedence"
                );
            } else {
                info!(
                    package_id = %package_id,
                    entitlement_id = %resolved.entitlement_id,
                    state = %resolved.state,
                    "entitlement resolved for package"
                );
            }
        }
        Ok(selected)
    }

    async fn append_decision_audit(
        &self,
        decision: EntitlementDecision,
    ) -> Result<(), CommerceError> {
        info!(
            package_id = %decision.package_id,
            developer_id = %decision.developer_id,
            operation = %decision.operation,
            allowed = decision.allowed,
            state = %decision.state,
            "entitlement decision audit appended"
        );
        self.decision_audit.write().await.push(decision);
        Ok(())
    }

    async fn decision_audit_by_package(
        &self,
        package_id: &PackageId,
    ) -> Result<Vec<EntitlementDecision>, CommerceError> {
        let list = self
            .decision_audit
            .read()
            .await
            .iter()
            .filter(|decision| &decision.package_id == package_id)
            .cloned()
            .collect::<Vec<_>>();
        Ok(list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::{DeveloperId, EntitlementDecision, EntitlementState};

    fn record(
        entitlement_id: &str,
        package_id: &str,
        state: EntitlementState,
    ) -> EntitlementRecord {
        let now = Utc::now();
        EntitlementRecord {
            entitlement_id: EntitlementId::new(entitlement_id),
            package_id: PackageId::new(package_id),
            developer_id: DeveloperId::new("developer.test"),
            state,
            granted_at: now,
            updated_at: now,
            expires_at: None,
            metadata: BTreeMap::new(),
        }
    }

    #[tokio::test]
    async fn upsert_and_get_record_work() {
        let store = InMemoryEntitlementStore::new();
        let entry = record("entitlement.one", "package.one", EntitlementState::valid());
        store.upsert_record(entry.clone()).await.unwrap();
        let loaded = store
            .get_record(&EntitlementId::new("entitlement.one"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.entitlement_id, entry.entitlement_id);
        assert_eq!(loaded.state, EntitlementState::valid());
    }

    #[tokio::test]
    async fn revoked_overrides_valid_for_same_package() {
        let store = InMemoryEntitlementStore::new();
        store
            .upsert_record(record(
                "entitlement.valid",
                "package.one",
                EntitlementState::valid(),
            ))
            .await
            .unwrap();
        store
            .upsert_record(record(
                "entitlement.revoked",
                "package.one",
                EntitlementState::revoked(),
            ))
            .await
            .unwrap();
        let resolved = store
            .resolve_for_package(&PackageId::new("package.one"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(resolved.state, EntitlementState::revoked());
    }

    #[tokio::test]
    async fn decision_audit_append_and_query_work() {
        let store = InMemoryEntitlementStore::new();
        let mut decision = EntitlementDecision::deny(
            PackageId::new("package.audit"),
            DeveloperId::new("developer.test"),
            "start",
            EntitlementState::missing(),
            "entitlement missing",
        );
        decision.entitlement_id = Some(EntitlementId::new("entitlement.audit"));
        store.append_decision_audit(decision).await.unwrap();
        let list = store
            .decision_audit_by_package(&PackageId::new("package.audit"))
            .await
            .unwrap();
        assert_eq!(list.len(), 1);
        assert!(!list[0].allowed);
    }
}
