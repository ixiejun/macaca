//! Provider-neutral organization governance semantics.
//!
//! State, Specification, Strategy, and Memento patterns keep organization
//! safety checks deterministic before a concrete directory provider is called.
//! Only bounded handles, hashes, counters, and replay references are retained.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

/// Lifecycle of a generic durable organization container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrganizationLifecycle {
    Planned,
    Active,
    Archived,
    Restored,
    Cancelled,
}

/// Lifecycle of a member relationship without owning account or profile state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrganizationMembershipLifecycle {
    Planned,
    Active,
    Suspended,
    Removed,
    DirectoryManaged,
}

/// Lifecycle of an invitation without retaining its delivery token or content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrganizationInvitationLifecycle {
    Planned,
    Pending,
    Accepted,
    Revoked,
    Expired,
}

/// Lifecycle of a role binding as provider-neutral identity evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrganizationRoleBindingLifecycle {
    Planned,
    Active,
    Removed,
    DirectoryManaged,
}

/// Bounded mutation facts supplied by policy and provider adapters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizationMutationEvidenceV1 {
    pub command: String,
    pub scope_ref: String,
    pub idempotency_key_hash: String,
    pub expected_version_hash: String,
    pub current_version_hash: String,
    pub directory_managed: bool,
    pub final_privileged_subject: bool,
    pub elevated_role: bool,
    pub approval_ref: Option<String>,
    pub policy_allowed: bool,
    pub entitlement_available: bool,
    pub provider_supported: bool,
    pub host_capability_enabled: bool,
    pub reserved_units: BTreeMap<String, u64>,
    pub required_units: BTreeMap<String, u64>,
    pub replay_ref: String,
}

/// Typed pre-provider result that never exposes provider payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrganizationMutationDecision {
    Allowed,
    ApprovalRequired,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    StaleVersion,
    QuotaExceeded,
}

/// Immutable memento describing a rejected competing mutation for replay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizationMutationDiagnosticV1 {
    pub decision: OrganizationMutationDecision,
    pub reason_code: String,
    pub replay_ref: String,
}

/// State Specification for organization, membership, invitation, and role transitions.
pub struct OrganizationLifecycleSpec;
impl OrganizationLifecycleSpec {
    /// Check allowed organization state transitions without consulting a provider.
    pub fn organization_allows(from: OrganizationLifecycle, to: OrganizationLifecycle) -> bool {
        matches!(
            (from, to),
            (
                OrganizationLifecycle::Planned,
                OrganizationLifecycle::Active
            ) | (
                OrganizationLifecycle::Active,
                OrganizationLifecycle::Archived
            ) | (
                OrganizationLifecycle::Archived,
                OrganizationLifecycle::Restored
            ) | (
                OrganizationLifecycle::Restored,
                OrganizationLifecycle::Active
            ) | (
                OrganizationLifecycle::Planned,
                OrganizationLifecycle::Cancelled
            )
        )
    }

    /// Check membership lifecycle transitions while preserving directory ownership.
    pub fn membership_allows(
        from: OrganizationMembershipLifecycle,
        to: OrganizationMembershipLifecycle,
    ) -> bool {
        matches!(
            (from, to),
            (
                OrganizationMembershipLifecycle::Planned,
                OrganizationMembershipLifecycle::Active
            ) | (
                OrganizationMembershipLifecycle::Active,
                OrganizationMembershipLifecycle::Suspended
            ) | (
                OrganizationMembershipLifecycle::Suspended,
                OrganizationMembershipLifecycle::Active
            ) | (
                OrganizationMembershipLifecycle::Active,
                OrganizationMembershipLifecycle::Removed
            )
        )
    }

    /// Check invitation lifecycle transitions without delivery behavior.
    pub fn invitation_allows(
        from: OrganizationInvitationLifecycle,
        to: OrganizationInvitationLifecycle,
    ) -> bool {
        matches!(
            (from, to),
            (
                OrganizationInvitationLifecycle::Planned,
                OrganizationInvitationLifecycle::Pending
            ) | (
                OrganizationInvitationLifecycle::Pending,
                OrganizationInvitationLifecycle::Accepted
            ) | (
                OrganizationInvitationLifecycle::Pending,
                OrganizationInvitationLifecycle::Revoked
            ) | (
                OrganizationInvitationLifecycle::Pending,
                OrganizationInvitationLifecycle::Expired
            )
        )
    }

    /// Check role-binding transitions while preventing a provider-owned binding mutation.
    pub fn role_binding_allows(
        from: OrganizationRoleBindingLifecycle,
        to: OrganizationRoleBindingLifecycle,
    ) -> bool {
        matches!(
            (from, to),
            (
                OrganizationRoleBindingLifecycle::Planned,
                OrganizationRoleBindingLifecycle::Active
            ) | (
                OrganizationRoleBindingLifecycle::Active,
                OrganizationRoleBindingLifecycle::Removed
            )
        )
    }
}

