use serde::{Deserialize, Serialize};

use super::finance_accounting_model::{bounded_token, AccountingArtifactHandle, AuditExportPlan};
use super::finance_accounting_reports::{AccountingPaginationMetadata, AccountingReportRequest};

/// Runtime controls required before long-running accounting commands run.
///
/// This value object keeps timeout, cancellation, and replay controls explicit
/// at the contract layer. Provider adapters can map the opaque cancellation
/// reference to native APIs, but OS traces only store bounded references.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountingExecutionControl {
    pub timeout_ms: u64,
    pub cancellation_ref: Option<String>,
    pub replay_pointer: String,
}

impl AccountingExecutionControl {
    /// Validate that timeout and cancellation controls are bounded and replayable.
    pub fn is_bounded(&self, max_timeout_ms: u64, max_ref_len: usize) -> bool {
        self.timeout_ms > 0
            && self.timeout_ms <= max_timeout_ms
            && bounded_token(&self.replay_pointer, max_ref_len)
            && self
                .cancellation_ref
                .as_ref()
                .is_some_and(|cancel| bounded_token(cancel, max_ref_len))
    }
}

/// Bounded-output counters for ledger, report, and export commands.
///
/// The counters are safe to store in trace, audit, and snapshot records because
/// they describe volume only. They never contain rows, exported files, or
/// provider-native payloads.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountingOutputBound {
    pub row_count: u32,
    pub estimated_bytes: u64,
}

impl AccountingOutputBound {
    /// Check that a command result stays inside predeclared row and byte limits.
    pub fn fits(&self, max_rows: u32, max_bytes: u64) -> bool {
        self.row_count <= max_rows && self.estimated_bytes <= max_bytes
    }
}

/// Specification object for accounting command output and execution limits.
///
/// Keeping the limits in one provider-neutral specification avoids multiple
/// execution paths for ledger pages, reports, and exports. Runtime-host services
/// can apply this same object before dispatching to built-in, plugin, remote,
/// mock, or unavailable providers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountingBoundedCommandSpec {
    pub max_page_size: u32,
    pub max_cursor_len: usize,
    pub max_async_ref_len: usize,
    pub max_timeout_ms: u64,
    pub max_output_rows: u32,
    pub max_output_bytes: u64,
    pub max_artifact_ref_len: usize,
}

impl Default for AccountingBoundedCommandSpec {
    fn default() -> Self {
        Self {
            max_page_size: 1_000,
            max_cursor_len: 256,
            max_async_ref_len: 256,
            max_timeout_ms: 120_000,
            max_output_rows: 10_000,
            max_output_bytes: 16 * 1024 * 1024,
            max_artifact_ref_len: 256,
        }
    }
}

impl AccountingBoundedCommandSpec {
    /// Validate ledger-page commands before returning normalized ledger rows.
    pub fn validates_ledger_command(
        &self,
        pagination: &AccountingPaginationMetadata,
        execution: &AccountingExecutionControl,
        output: &AccountingOutputBound,
    ) -> bool {
        pagination.is_bounded(self.max_page_size, self.max_cursor_len)
            && execution.is_bounded(self.max_timeout_ms, self.max_async_ref_len)
            && output.fits(self.max_output_rows, self.max_output_bytes)
    }

    /// Validate report commands, including report pagination and async metadata.
    pub fn validates_report_command(
        &self,
        request: &AccountingReportRequest,
        execution: &AccountingExecutionControl,
        output: &AccountingOutputBound,
    ) -> bool {
        request.is_bounded(16, self.max_page_size)
            && request
                .async_metadata
                .as_ref()
                .is_none_or(|metadata| metadata.is_replayable(self.max_async_ref_len))
            && execution.is_bounded(self.max_timeout_ms, self.max_async_ref_len)
            && output.fits(self.max_output_rows, self.max_output_bytes)
    }

    /// Validate audit export commands without storing exported artifact content.
    pub fn validates_export_command(
        &self,
        plan: &AuditExportPlan,
        artifact: &AccountingArtifactHandle,
        execution: &AccountingExecutionControl,
        output: &AccountingOutputBound,
    ) -> bool {
        plan.is_bounded(self.max_artifact_ref_len)
            && artifact.is_bounded(self.max_artifact_ref_len)
            && execution.is_bounded(self.max_timeout_ms, self.max_async_ref_len)
            && output.fits(self.max_output_rows, self.max_output_bytes)
    }
}
