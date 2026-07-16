use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::model::DomainPackProviderCapabilityState;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountingScope {
    pub tenant_scope: String,
    pub entity_ref: String,
    pub ledger_book_ref: String,
    pub permission_scope: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountingProviderCapability {
    pub provider_class: String,
    pub supported_commands: BTreeSet<String>,
    pub supported_reports: BTreeSet<String>,
    pub write_support: bool,
    pub state: DomainPackProviderCapabilityState,
}

impl AccountingProviderCapability {
    /// Check whether a provider descriptor can accept a mutating accounting command.
    ///
    /// This is descriptor preflight only. Runtime policy, approval, resource, and entitlement
    /// gates still run in service layers before any provider adapter can execute side effects.
    pub fn allows_write_command(&self, command: &str) -> bool {
        self.is_usable() && self.write_support && self.supported_commands.contains(command)
    }

    /// Check whether a provider descriptor advertises a report family without binding a vendor.
    pub fn supports_report(&self, report_family: &str) -> bool {
        self.is_usable() && self.supported_reports.contains(report_family)
    }

    fn is_usable(&self) -> bool {
        !matches!(
            self.state,
            DomainPackProviderCapabilityState::Unavailable
                | DomainPackProviderCapabilityState::Unsupported
                | DomainPackProviderCapabilityState::Retired
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountingRedactionPolicy {
    pub policy_ref: String,
    pub redacted_fields: BTreeSet<String>,
    pub export_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountingEntity {
    pub entity_ref: String,
    pub display_name_ref: String,
    pub base_currency: String,
    pub region_code: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerBook {
    pub book_ref: String,
    pub entity_ref: String,
    pub accounting_basis: String,
    pub base_currency: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountingPeriod {
    pub period_ref: String,
    pub fiscal_year: i32,
    pub fiscal_period: String,
    pub lock: AccountingPeriodLock,
    pub close_state: AccountingPeriodCloseState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountingPeriodLock {
    pub locked: bool,
    pub lock_reason: Option<String>,
    pub locked_at_epoch_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountingPeriodCloseState {
    pub state: String,
    pub closed_by_ref: Option<String>,
    pub closed_at_epoch_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChartOfAccounts {
    pub chart_ref: String,
    pub book_ref: String,
    pub accounts: Vec<AccountHandle>,
    pub concurrency: AccountingConcurrencyToken,
}

impl ChartOfAccounts {
    /// Keep catalog fixtures bounded so tests never embed full provider charts.
    pub fn is_bounded(&self, max_accounts: usize) -> bool {
        !self.accounts.is_empty() && self.accounts.len() <= max_accounts
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountHandle {
    pub account_ref: String,
    pub account_code_ref: String,
    pub display_name_ref: String,
    pub class: AccountClass,
    pub active: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountClass {
    pub class_id: String,
    pub normal_balance: String,
    pub report_family: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountingDimension {
    pub dimension_ref: String,
    pub dimension_kind: String,
    pub value_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountingConcurrencyToken {
    pub token_hash: String,
    pub source_version_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountMutationPlan {
    pub plan_ref: String,
    pub operation: String,
    pub account: AccountHandle,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountMutationResult {
    pub result_state: String,
    pub account_ref: String,
    pub concurrency: AccountingConcurrencyToken,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalEntryPlan {
    pub plan_ref: String,
    pub period_ref: String,
    pub lines: Vec<JournalLine>,
    pub idempotency_key: String,
}

impl JournalEntryPlan {
    /// Double-entry plans must balance aggregate debits and credits before side effects run.
    pub fn balances(&self) -> bool {
        let (debits, credits) = self.lines.iter().fold((0_i64, 0_i64), |acc, line| {
            (acc.0 + line.debit_micros, acc.1 + line.credit_micros)
        });
        debits == credits && debits > 0
    }

    /// Verify debit and credit totals per currency rather than only at aggregate level.
    pub fn balances_by_currency(&self) -> bool {
        let mut totals = BTreeMap::<String, (i64, i64)>::new();
        for line in &self.lines {
            let entry = totals.entry(line.currency.clone()).or_default();
            entry.0 += line.debit_micros;
            entry.1 += line.credit_micros;
        }
        !totals.is_empty()
            && totals
                .values()
                .all(|(debits, credits)| debits == credits && *debits > 0)
    }

    /// Require idempotency for all planned accounting mutations.
    pub fn has_idempotency_key(&self, max_len: usize) -> bool {
        bounded_token(&self.idempotency_key, max_len)
    }

    /// Ensure every line carries all required accounting dimensions by kind.
    pub fn has_required_dimensions(&self, required_kinds: &BTreeSet<String>) -> bool {
        self.lines.iter().all(|line| {
            let present = line
                .dimensions
                .iter()
                .map(|dimension| dimension.dimension_kind.as_str())
                .collect::<BTreeSet<_>>();
            required_kinds
                .iter()
                .all(|required| present.contains(required.as_str()))
        })
    }

    /// Reject plans that reference inactive account handles before provider dispatch.
    pub fn references_only_active_accounts(&self, active_accounts: &BTreeSet<String>) -> bool {
        self.lines
            .iter()
            .all(|line| active_accounts.contains(&line.account_ref))
    }

    /// Validate currency-code and tax-code reference shape without provider-specific rules.
    pub fn has_valid_reference_shapes(&self) -> bool {
        self.lines.iter().all(|line| {
            is_iso_currency_code(&line.currency)
                && line.debit_micros >= 0
                && line.credit_micros >= 0
                && line.debit_micros != line.credit_micros
                && line
                    .tax_code
                    .as_ref()
                    .is_none_or(TaxCodeReference::has_valid_shape)
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalEntry {
    pub journal_ref: String,
    pub status: JournalStatusMetadata,
    pub lines: Vec<JournalLine>,
    pub posting: PostingEvidence,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalLine {
    pub line_ref: String,
    pub account_ref: String,
    pub debit_micros: i64,
    pub credit_micros: i64,
    pub currency: String,
    pub dimensions: Vec<AccountingDimension>,
    pub tax_code: Option<TaxCodeReference>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub entry_ref: String,
    pub journal_ref: String,
    pub account_ref: String,
    pub amount_micros: i64,
    pub source: AccountingSourceReference,
    pub reversal: Option<ReversalReference>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReversalReference {
    pub reversal_ref: String,
    pub reversed_entry_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountingSourceReference {
    pub source_ref: String,
    pub source_kind: String,
    pub evidence_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxCodeReference {
    pub tax_code_ref: String,
    pub jurisdiction_ref: String,
}

impl TaxCodeReference {
    /// Keep tax references opaque and bounded; filing and jurisdiction semantics live elsewhere.
    pub fn has_valid_shape(&self) -> bool {
        bounded_token(&self.tax_code_ref, 128) && bounded_token(&self.jurisdiction_ref, 128)
    }
}

impl AccountingRedactionPolicy {
    /// Keep redaction metadata bounded so export traces carry policy references, not raw fields.
    pub fn is_bounded(&self, max_ref_len: usize) -> bool {
        bounded_token(&self.policy_ref, max_ref_len)
            && bounded_token(&self.export_profile, max_ref_len)
            && self
                .redacted_fields
                .iter()
                .all(|field| bounded_token(field, max_ref_len))
    }
}

impl AccountMutationPlan {
    /// Account mutations are side-effect plans and must be idempotent.
    pub fn has_idempotency_key(&self, max_len: usize) -> bool {
        bounded_token(&self.idempotency_key, max_len)
    }
}

impl AccountingPeriod {
    /// Locked or closed periods cannot accept new postings through this pack.
    pub fn allows_posting(&self) -> bool {
        !self.lock.locked && self.close_state.state != "closed"
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostingEvidence {
    pub evidence_ref: String,
    pub posted_at_epoch_ms: u64,
    pub provider_trace_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalStatusMetadata {
    pub state: String,
    pub immutable: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatementLine {
    pub statement_line_ref: String,
    pub amount_micros: i64,
    pub currency: String,
    pub source_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconciliationCandidate {
    pub statement_line_ref: String,
    pub ledger_entry_ref: String,
    pub confidence: ReconciliationConfidence,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconciliationPlan {
    pub plan_ref: String,
    pub candidates: Vec<ReconciliationCandidate>,
    pub conflict_reasons: Vec<ReconciliationConflictReason>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconciliationResult {
    pub result_state: String,
    pub applied_actions: Vec<ReconciliationAppliedAction>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconciliationConflictReason {
    pub code: String,
    pub trace_safe_detail: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconciliationConfidence {
    pub score_millis: u16,
    pub method_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconciliationAppliedAction {
    pub action_ref: String,
    pub evidence_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditExportPlan {
    pub plan_ref: String,
    pub export_format: String,
    pub retention_policy: String,
    pub redaction: AccountingRedactionPolicy,
}

impl AuditExportPlan {
    /// Validate export plans without embedding raw exported records or provider payloads.
    pub fn is_bounded(&self, max_ref_len: usize) -> bool {
        bounded_token(&self.plan_ref, max_ref_len)
            && bounded_token(&self.export_format, max_ref_len)
            && bounded_token(&self.retention_policy, max_ref_len)
            && self.redaction.is_bounded(max_ref_len)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditExportResult {
    pub artifact: AccountingArtifactHandle,
    pub checksum: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountingArtifactHandle {
    pub artifact_id: String,
    pub artifact_kind: String,
    pub access_policy: String,
    pub expires_at_epoch_ms: u64,
}

impl AccountingArtifactHandle {
    /// Artifact handles remain trace-safe by exposing opaque references and access policy only.
    pub fn is_bounded(&self, max_ref_len: usize) -> bool {
        bounded_token(&self.artifact_id, max_ref_len)
            && bounded_token(&self.artifact_kind, max_ref_len)
            && bounded_token(&self.access_policy, max_ref_len)
            && self.expires_at_epoch_ms > 0
    }
}

pub(super) fn bounded_token(value: &str, max_len: usize) -> bool {
    !value.is_empty() && value.len() <= max_len && !value.contains('\n')
}

pub(super) fn is_iso_currency_code(value: &str) -> bool {
    value.len() == 3 && value.chars().all(|ch| ch.is_ascii_uppercase())
}
