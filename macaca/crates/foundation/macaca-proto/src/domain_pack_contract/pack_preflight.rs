use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::model::DomainPackDefinition;

/// Bounded result categories exposed before a service provider is dispatched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DomainPackPreflightStatus {
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    QuotaExceeded,
}

/// Trace-safe reason for a command rejected before a provider side effect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainPackPreflightRejection {
    pub status: DomainPackPreflightStatus,
    pub reason_code: String,
}

impl DomainPackPreflightRejection {
    fn new(status: DomainPackPreflightStatus, reason_code: &str) -> Self {
        Self {
            status,
            reason_code: reason_code.into(),
        }
    }
}

/// Policy evidence produced outside the contract layer and represented by references only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainPackPolicyEvidence {
    pub decision_ref: String,
    pub allowed: bool,
    pub reason_code: String,
}

/// Approval evidence required for commands with sensitive side effects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainPackApprovalEvidence {
    pub approval_ref: String,
    pub approved: bool,
    pub reason_code: String,
}

/// Entitlement and runtime availability facts supplied by the host composition root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainPackEntitlementEvidence {
    pub entitlement_ref: String,
    pub provider_available: bool,
    pub scope_granted: bool,
    pub command_supported: bool,
    pub host_capability_enabled: bool,
    pub reason_code: String,
}

/// Resource units reserved by a host-owned meter before a provider call.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainPackResourceReservation {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub units: BTreeMap<String, u64>,
}

impl DomainPackResourceReservation {
    /// Return true when every required named unit is reserved at or above its demand.
    pub fn covers(&self, required: &Self) -> bool {
        required
            .units
            .iter()
            .all(|(unit, required_units)| self.units.get(unit).unwrap_or(&0) >= required_units)
    }
}

/// Provider-neutral input evaluated before an application command can dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainPackCommandPreflight {
    pub command_name: String,
    pub requested_scope: String,
    pub policy: DomainPackPolicyEvidence,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval: Option<DomainPackApprovalEvidence>,
    pub entitlement: DomainPackEntitlementEvidence,
    pub required_resources: DomainPackResourceReservation,
    pub reserved_resources: DomainPackResourceReservation,
}

/// Descriptor-driven Specification used by service runtimes before provider dispatch.
///
/// This remains generic: it validates descriptor-owned command and scope
/// allowlists plus host-supplied evidence, but it never selects a provider or
/// interprets application, workflow, tenant, or business semantics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainPackCommandPreflightSpec {
    allowed_commands: BTreeSet<String>,
    allowed_scopes: BTreeSet<String>,
    approval_required_commands: BTreeSet<String>,
}

impl DomainPackCommandPreflightSpec {
    /// Construct a preflight Specification from descriptor command and scope metadata.
    pub fn from_definition(
        definition: &DomainPackDefinition,
        approval_required_commands: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            allowed_commands: definition
                .metadata
                .service_command_schemas
                .values()
                .flatten()
                .cloned()
                .collect(),
            allowed_scopes: definition.metadata.permission_scopes.clone(),
            approval_required_commands: approval_required_commands
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }

    /// Reject invalid or unentitled commands before the provider closure is reachable.
    pub fn evaluate(
        &self,
        preflight: &DomainPackCommandPreflight,
    ) -> Result<(), DomainPackPreflightRejection> {
        if !bounded(&preflight.command_name)
            || !self.allowed_commands.contains(&preflight.command_name)
        {
            return Err(DomainPackPreflightRejection::new(
                DomainPackPreflightStatus::Unsupported,
                "unsupported_command",
            ));
        }
        if !bounded(&preflight.requested_scope)
            || !self.allowed_scopes.contains(&preflight.requested_scope)
        {
            return Err(DomainPackPreflightRejection::new(
                DomainPackPreflightStatus::Denied,
                "permission_not_declared",
            ));
        }
        if !valid_policy(&preflight.policy) || !preflight.policy.allowed {
            return Err(DomainPackPreflightRejection::new(
                DomainPackPreflightStatus::Denied,
                "policy_denied",
            ));
        }
        if !valid_entitlement(&preflight.entitlement) || !preflight.entitlement.provider_available {
            return Err(DomainPackPreflightRejection::new(
                DomainPackPreflightStatus::Unavailable,
                "provider_unavailable",
            ));
        }
        if !preflight.entitlement.scope_granted {
            return Err(DomainPackPreflightRejection::new(
                DomainPackPreflightStatus::Denied,
                "entitlement_denied",
            ));
        }
        if !preflight.entitlement.command_supported {
            return Err(DomainPackPreflightRejection::new(
                DomainPackPreflightStatus::Unsupported,
                "command_not_supported",
            ));
        }
        if !preflight.entitlement.host_capability_enabled {
            return Err(DomainPackPreflightRejection::new(
                DomainPackPreflightStatus::Unavailable,
                "host_capability_disabled",
            ));
        }
        if self
            .approval_required_commands
            .contains(&preflight.command_name)
            && !preflight.approval.as_ref().is_some_and(valid_approval)
        {
            return Err(DomainPackPreflightRejection::new(
                DomainPackPreflightStatus::Denied,
                "approval_required",
            ));
        }
        if !preflight
            .reserved_resources
            .covers(&preflight.required_resources)
        {
            return Err(DomainPackPreflightRejection::new(
                DomainPackPreflightStatus::QuotaExceeded,
                "resource_reservation_insufficient",
            ));
        }
        Ok(())
    }

    /// Invoke a provider closure only after the complete preflight is accepted.
    pub fn dispatch_after_preflight<T>(
        &self,
        preflight: &DomainPackCommandPreflight,
        dispatch: impl FnOnce() -> T,
    ) -> Result<T, DomainPackPreflightRejection> {
        self.evaluate(preflight)?;
        Ok(dispatch())
    }
}

fn bounded(value: &str) -> bool {
    !value.is_empty() && value.len() <= 128 && !value.contains('\n')
}

fn valid_policy(evidence: &DomainPackPolicyEvidence) -> bool {
    bounded(&evidence.decision_ref) && bounded(&evidence.reason_code)
}

fn valid_approval(evidence: &DomainPackApprovalEvidence) -> bool {
    evidence.approved && bounded(&evidence.approval_ref) && bounded(&evidence.reason_code)
}

fn valid_entitlement(evidence: &DomainPackEntitlementEvidence) -> bool {
    bounded(&evidence.entitlement_ref) && bounded(&evidence.reason_code)
}
