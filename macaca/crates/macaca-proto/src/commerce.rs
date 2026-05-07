//! Provider-neutral Store/Entitlement contracts for Route C Phase 08.
//!
//! This module defines data-only commerce primitives used by package guards,
//! entitlement services, encrypted package hooks, and metering/audit flows.
//! The contracts intentionally avoid binding to concrete payment providers,
//! marketplace vendors, chain networks, or application-specific workflows.

use std::collections::BTreeMap;
use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{CapabilityId, DeveloperId, PackageId};

/// Stable entitlement identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EntitlementId(String);

impl EntitlementId {
    /// Create an entitlement identifier after trimming surrounding whitespace.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into().trim().to_string())
    }

    /// Return the raw entitlement identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EntitlementId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Stable subscription plan identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SubscriptionPlanId(String);

impl SubscriptionPlanId {
    /// Create a subscription plan identifier after trimming surrounding whitespace.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into().trim().to_string())
    }

    /// Return the raw plan identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SubscriptionPlanId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Stable metering event identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MeteringEventId(String);

impl MeteringEventId {
    /// Create a metering event identifier after trimming surrounding whitespace.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into().trim().to_string())
    }

    /// Return the raw metering event identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for MeteringEventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// String-backed license classification.
///
/// A string-backed value keeps compatibility with future third-party store
/// vocabularies while exposing canonical helper constructors and predicates.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LicenseType(String);

impl LicenseType {
    /// Create a normalized license type from a raw string.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into().trim().to_lowercase())
    }

    /// Canonical free license marker.
    pub fn free() -> Self {
        Self("free".into())
    }

    /// Canonical open-source license marker.
    pub fn open_source() -> Self {
        Self("open_source".into())
    }

    /// Canonical one-time paid license marker.
    pub fn paid() -> Self {
        Self("paid".into())
    }

    /// Canonical subscription license marker.
    pub fn subscription() -> Self {
        Self("subscription".into())
    }

    /// Canonical metered license marker.
    pub fn metered() -> Self {
        Self("metered".into())
    }

    /// Return the normalized license type string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Return true when runtime authorization requires entitlement checks.
    pub fn is_paid_family(&self) -> bool {
        matches!(self.0.as_str(), "paid" | "subscription" | "metered")
    }
}

impl Default for LicenseType {
    fn default() -> Self {
        Self::free()
    }
}

impl fmt::Display for LicenseType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// String-backed entitlement state.
///
/// Phase 08 requires well-known states while preserving unknown values for
/// forward-compatible transport and storage.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EntitlementState(String);

impl EntitlementState {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into().trim().to_lowercase())
    }

    pub fn valid() -> Self {
        Self("valid".into())
    }

    pub fn expired() -> Self {
        Self("expired".into())
    }

    pub fn missing() -> Self {
        Self("missing".into())
    }

    pub fn revoked() -> Self {
        Self("revoked".into())
    }

    pub fn region_blocked() -> Self {
        Self("region_blocked".into())
    }

    pub fn usage_exceeded() -> Self {
        Self("usage_exceeded".into())
    }

    pub fn unknown_offline() -> Self {
        Self("unknown_offline".into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EntitlementState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Commerce metadata attached to package manifests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommerceMetadata {
    pub license_type: LicenseType,
    pub store_required: bool,
    pub entitlement_id: Option<EntitlementId>,
    pub subscription_plan: Option<SubscriptionPlanId>,
    pub metering_enabled: bool,
    pub offline_grace_period_seconds: Option<u64>,
    pub revocation_policy: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

impl Default for CommerceMetadata {
    fn default() -> Self {
        Self {
            license_type: LicenseType::free(),
            store_required: false,
            entitlement_id: None,
            subscription_plan: None,
            metering_enabled: false,
            offline_grace_period_seconds: None,
            revocation_policy: None,
            metadata: BTreeMap::new(),
        }
    }
}

/// Subscription plan metadata used for policy and diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriptionPlan {
    pub id: SubscriptionPlanId,
    pub name: String,
    pub billing_period: String,
    pub metadata: BTreeMap<String, String>,
}

/// Entitlement record stored by persistence implementations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntitlementRecord {
    pub entitlement_id: EntitlementId,
    pub package_id: PackageId,
    pub developer_id: DeveloperId,
    pub state: EntitlementState,
    pub granted_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub metadata: BTreeMap<String, String>,
}

/// Auditable entitlement decision emitted by runtime guards.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntitlementDecision {
    pub entitlement_id: Option<EntitlementId>,
    pub package_id: PackageId,
    pub developer_id: DeveloperId,
    pub operation: String,
    pub state: EntitlementState,
    pub allowed: bool,
    pub reason: String,
    pub decided_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

impl EntitlementDecision {
    /// Build an allowed decision with explicit operation context.
    pub fn allow(
        package_id: PackageId,
        developer_id: DeveloperId,
        operation: impl Into<String>,
        state: EntitlementState,
    ) -> Self {
        Self {
            entitlement_id: None,
            package_id,
            developer_id,
            operation: operation.into(),
            state,
            allowed: true,
            reason: "authorized".into(),
            decided_at: Utc::now(),
            metadata: BTreeMap::new(),
        }
    }

    /// Build a denied decision with explicit reason.
    pub fn deny(
        package_id: PackageId,
        developer_id: DeveloperId,
        operation: impl Into<String>,
        state: EntitlementState,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            entitlement_id: None,
            package_id,
            developer_id,
            operation: operation.into(),
            state,
            allowed: false,
            reason: reason.into(),
            decided_at: Utc::now(),
            metadata: BTreeMap::new(),
        }
    }
}

