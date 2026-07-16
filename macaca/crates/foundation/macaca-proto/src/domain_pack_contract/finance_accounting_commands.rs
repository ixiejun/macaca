use serde::{Deserialize, Serialize};

use super::finance_common::{
    define_finance_command_wrappers, FinanceCommandEnvelope, FinanceError, FinancePage,
};

define_finance_command_wrappers!(
    AccountingInspectProviderCommand,
    AccountingListEntitiesCommand,
    AccountingInspectPeriodCommand,
    AccountingGetChartOfAccountsCommand,
    AccountingGetAccountCommand,
    AccountingPlanAccountCommand,
    AccountingAccountRequestCommand,
    AccountingPlanJournalCommand,
    AccountingPostJournalCommand,
    AccountingListJournalEntriesCommand,
    AccountingGetLedgerEntriesCommand,
    AccountingImportStatementLinesCommand,
    AccountingPlanReconciliationCommand,
    AccountingReconciliationRequestCommand,
    AccountingGenerateTrialBalanceCommand,
    AccountingGenerateBalanceSheetCommand,
    AccountingGenerateProfitLossCommand,
    AccountingGenerateCashFlowCommand,
    AccountingPlanAuditExportCommand,
    AccountingAuditExportRequestCommand,
    AccountingGetArtifactHandleCommand,
);

/// Provider-neutral accounting outcome taxonomy shared by all accounting commands.
///
/// Providers may add capability-specific diagnostics inside `FinanceError`, but the
/// envelope status remains stable so admission, trace, audit, and SDK callers can
/// reason about unavailable, denied, conflict, quota, and stale-data states uniformly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountingResultStatus {
    Success,
    Partial,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    QuotaExceeded,
    StaleData,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountingResultEnvelope<T> {
    pub status: AccountingResultStatus,
    pub data: Option<T>,
    pub page: Option<FinancePage<T>>,
    pub error: Option<FinanceError>,
}
