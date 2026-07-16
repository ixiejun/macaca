use serde::{Deserialize, Serialize};

use super::finance_accounting_model::{bounded_token, is_iso_currency_code, AccountingDimension};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountingFreshness {
    pub source_timestamp_epoch_ms: u64,
    pub cache_timestamp_epoch_ms: Option<u64>,
    pub freshness_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountingAttribution {
    pub source_ref: String,
    pub license_class: String,
    pub required_display_ref: Option<String>,
}

/// Bounded pagination metadata for report-producing accounting commands.
///
/// The DTO records cursor state and truncation without embedding raw ledger rows,
/// provider-native pages, or unbounded report output in traces or snapshots.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountingPaginationMetadata {
    pub next_cursor: Option<String>,
    pub page_size: u32,
    pub truncated: bool,
}

impl AccountingPaginationMetadata {
    /// Bound cursor metadata before it enters traces, snapshots, or SDK diagnostics.
    pub fn is_bounded(&self, max_page_size: u32, max_cursor_len: usize) -> bool {
        self.page_size > 0
            && self.page_size <= max_page_size
            && self
                .next_cursor
                .as_ref()
                .is_none_or(|cursor| cursor.len() <= max_cursor_len && !cursor.contains('\n'))
    }
}

/// Provider-neutral asynchronous report metadata.
///
/// Long-running reports return opaque job and artifact references so callers can
/// resume through the service runtime without learning provider-native job ids.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountingAsyncMetadata {
    pub job_ref: String,
    pub state: String,
    pub submitted_at_epoch_ms: u64,
    pub result_artifact_ref: Option<String>,
    pub replay_pointer: String,
}

impl AccountingAsyncMetadata {
    /// Ensure async report metadata is resumable without exposing provider-native job payloads.
    pub fn is_replayable(&self, max_ref_len: usize) -> bool {
        bounded_token(&self.job_ref, max_ref_len)
            && bounded_token(&self.state, max_ref_len)
            && bounded_token(&self.replay_pointer, max_ref_len)
            && self
                .result_artifact_ref
                .as_ref()
                .is_none_or(|artifact| bounded_token(artifact, max_ref_len))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountingReportRequest {
    pub request_ref: String,
    pub basis: String,
    pub period_range: String,
    pub dimensions: Vec<AccountingDimension>,
    pub currency: String,
    pub pagination: AccountingPaginationMetadata,
    pub async_metadata: Option<AccountingAsyncMetadata>,
}

impl AccountingReportRequest {
    /// Validate report request bounds before a report provider or async job is selected.
    pub fn is_bounded(&self, max_dimensions: usize, max_page_size: u32) -> bool {
        bounded_token(&self.request_ref, 128)
            && bounded_token(&self.basis, 64)
            && bounded_token(&self.period_range, 128)
            && is_iso_currency_code(&self.currency)
            && self.dimensions.len() <= max_dimensions
            && self.pagination.is_bounded(max_page_size, 256)
            && self
                .async_metadata
                .as_ref()
                .is_none_or(|metadata| metadata.is_replayable(256))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrialBalanceReport {
    pub report_ref: String,
    pub rows: Vec<AccountingReportLine>,
    pub basis: String,
    pub pagination: AccountingPaginationMetadata,
    pub async_metadata: Option<AccountingAsyncMetadata>,
    pub freshness: AccountingFreshness,
    pub attribution: AccountingAttribution,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BalanceSheetReport {
    pub report_ref: String,
    pub rows: Vec<AccountingReportLine>,
    pub basis: String,
    pub pagination: AccountingPaginationMetadata,
    pub async_metadata: Option<AccountingAsyncMetadata>,
    pub freshness: AccountingFreshness,
    pub attribution: AccountingAttribution,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfitLossReport {
    pub report_ref: String,
    pub rows: Vec<AccountingReportLine>,
    pub basis: String,
    pub pagination: AccountingPaginationMetadata,
    pub async_metadata: Option<AccountingAsyncMetadata>,
    pub freshness: AccountingFreshness,
    pub attribution: AccountingAttribution,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CashFlowReport {
    pub report_ref: String,
    pub rows: Vec<AccountingReportLine>,
    pub basis: String,
    pub pagination: AccountingPaginationMetadata,
    pub async_metadata: Option<AccountingAsyncMetadata>,
    pub freshness: AccountingFreshness,
    pub attribution: AccountingAttribution,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountingReportLine {
    pub line_ref: String,
    pub label_ref: String,
    pub amount_micros: i64,
}