/// Strategy-free safety specification evaluated before provider dispatch.
pub struct OrganizationMutationSpec;
impl OrganizationMutationSpec {
    /// Apply deterministic policy, availability, concurrency, approval, and budget checks.
    pub fn evaluate(evidence: &OrganizationMutationEvidenceV1) -> OrganizationMutationDiagnosticV1 {
        let decision = if !bounded(&evidence.command)
            || !bounded(&evidence.scope_ref)
            || !bounded(&evidence.replay_ref)
        {
            OrganizationMutationDecision::Denied
        } else if !evidence.policy_allowed {
            OrganizationMutationDecision::Denied
        } else if !evidence.entitlement_available || !evidence.host_capability_enabled {
            OrganizationMutationDecision::Unavailable
        } else if !evidence.provider_supported {
            OrganizationMutationDecision::Unsupported
        } else if !covers(&evidence.reserved_units, &evidence.required_units) {
            OrganizationMutationDecision::QuotaExceeded
        } else if !evidence.expected_version_hash.is_empty()
            && evidence.expected_version_hash != evidence.current_version_hash
        {
            OrganizationMutationDecision::StaleVersion
        } else if is_directory_mutation(&evidence.command) && evidence.directory_managed {
            OrganizationMutationDecision::Conflict
        } else if removes_privileged_subject(&evidence.command) && evidence.final_privileged_subject
        {
            OrganizationMutationDecision::Conflict
        } else if requires_approval(&evidence.command, evidence.elevated_role)
            && !evidence.approval_ref.as_deref().is_some_and(bounded)
        {
            OrganizationMutationDecision::ApprovalRequired
        } else if !bounded(&evidence.idempotency_key_hash) && is_side_effect(&evidence.command) {
            OrganizationMutationDecision::Conflict
        } else {
            OrganizationMutationDecision::Allowed
        };
        OrganizationMutationDiagnosticV1 {
            decision,
            reason_code: reason_code(decision).into(),
            replay_ref: evidence.replay_ref.clone(),
        }
    }

    /// Invoke a provider closure only after all generic safety checks pass.
    pub fn dispatch_after_validation<T>(
        evidence: &OrganizationMutationEvidenceV1,
        dispatch: impl FnOnce() -> T,
    ) -> Result<T, OrganizationMutationDiagnosticV1> {
        let diagnostic = Self::evaluate(evidence);
        if diagnostic.decision == OrganizationMutationDecision::Allowed {
            Ok(dispatch())
        } else {
            Err(diagnostic)
        }
    }
}

/// Filter projections before pagination so hidden subjects affect neither counts nor cursors.
pub fn filtered_organization_page<T: Clone>(
    authorized: &[T],
    cursor: Option<usize>,
    page_size: usize,
) -> (Vec<T>, Option<String>) {
    let start = cursor.unwrap_or(0).min(authorized.len());
    let end = start
        .saturating_add(page_size.clamp(1, 100))
        .min(authorized.len());
    (
        authorized[start..end].to_vec(),
        (end < authorized.len()).then(|| format!("cursor:{end}")),
    )
}

/// Preserve previous role references when a re-evaluated binding is projected.
pub fn preserved_role_history(
    previous: &BTreeSet<String>,
    current: &BTreeSet<String>,
) -> BTreeSet<String> {
    previous.union(current).cloned().collect()
}

fn requires_approval(command: &str, elevated_role: bool) -> bool {
    matches!(
        command,
        "organization.create_invitation"
            | "organization.archive"
            | "organization.restore"
            | "organization.export_audit"
    ) || (command == "organization.request_role_binding" && elevated_role)
        || command == "organization.request_membership_change"
}
fn is_side_effect(command: &str) -> bool {
    !command.starts_with("organization.plan_")
        && !matches!(
            command,
            "organization.get"
                | "organization.search"
                | "organization.list_members"
                | "organization.get_membership"
                | "organization.list_role_bindings"
                | "organization.inspect_provider"
                | "organization.discover_schema"
                | "organization.inspect_directory_links"
                | "organization.inspect_invitation"
                | "organization.get_artifact"
        )
}
fn is_directory_mutation(command: &str) -> bool {
    matches!(
        command,
        "organization.request_membership_change" | "organization.request_role_binding"
    )
}
fn removes_privileged_subject(command: &str) -> bool {
    matches!(
        command,
        "organization.request_membership_change" | "organization.request_role_binding"
    )
}
fn covers(reserved: &BTreeMap<String, u64>, required: &BTreeMap<String, u64>) -> bool {
    required
        .iter()
        .all(|(key, value)| reserved.get(key).unwrap_or(&0) >= value)
}
fn bounded(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 256 && !value.contains('\n')
}
fn reason_code(decision: OrganizationMutationDecision) -> &'static str {
    match decision {
        OrganizationMutationDecision::Allowed => "allowed",
        OrganizationMutationDecision::ApprovalRequired => "approval_required",
        OrganizationMutationDecision::Denied => "policy_denied",
        OrganizationMutationDecision::Unavailable => "provider_unavailable",
        OrganizationMutationDecision::Unsupported => "command_unsupported",
        OrganizationMutationDecision::Conflict => "state_conflict",
        OrganizationMutationDecision::StaleVersion => "stale_version",
        OrganizationMutationDecision::QuotaExceeded => "resource_reservation_insufficient",
    }
}
