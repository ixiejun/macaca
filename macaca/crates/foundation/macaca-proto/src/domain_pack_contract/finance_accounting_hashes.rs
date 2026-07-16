use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::finance_accounting::{finance_accounting_pack_definition, FINANCE_ACCOUNTING_COMMANDS};
use super::finance_accounting_commands::AccountingResultStatus;
use super::finance_accounting_model::{
    AccountHandle, AccountingArtifactHandle, AccountingConcurrencyToken, AccountingEntity,
    AccountingProviderCapability, AccountingRedactionPolicy, ChartOfAccounts, JournalEntryPlan,
    JournalLine, ReconciliationCandidate, ReconciliationPlan,
};
use super::finance_accounting_reports::{
    AccountingAsyncMetadata, AccountingAttribution, AccountingFreshness,
    AccountingPaginationMetadata, AccountingReportLine, AccountingReportRequest,
    BalanceSheetReport, CashFlowReport, ProfitLossReport, TrialBalanceReport,
};
use super::finance_common::finance_stable_hash;
use super::model::DomainPackProviderCapabilityState;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountingDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub entity_hash: String,
    pub chart_hash: String,
    pub journal_hash: String,
    pub reconciliation_hash: String,
    pub report_request_hash: String,
    pub trial_balance_hash: String,
    pub balance_sheet_hash: String,
    pub profit_loss_hash: String,
    pub cash_flow_hash: String,
    pub artifact_hash: String,
    pub redaction_hash: String,
}

/// Produce stable, trace-safe hashes over synthetic accounting contract DTOs.
///
/// These hashes are compatibility evidence for schema evolution. They deliberately avoid raw
/// ledgers, bank statements, account numbers, tax identifiers, attachments, and provider payloads.
pub fn finance_accounting_descriptor_hashes() -> AccountingDescriptorHashes {
    let pagination = AccountingPaginationMetadata {
        next_cursor: Some("cursor".into()),
        page_size: 100,
        truncated: false,
    };
    let async_metadata = AccountingAsyncMetadata {
        job_ref: "job".into(),
        state: "completed".into(),
        submitted_at_epoch_ms: 1,
        result_artifact_ref: Some("artifact".into()),
        replay_pointer: "replay".into(),
    };
    let freshness = AccountingFreshness {
        source_timestamp_epoch_ms: 1,
        cache_timestamp_epoch_ms: Some(2),
        freshness_class: "current".into(),
    };
    let attribution = AccountingAttribution {
        source_ref: "source".into(),
        license_class: "synthetic".into(),
        required_display_ref: Some("display".into()),
    };
    let report_line = AccountingReportLine {
        line_ref: "line".into(),
        label_ref: "label".into(),
        amount_micros: 100,
    };

    AccountingDescriptorHashes {
        command_schema_hash: accounting_stable_hash(&FINANCE_ACCOUNTING_COMMANDS),
        result_schema_hash: accounting_stable_hash(&AccountingResultStatus::Success),
        descriptor_hash: accounting_stable_hash(&finance_accounting_pack_definition()),
        provider_capability_hash: accounting_stable_hash(&AccountingProviderCapability {
            provider_class: "mock".into(),
            supported_commands: BTreeSet::from(["accounting.inspect_provider".into()]),
            supported_reports: BTreeSet::from(["trial_balance".into()]),
            write_support: false,
            state: DomainPackProviderCapabilityState::Preview,
        }),
        entity_hash: accounting_stable_hash(&AccountingEntity {
            entity_ref: "entity".into(),
            display_name_ref: "display".into(),
            base_currency: "USD".into(),
            region_code: "US".into(),
        }),
        chart_hash: accounting_stable_hash(&ChartOfAccounts {
            chart_ref: "chart".into(),
            book_ref: "book".into(),
            accounts: vec![AccountHandle::default()],
            concurrency: AccountingConcurrencyToken::default(),
        }),
        journal_hash: accounting_stable_hash(&JournalEntryPlan {
            plan_ref: "journal-plan".into(),
            period_ref: "period".into(),
            lines: vec![
                JournalLine {
                    debit_micros: 100,
                    currency: "USD".into(),
                    ..Default::default()
                },
                JournalLine {
                    credit_micros: 100,
                    currency: "USD".into(),
                    ..Default::default()
                },
            ],
            idempotency_key: "idem".into(),
        }),
        reconciliation_hash: accounting_stable_hash(&ReconciliationPlan {
            plan_ref: "reconcile".into(),
            candidates: vec![ReconciliationCandidate::default()],
            conflict_reasons: vec![],
        }),
        report_request_hash: accounting_stable_hash(&AccountingReportRequest {
            request_ref: "report-request".into(),
            basis: "accrual".into(),
            period_range: "2026-Q2".into(),
            dimensions: vec![],
            currency: "USD".into(),
            pagination: pagination.clone(),
            async_metadata: Some(async_metadata.clone()),
        }),
        trial_balance_hash: accounting_stable_hash(&TrialBalanceReport {
            report_ref: "report".into(),
            rows: vec![report_line.clone()],
            basis: "accrual".into(),
            pagination: pagination.clone(),
            async_metadata: Some(async_metadata.clone()),
            freshness: freshness.clone(),
            attribution: attribution.clone(),
        }),
        balance_sheet_hash: accounting_stable_hash(&BalanceSheetReport {
            report_ref: "balance-sheet".into(),
            rows: vec![report_line.clone()],
            basis: "accrual".into(),
            pagination: pagination.clone(),
            async_metadata: Some(async_metadata.clone()),
            freshness: freshness.clone(),
            attribution: attribution.clone(),
        }),
        profit_loss_hash: accounting_stable_hash(&ProfitLossReport {
            report_ref: "profit-loss".into(),
            rows: vec![report_line.clone()],
            basis: "accrual".into(),
            pagination: pagination.clone(),
            async_metadata: Some(async_metadata.clone()),
            freshness: freshness.clone(),
            attribution: attribution.clone(),
        }),
        cash_flow_hash: accounting_stable_hash(&CashFlowReport {
            report_ref: "cash-flow".into(),
            rows: vec![report_line],
            basis: "cash".into(),
            pagination,
            async_metadata: Some(async_metadata),
            freshness,
            attribution,
        }),
        artifact_hash: accounting_stable_hash(&AccountingArtifactHandle::default()),
        redaction_hash: accounting_stable_hash(&AccountingRedactionPolicy::default()),
    }
}

pub fn accounting_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    finance_stable_hash(value)
}
