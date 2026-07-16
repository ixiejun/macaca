use super::finance_accounting_model::bounded_token;
use super::finance_common::{finance_pack_definition, FinancePackDescriptor, FinanceProviderClass};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};
use crate::{ServiceError, ServiceResult};

pub use super::finance_accounting_bounds::*;
pub use super::finance_accounting_commands::*;
pub use super::finance_accounting_hashes::*;
pub use super::finance_accounting_model::*;
pub use super::finance_accounting_preflight::*;
pub use super::finance_accounting_reports::*;

pub const FINANCE_ACCOUNTING_PACK_ID: &str = "pack.finance.accounting.v1";
pub const FINANCE_ACCOUNTING_SERVICE_ID: &str = "service.finance.accounting";
pub const FINANCE_ACCOUNTING_COMMANDS: &[&str] = &[
    "accounting.inspect_provider",
    "accounting.list_entities",
    "accounting.inspect_period",
    "accounting.get_chart_of_accounts",
    "accounting.get_account",
    "accounting.plan_account",
    "accounting.account_request",
    "accounting.plan_journal",
    "accounting.post_journal",
    "accounting.list_journal_entries",
    "accounting.get_ledger_entries",
    "accounting.import_statement_lines",
    "accounting.plan_reconciliation",
    "accounting.reconciliation_request",
    "accounting.generate_trial_balance",
    "accounting.generate_balance_sheet",
    "accounting.generate_profit_loss",
    "accounting.generate_cash_flow",
    "accounting.plan_audit_export",
    "accounting.audit_export_request",
    "accounting.get_artifact_handle",
];

const ACCOUNTING_PERMISSION_SCOPES: &[&str] = &[
    "finance.accounting.read",
    "finance.accounting.write",
    "finance.accounting.reconcile",
    "finance.accounting.report",
    "finance.accounting.audit_export",
];

/// Specification for application-declared accounting permission scopes.
///
/// This validator is intentionally descriptor-only. Admission, policy,
/// entitlement, and approval checks still run later in service layers, but
/// manifests can be rejected early when they request unknown accounting scopes
/// or unbounded accounting entity references.
#[derive(Debug, Clone, Copy, Default)]
pub struct AccountingDeclarationSpec;

impl AccountingDeclarationSpec {
    /// Validate one accounting declaration scope without consulting providers.
    pub fn validate_scope(
        &self,
        scope: &super::finance_accounting_model::AccountingScope,
    ) -> ServiceResult<()> {
        for value in [
            scope.tenant_scope.as_str(),
            scope.entity_ref.as_str(),
            scope.ledger_book_ref.as_str(),
        ] {
            if !bounded_token(value, 128) {
                return Err(ServiceError::InvalidArgument(
                    "accounting declaration references must be bounded tokens".into(),
                ));
            }
        }
        if !ACCOUNTING_PERMISSION_SCOPES.contains(&scope.permission_scope.as_str()) {
            return Err(ServiceError::InvalidArgument(format!(
                "unsupported accounting permission scope `{}`",
                scope.permission_scope
            )));
        }
        Ok(())
    }

    /// Expose the allowlist used by descriptors, SDK docs, and manifest admission.
    pub fn allowed_scopes(&self) -> &'static [&'static str] {
        ACCOUNTING_PERMISSION_SCOPES
    }
}

const ACCOUNTING_LEDGER_METADATA: &[(&str, &str)] = &[
    ("entities", "true"),
    ("chart_of_accounts", "true"),
    ("period_locks", "true"),
    ("journals", "true"),
];
const ACCOUNTING_REPORT_METADATA: &[(&str, &str)] = &[
    ("trial_balance", "true"),
    ("balance_sheet", "true"),
    ("profit_loss", "true"),
    ("cash_flow", "optional"),
];
const ACCOUNTING_WRITE_METADATA: &[(&str, &str)] = &[
    ("account_mutation", "approval_required"),
    ("journal_posting", "approval_required"),
    ("reconciliation", "approval_required"),
    ("idempotency", "required"),
];
const ACCOUNTING_MOCK_METADATA: &[(&str, &str)] = &[
    ("ledger", "synthetic"),
    ("reports", "synthetic"),
    ("callable", "false"),
];
const ACCOUNTING_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("ledger", "false"),
    ("reports", "false"),
    ("write", "false"),
    ("reason", "provider_not_installed"),
];

const ACCOUNTING_PROVIDER_CLASSES: &[FinanceProviderClass<'_>] = &[
    FinanceProviderClass {
        provider_class: "accounting-ledger",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: ACCOUNTING_LEDGER_METADATA,
    },
    FinanceProviderClass {
        provider_class: "accounting-reporting",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: ACCOUNTING_REPORT_METADATA,
    },
    FinanceProviderClass {
        provider_class: "accounting-write",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: ACCOUNTING_WRITE_METADATA,
    },
    FinanceProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: ACCOUNTING_MOCK_METADATA,
    },
    FinanceProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: ACCOUNTING_UNAVAILABLE_METADATA,
    },
];

/// Build the accounting descriptor without binding ERP, bank-feed, payroll, tax, or invoice code.
///
/// The descriptor is the Bridge between app manifests and future provider implementations:
/// proto owns provider-neutral command schemas and runtime-host remains the only approved
/// composition root for concrete accounting adapters.
pub fn finance_accounting_pack_definition() -> DomainPackDefinition {
    finance_pack_definition(FinancePackDescriptor {
        pack_id: FINANCE_ACCOUNTING_PACK_ID,
        child_change_id: "openspec:add-pack-finance-accounting",
        docs_slug: "accounting",
        sdk_slug: "accounting",
        service_id: FINANCE_ACCOUNTING_SERVICE_ID,
        commands: FINANCE_ACCOUNTING_COMMANDS,
        permission_scopes: ACCOUNTING_PERMISSION_SCOPES,
        provider_classes: ACCOUNTING_PROVIDER_CLASSES,
        health_probe: "accounting.inspect_provider",
        unavailable_reason: "finance_accounting_provider_not_installed",
        replay_schema: "finance.accounting.replay.v1",
        data_classification: "regulated_accounting_reference_metadata",
        retention_policy: "ledger_report_reconciliation_and_artifact_metadata_by_reference",
        redaction_policy: "credentials_account_numbers_tax_identifiers_attachments_raw_ledgers_provider_payloads_and_unbounded_reports_redacted",
        timeout_ms: 120_000,
        budget_units: 4,
        examples: &[
            "Declare `pack.finance.accounting.v1` as optional until an accounting provider is installed.",
            "Plan side effects first, require approval, and use posting evidence instead of raw ledger payloads.",
        ],
        migration_notes: &[
            "Accounting commands become callable only after an approved accounting provider registers matching schemas.",
            "Payroll, invoices, payments, tax filing, and application-specific posting workflows remain outside this pack.",
        ],
    })
}
