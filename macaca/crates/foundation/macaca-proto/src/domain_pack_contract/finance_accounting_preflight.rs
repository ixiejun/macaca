use serde::{Deserialize, Serialize};

use super::finance_accounting_commands::AccountingResultStatus;
use super::finance_accounting_model::bounded_token;

const ACCOUNTING_READ_COMMANDS: &[&str] = &[
    "accounting.inspect_provider",
    "accounting.list_entities",
    "accounting.inspect_period",
    "accounting.get_chart_of_accounts",
    "accounting.get_account",
    "accounting.list_journal_entries",
    "accounting.get_ledger_entries",
    "accounting.get_artifact_handle",
];

const ACCOUNTING_PLANNING_COMMANDS: &[&str] = &[
    "accounting.plan_account",
    "accounting.plan_journal",
    "accounting.plan_reconciliation",
    "accounting.plan_audit_export",
];

const ACCOUNTING_APPROVAL_COMMANDS: &[&str] = &[
    "accounting.account_request",
    "accounting.post_journal",
    "accounting.import_statement_lines",
    "accounting.reconciliation_request",
    "accounting.audit_export_request",
];

const ACCOUNTING_WRITE_COMMANDS: &[&str] = &[
    "accounting.account_request",
    "accounting.post_journal",
    "accounting.import_statement_lines",
    "accounting.reconciliation_request",
];

const ACCOUNTING_REPORT_COMMANDS: &[&str] = &[
    "accounting.generate_trial_balance",
    "accounting.generate_balance_sheet",
    "accounting.generate_profit_loss",
    "accounting.generate_cash_flow",
];

const ACCOUNTING_EXPORT_COMMANDS: &[&str] = &[
    "accounting.plan_audit_export",
    "accounting.audit_export_request",
    "accounting.get_artifact_handle",
];

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountingPolicyDecision {
    pub decision_ref: String,
    pub allowed: bool,
    pub reason_code: String,
}