/// Structured metering event emitted for paid capability calls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeteringEvent {
    pub event_id: MeteringEventId,
    pub entitlement_id: Option<EntitlementId>,
    pub package_id: PackageId,
    pub developer_id: DeveloperId,
    pub capability_id: Option<CapabilityId>,
    pub session_id: Option<String>,
    pub operation: String,
    pub quantity: u64,
    pub unit: String,
    pub status: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

/// Structured errors used by Store/Entitlement boundaries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum CommerceError {
    #[error("missing entitlement id")]
    MissingEntitlementId,

    #[error("entitlement not found: {0}")]
    EntitlementNotFound(String),

    #[error("entitlement rejected with state: {0}")]
    EntitlementRejected(String),

    #[error("entitlement policy unavailable: {0}")]
    PolicyUnavailable(String),

    #[error("metering rejected: {0}")]
    MeteringRejected(String),

    #[error("encrypted package denied: {0}")]
    EncryptedPackageDenied(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_metadata(license_type: LicenseType) -> CommerceMetadata {
        let mut metadata = CommerceMetadata {
            license_type,
            store_required: true,
            entitlement_id: Some(EntitlementId::new("entitlement.test")),
            subscription_plan: Some(SubscriptionPlanId::new("plan.pro")),
            metering_enabled: true,
            offline_grace_period_seconds: Some(3600),
            revocation_policy: Some("deny_immediately".into()),
            metadata: BTreeMap::new(),
        };
        metadata
            .metadata
            .insert("distribution".into(), "marketplace".into());
        metadata
    }

    #[test]
    fn free_open_paid_subscription_metered_round_trip() {
        for license in [
            LicenseType::free(),
            LicenseType::open_source(),
            LicenseType::paid(),
            LicenseType::subscription(),
            LicenseType::metered(),
        ] {
            let metadata = fixture_metadata(license.clone());
            let encoded = serde_json::to_vec(&metadata).unwrap();
            let decoded: CommerceMetadata = serde_json::from_slice(&encoded).unwrap();
            assert_eq!(decoded.license_type, license);
            assert!(decoded.store_required);
            assert!(decoded.metering_enabled);
        }
    }

    #[test]
    fn unknown_license_value_is_preserved() {
        let metadata = fixture_metadata(LicenseType::new("enterprise_bundle"));
        let decoded: CommerceMetadata =
            serde_json::from_str(&serde_json::to_string(&metadata).unwrap()).unwrap();
        assert_eq!(decoded.license_type.as_str(), "enterprise_bundle");
        assert!(!decoded.license_type.is_paid_family());
    }
}
