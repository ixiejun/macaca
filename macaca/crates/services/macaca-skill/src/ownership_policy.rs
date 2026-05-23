//! Executable ownership policy for self-evolving Skill governance.
//!
//! Ownership rules are a Specification pattern: callers provide sanitized
//! package facts, and the Skill service returns a bounded decision before any
//! curation, evolution, merge, alias, or mutation command can touch governance
//! state or package bytes.  Keeping this logic in `macaca-skill` prevents Web,
//! CLI, frontend, Context, Task, or runtime-host adapters from becoming the
//! semantic owners of package policy.

use serde::{Deserialize, Serialize};

use crate::mutation::SkillPackageOwnershipClass;

/// Service-owned operations that package ownership policy can admit or deny.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillOwnershipOperation {
    PatchActiveContent,
    Archive,
    Restore,
    Supersede,
    Merge,
    Alias,
    Metadata,
    LocalOverlayDraft,
}

impl SkillOwnershipOperation {
    /// Return true when the operation can change active lifecycle state or
    /// package bytes. Alias and metadata are intentionally lower-risk because
    /// paid, encrypted, marketplace, and bundled packages still need observable
    /// governance without allowing content mutation.
    pub fn mutates_active_package_or_lifecycle(&self) -> bool {
        matches!(
            self,
            Self::PatchActiveContent
                | Self::Archive
                | Self::Restore
                | Self::Supersede
                | Self::Merge
        )
    }
}

/// Bounded ownership-policy decision returned to service providers and SDKs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillOwnershipPolicyDecision {
    Allowed,
    RequiresLocalOverlay,
    RequiresApplicationScope,
    RequiresMutationEntitlement,
    RequiresExplicitApproval,
    Denied,
}

impl SkillOwnershipPolicyDecision {
    /// Collapse every non-allowed state into a mutation denial.
    pub fn denied_reason(&self) -> Option<&'static str> {
        match self {
            Self::Allowed => None,
            Self::RequiresLocalOverlay => {
                Some("marketplace skill changes require a local overlay draft")
            }
            Self::RequiresApplicationScope => {
                Some("application-owned skill changes require application-scope policy approval")
            }
            Self::RequiresMutationEntitlement => {
                Some("paid or encrypted skill mutation requires mutation entitlement")
            }
            Self::RequiresExplicitApproval => {
                Some("central or tenant skill mutation requires explicit approval")
            }
            Self::Denied => Some("protected skill package ownership denies active mutation"),
        }
    }
}

/// Sanitized facts used by ownership policy evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillOwnershipPolicyContext {
    pub ownership: SkillPackageOwnershipClass,
    pub operation: SkillOwnershipOperation,
    pub automatic_agent_flow: bool,
    pub application_scope_approved: bool,
    pub mutation_entitlement_granted: bool,
    pub explicit_approval_granted: bool,
}

impl SkillOwnershipPolicyContext {
    /// Build the default context for generic autonomous agent flows.
    pub fn generic_agent_flow(
        ownership: SkillPackageOwnershipClass,
        operation: SkillOwnershipOperation,
    ) -> Self {
        Self {
            ownership,
            operation,
            automatic_agent_flow: true,
            application_scope_approved: false,
            mutation_entitlement_granted: false,
            explicit_approval_granted: false,
        }
    }
}

/// Conservative package ownership policy used by self-evolving Skill commands.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillOwnershipPolicy;

impl SkillOwnershipPolicy {
    /// Evaluate one operation against the package ownership class.
    pub fn evaluate(&self, context: &SkillOwnershipPolicyContext) -> SkillOwnershipPolicyDecision {
        if matches!(
            context.operation,
            SkillOwnershipOperation::Alias | SkillOwnershipOperation::Metadata
        ) {
            return SkillOwnershipPolicyDecision::Allowed;
        }

        match context.ownership {
            SkillPackageOwnershipClass::AgentPrivate => SkillOwnershipPolicyDecision::Allowed,
            SkillPackageOwnershipClass::CentralUser | SkillPackageOwnershipClass::Tenant => {
                if context.automatic_agent_flow
                    && context.operation.mutates_active_package_or_lifecycle()
                    && !context.explicit_approval_granted
                {
                    SkillOwnershipPolicyDecision::RequiresExplicitApproval
                } else {
                    SkillOwnershipPolicyDecision::Allowed
                }
            }
            SkillPackageOwnershipClass::Bundled | SkillPackageOwnershipClass::PluginProvided => {
                if context.operation.mutates_active_package_or_lifecycle() {
                    SkillOwnershipPolicyDecision::Denied
                } else {
                    SkillOwnershipPolicyDecision::Allowed
                }
            }
            SkillPackageOwnershipClass::Marketplace => {
                if matches!(
                    context.operation,
                    SkillOwnershipOperation::LocalOverlayDraft
                ) {
                    SkillOwnershipPolicyDecision::Allowed
                } else if context.operation.mutates_active_package_or_lifecycle() {
                    SkillOwnershipPolicyDecision::RequiresLocalOverlay
                } else {
                    SkillOwnershipPolicyDecision::Allowed
                }
            }
            SkillPackageOwnershipClass::ApplicationOwned => {
                if context.operation.mutates_active_package_or_lifecycle()
                    && !context.application_scope_approved
                {
                    SkillOwnershipPolicyDecision::RequiresApplicationScope
                } else {
                    SkillOwnershipPolicyDecision::Allowed
                }
            }
            SkillPackageOwnershipClass::Paid | SkillPackageOwnershipClass::Encrypted => {
                if context.operation.mutates_active_package_or_lifecycle()
                    && !context.mutation_entitlement_granted
                {
                    SkillOwnershipPolicyDecision::RequiresMutationEntitlement
                } else {
                    SkillOwnershipPolicyDecision::Allowed
                }
            }
        }
    }
}