impl AccountingPolicyDecision {
    /// Validate that a policy decision is explicit and safe to place in audit logs.
    pub fn is_bounded(&self) -> bool {
        bounded_token(&self.decision_ref, 128) && bounded_token(&self.reason_code, 128)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountingApprovalDecision {
    pub approval_ref: String,
    pub approved: bool,
    pub reason_code: String,
}

impl AccountingApprovalDecision {
    /// Validate approval metadata without embedding approver identity or approval payloads.
    pub fn is_bounded(&self) -> bool {
        bounded_token(&self.approval_ref, 128) && bounded_token(&self.reason_code, 128)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountingEntitlementDecision {
    pub entitlement_ref: String,
    pub provider_access: bool,
    pub write_support: bool,
    pub report_support: bool,
    pub export_support: bool,
    pub entity_access: bool,
    pub reason_code: String,
}

impl AccountingEntitlementDecision {
    /// Validate entitlement evidence before provider selection.
    pub fn is_bounded(&self) -> bool {
        bounded_token(&self.entitlement_ref, 128) && bounded_token(&self.reason_code, 128)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountingConsistencyDecision {
    pub decision_ref: String,
    pub conflict_free: bool,
    pub freshness_current: bool,
    pub reason_code: String,
}

impl AccountingConsistencyDecision {
    /// Validate conflict and freshness evidence without raw ledger rows.
    pub fn is_bounded(&self) -> bool {
        bounded_token(&self.decision_ref, 128) && bounded_token(&self.reason_code, 128)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountingResourceRequirement {
    pub provider_call_units: u64,
    pub report_generation_units: u64,
    pub ledger_page_units: u64,
    pub export_bytes: u64,
    pub retained_artifact_units: u64,
    pub network_quota_units: u64,
    pub storage_bytes: u64,
    pub async_job_slots: u64,
}

impl AccountingResourceRequirement {
    /// Build deterministic resource requirements from the command family.
    pub fn for_command(command_name: &str) -> Self {
        let mut requirement = Self {
            provider_call_units: 1,
            network_quota_units: 1,
            ..Default::default()
        };
        if command_name == "accounting.get_ledger_entries"
            || command_name == "accounting.list_journal_entries"
        {
            requirement.ledger_page_units = 1;
        }
        if is_report_command(command_name) {
            requirement.report_generation_units = 1;
            requirement.async_job_slots = 1;
        }
        if is_export_command(command_name) {
            requirement.export_bytes = 1;
            requirement.retained_artifact_units = 1;
            requirement.storage_bytes = 1;
        }
        requirement
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountingResourceReservation {
    pub reservation_ref: String,
    pub provider_call_units: u64,
    pub report_generation_units: u64,
    pub ledger_page_units: u64,
    pub export_bytes: u64,
    pub retained_artifact_units: u64,
    pub network_quota_units: u64,
    pub storage_bytes: u64,
    pub async_job_slots: u64,
}

impl AccountingResourceReservation {
    /// Validate that resource reservation metadata is bounded.
    pub fn is_bounded(&self) -> bool {
        bounded_token(&self.reservation_ref, 128)
    }

    /// Check whether reserved resources cover the command's declared requirement.
    pub fn covers(&self, requirement: &AccountingResourceRequirement) -> bool {
        self.is_bounded()
            && self.provider_call_units >= requirement.provider_call_units
            && self.report_generation_units >= requirement.report_generation_units
            && self.ledger_page_units >= requirement.ledger_page_units
            && self.export_bytes >= requirement.export_bytes
            && self.retained_artifact_units >= requirement.retained_artifact_units
            && self.network_quota_units >= requirement.network_quota_units
            && self.storage_bytes >= requirement.storage_bytes
            && self.async_job_slots >= requirement.async_job_slots
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountingCommandPreflight {
    pub command_name: String,
    pub policy: AccountingPolicyDecision,
    pub approval: Option<AccountingApprovalDecision>,
    pub resources: AccountingResourceReservation,
    pub entitlement: AccountingEntitlementDecision,
    pub consistency: AccountingConsistencyDecision,
}

impl AccountingCommandPreflight {
    /// Build an allowed preflight fixture for SDK tests and mock providers.
    pub fn allowed(command_name: impl Into<String>) -> Self {
        Self {
            command_name: command_name.into(),
            policy: AccountingPolicyDecision {
                decision_ref: "policy".into(),
                allowed: true,
                reason_code: "allowed".into(),
            },
            approval: Some(AccountingApprovalDecision {
                approval_ref: "approval".into(),
                approved: true,
                reason_code: "approved".into(),
            }),
            resources: AccountingResourceReservation {
                reservation_ref: "reservation".into(),
                provider_call_units: 1,
                report_generation_units: 1,
                ledger_page_units: 1,
                export_bytes: 1,
                retained_artifact_units: 1,
                network_quota_units: 1,
                storage_bytes: 1,
                async_job_slots: 1,
            },
            entitlement: AccountingEntitlementDecision {
                entitlement_ref: "entitlement".into(),
                provider_access: true,
                write_support: true,
                report_support: true,
                export_support: true,
                entity_access: true,
                reason_code: "entitled".into(),
            },
            consistency: AccountingConsistencyDecision {
                decision_ref: "consistency".into(),
                conflict_free: true,
                freshness_current: true,
                reason_code: "current".into(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountingPreflightRejection {
    pub status: AccountingResultStatus,
    pub reason_code: String,
}

/// Specification for pre-provider accounting command checks.
///
/// This object centralizes policy, approval, resource, entitlement, conflict,
/// and freshness requirements so SDK helpers and service providers do not grow
/// divergent execution paths.
#[derive(Debug, Clone, Copy, Default)]
pub struct AccountingCommandPreflightSpec;

impl AccountingCommandPreflightSpec {
    /// Evaluate preflight evidence and return the typed result status a service would return.
    pub fn evaluate(
        &self,
        preflight: &AccountingCommandPreflight,
    ) -> Result<(), AccountingPreflightRejection> {
        let command_name = preflight.command_name.as_str();
        if !is_known_command(command_name) {
            return Err(rejection(
                AccountingResultStatus::Unsupported,
                "unsupported_command",
            ));
        }
        if !preflight.policy.is_bounded() || !preflight.policy.allowed {
            return Err(rejection(AccountingResultStatus::Denied, "policy_denied"));
        }
        if !preflight.entitlement.is_bounded() || !preflight.entitlement.provider_access {
            return Err(rejection(
                AccountingResultStatus::Unavailable,
                "provider_access_unavailable",
            ));
        }
        if !preflight.entitlement.entity_access {
            return Err(rejection(AccountingResultStatus::Denied, "entity_denied"));
        }
        if is_write_command(command_name) && !preflight.entitlement.write_support {
            return Err(rejection(
                AccountingResultStatus::Unsupported,
                "write_support_missing",
            ));
        }
        if is_report_command(command_name) && !preflight.entitlement.report_support {
            return Err(rejection(
                AccountingResultStatus::Unsupported,
                "report_support_missing",
            ));
        }
        if is_export_command(command_name) && !preflight.entitlement.export_support {
            return Err(rejection(
                AccountingResultStatus::Unsupported,
                "export_support_missing",
            ));
        }
        if requires_approval(command_name)
            && !preflight
                .approval
                .as_ref()
                .is_some_and(|approval| approval.is_bounded() && approval.approved)
        {
            return Err(rejection(
                AccountingResultStatus::Denied,
                "approval_required",
            ));
        }
        let requirement = AccountingResourceRequirement::for_command(command_name);
        if !preflight.resources.covers(&requirement) {
            return Err(rejection(
                AccountingResultStatus::QuotaExceeded,
                "resource_reservation_insufficient",
            ));
        }
        if !preflight.consistency.is_bounded() || !preflight.consistency.conflict_free {
            return Err(rejection(
                AccountingResultStatus::Conflict,
                "conflict_detected",
            ));
        }
        if !preflight.consistency.freshness_current {
            return Err(rejection(AccountingResultStatus::StaleData, "stale_data"));
        }
        Ok(())
    }
}

pub fn requires_approval(command_name: &str) -> bool {
    ACCOUNTING_APPROVAL_COMMANDS.contains(&command_name)
}

pub fn is_write_command(command_name: &str) -> bool {
    ACCOUNTING_WRITE_COMMANDS.contains(&command_name)
}

pub fn is_report_command(command_name: &str) -> bool {
    ACCOUNTING_REPORT_COMMANDS.contains(&command_name)
}

pub fn is_export_command(command_name: &str) -> bool {
    ACCOUNTING_EXPORT_COMMANDS.contains(&command_name)
}

fn is_known_command(command_name: &str) -> bool {
    ACCOUNTING_READ_COMMANDS.contains(&command_name)
        || ACCOUNTING_PLANNING_COMMANDS.contains(&command_name)
        || ACCOUNTING_APPROVAL_COMMANDS.contains(&command_name)
        || ACCOUNTING_REPORT_COMMANDS.contains(&command_name)
}

fn rejection(status: AccountingResultStatus, reason_code: &str) -> AccountingPreflightRejection {
    AccountingPreflightRejection {
        status,
        reason_code: reason_code.into(),
    }
}
