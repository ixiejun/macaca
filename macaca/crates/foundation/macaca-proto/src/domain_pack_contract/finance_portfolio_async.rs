//! Asynchronous report lifecycle DTOs for the provider-neutral portfolio contract.

use serde::{Deserialize, Serialize};

/// Provider-neutral state evidence for a bounded asynchronous report request.
///
/// Providers own scheduling and cancellation mechanics. This DTO gives the
/// runtime a stable, trace-safe state projection that can be persisted in an
/// audit snapshot without retaining report content or provider payloads.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioReportJob {
    pub job_ref: String,
    pub request_ref: String,
    pub state: String,
    pub timeout_ms: u64,
    pub cancellation_ref: Option<String>,
    pub report_ref: Option<String>,
    pub replay_pointer: String,
}
