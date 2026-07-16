use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::ai_common::ai_bounded_token;

/// Generic declaration validator shared by AI child packs.
///
/// The validator keeps manifest admission provider-neutral: it checks only
/// bounded scope strings against descriptor-owned allowlists and never chooses
/// a model, provider, tenant workflow, or application behavior.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiPackDeclarationSpec {
    pub allowed_scopes: BTreeSet<String>,
}

impl AiPackDeclarationSpec {
    /// Build a declaration spec from a child-pack descriptor scope list.
    pub fn new(scopes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            allowed_scopes: scopes.into_iter().map(Into::into).collect(),
        }
    }

    /// Validate one requested scope before an application can use an AI pack.
    pub fn validate_scope(&self, scope: &str) -> Result<(), AiPackPreflightRejection> {
        if !ai_bounded_token(scope, 128) {
            return Err(AiPackPreflightRejection::new(
                AiPackPreflightStatus::Denied,
                "scope_unbounded",
            ));
        }
        if !self.allowed_scopes.contains(scope) {
            return Err(AiPackPreflightRejection::new(
                AiPackPreflightStatus::Denied,
                "scope_not_declared",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiPackPreflightStatus {
    Denied,
    Unavailable,
    Unsupported,
    QuotaExceeded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiPackPreflightRejection {
    pub status: AiPackPreflightStatus,
    pub reason_code: String,
}

impl AiPackPreflightRejection {
    /// Create a bounded, trace-safe rejection reason.
    pub fn new(status: AiPackPreflightStatus, reason_code: impl Into<String>) -> Self {
        Self {
            status,
            reason_code: reason_code.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiPackPolicyDecision {
    pub decision_ref: String,
    pub allowed: bool,
    pub reason_code: String,
}

impl AiPackPolicyDecision {
    /// Validate policy evidence without storing raw prompts, media, or provider payloads.
    pub fn is_bounded(&self) -> bool {
        ai_bounded_token(&self.decision_ref, 128) && ai_bounded_token(&self.reason_code, 128)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiPackApprovalDecision {
    pub approval_ref: String,
    pub approved: bool,
    pub reason_code: String,
}

impl AiPackApprovalDecision {
    /// Validate approval evidence for sensitive or long-running AI operations.
    pub fn is_bounded(&self) -> bool {
        ai_bounded_token(&self.approval_ref, 128) && ai_bounded_token(&self.reason_code, 128)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiPackEntitlementDecision {
    pub entitlement_ref: String,
    pub provider_access: bool,
    pub scope_granted: bool,
    pub command_supported: bool,
    pub host_capability_enabled: bool,
    pub reason_code: String,
}

impl AiPackEntitlementDecision {
    /// Validate entitlement evidence before provider dispatch.
    pub fn is_bounded(&self) -> bool {
        ai_bounded_token(&self.entitlement_ref, 128) && ai_bounded_token(&self.reason_code, 128)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiPackResourceUnits {
    pub provider_calls: u64,
    pub token_units: u64,
    pub media_units: u64,
    pub storage_bytes: u64,
    pub retained_output_bytes: u64,
    pub rate_units: u64,
}

impl AiPackResourceUnits {
    /// Check whether reserved resource units cover the declared command requirement.
    pub fn covers(&self, required: &AiPackResourceUnits) -> bool {
        self.provider_calls >= required.provider_calls
            && self.token_units >= required.token_units
            && self.media_units >= required.media_units
            && self.storage_bytes >= required.storage_bytes
            && self.retained_output_bytes >= required.retained_output_bytes
            && self.rate_units >= required.rate_units
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiPackCommandPreflight {
    pub command_name: String,
    pub requested_scope: String,
    pub policy: AiPackPolicyDecision,
    pub approval: Option<AiPackApprovalDecision>,
    pub entitlement: AiPackEntitlementDecision,
    pub required_resources: AiPackResourceUnits,
    pub reserved_resources: AiPackResourceUnits,
}

impl AiPackCommandPreflight {
    /// Build an accepted fixture for contract tests and mock providers.
    pub fn allowed(command_name: impl Into<String>, requested_scope: impl Into<String>) -> Self {
        Self {
            command_name: command_name.into(),
            requested_scope: requested_scope.into(),
            policy: AiPackPolicyDecision {
                decision_ref: "policy".into(),
                allowed: true,
                reason_code: "allowed".into(),
            },
            approval: Some(AiPackApprovalDecision {
                approval_ref: "approval".into(),
                approved: true,
                reason_code: "approved".into(),
            }),
            entitlement: AiPackEntitlementDecision {
                entitlement_ref: "entitlement".into(),
                provider_access: true,
                scope_granted: true,
                command_supported: true,
                host_capability_enabled: true,
                reason_code: "granted".into(),
            },
            required_resources: AiPackResourceUnits {
                provider_calls: 1,
                token_units: 1,
                media_units: 0,
                storage_bytes: 1,
                retained_output_bytes: 1,
                rate_units: 1,
            },
            reserved_resources: AiPackResourceUnits {
                provider_calls: 1,
                token_units: 1,
                media_units: 1,
                storage_bytes: 1,
                retained_output_bytes: 1,
                rate_units: 1,
            },
        }
    }
}

/// Specification object for provider-neutral AI command preflight.
///
/// Runtime services can apply this before model/provider dispatch. It centralizes
/// declaration, policy, approval, entitlement, host-capability, and resource
/// checks so AI child packs do not grow divergent execution paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiPackCommandPreflightSpec {
    pub declaration: AiPackDeclarationSpec,
    pub allowed_commands: BTreeSet<String>,
    pub approval_required_commands: BTreeSet<String>,
}

impl AiPackCommandPreflightSpec {
    /// Build a command preflight spec from descriptor-owned command and scope lists.
    pub fn new(
        commands: impl IntoIterator<Item = impl Into<String>>,
        scopes: impl IntoIterator<Item = impl Into<String>>,
        approval_required_commands: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            declaration: AiPackDeclarationSpec::new(scopes),
            allowed_commands: commands.into_iter().map(Into::into).collect(),
            approval_required_commands: approval_required_commands
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }

    /// Evaluate preflight evidence and return a structured rejection before provider dispatch.
    pub fn evaluate(
        &self,
        preflight: &AiPackCommandPreflight,
    ) -> Result<(), AiPackPreflightRejection> {
        if !ai_bounded_token(&preflight.command_name, 128)
            || !self.allowed_commands.contains(&preflight.command_name)
        {
            return Err(AiPackPreflightRejection::new(
                AiPackPreflightStatus::Unsupported,
                "unsupported_command",
            ));
        }
        self.declaration
            .validate_scope(&preflight.requested_scope)?;
        if !preflight.policy.is_bounded() || !preflight.policy.allowed {
            return Err(AiPackPreflightRejection::new(
                AiPackPreflightStatus::Denied,
                "policy_denied",
            ));
        }
        if !preflight.entitlement.is_bounded() || !preflight.entitlement.provider_access {
            return Err(AiPackPreflightRejection::new(
                AiPackPreflightStatus::Unavailable,
                "provider_unavailable",
            ));
        }
        if !preflight.entitlement.scope_granted {
            return Err(AiPackPreflightRejection::new(
                AiPackPreflightStatus::Denied,
                "entitlement_denied",
            ));
        }
        if !preflight.entitlement.command_supported {
            return Err(AiPackPreflightRejection::new(
                AiPackPreflightStatus::Unsupported,
                "command_not_supported",
            ));
        }
        if !preflight.entitlement.host_capability_enabled {
            return Err(AiPackPreflightRejection::new(
                AiPackPreflightStatus::Unavailable,
                "host_capability_disabled",
            ));
        }
        if self
            .approval_required_commands
            .contains(&preflight.command_name)
            && !preflight
                .approval
                .as_ref()
                .is_some_and(|approval| approval.is_bounded() && approval.approved)
        {
            return Err(AiPackPreflightRejection::new(
                AiPackPreflightStatus::Denied,
                "approval_required",
            ));
        }
        if !preflight
            .reserved_resources
            .covers(&preflight.required_resources)
        {
            return Err(AiPackPreflightRejection::new(
                AiPackPreflightStatus::QuotaExceeded,
                "resource_reservation_insufficient",
            ));
        }
        Ok(())
    }

    /// Execute provider dispatch only after all preflight gates accept the command.
    ///
    /// This Decorator-style helper gives runtime services and mock providers a
    /// single provider-neutral entry point for enforcing declaration, policy,
    /// approval, entitlement, host-capability, and resource checks before any
    /// concrete provider side effect can run. The closure is intentionally
    /// generic so this contract layer never constructs or names a provider.
    pub fn dispatch_after_preflight<T>(
        &self,
        preflight: &AiPackCommandPreflight,
        dispatch: impl FnOnce() -> T,
    ) -> Result<T, AiPackPreflightRejection> {
        self.evaluate(preflight)?;
        Ok(dispatch())
    }
}
