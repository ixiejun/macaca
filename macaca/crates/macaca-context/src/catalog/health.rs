//! Rolling **Observer** state: inexpensive last-known outcomes derived from [`crate::report::ProviderRuntimeSummary`].
//!
//! This never stores prompt bodies — only ids, coarse outcomes, and latencies suitable for `/api` dumps.

use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::report::ProviderRuntimeSummary;

/// One row mirrored from the most recent invocation entry for a `provider_id`.
#[derive(Debug, Clone, Serialize)]
pub struct ProviderHealthSnapshot {
    pub outcome: String,
    pub latency_ms: u64,
    pub last_seen_epoch_ms: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub implementation_version: Option<String>,
}

/// Thread-safe map from `provider_id` to rolling snapshot.
#[derive(Default)]
pub struct ProviderHealthLedger {
    rows: RwLock<HashMap<String, ProviderHealthSnapshot>>,
}

impl ProviderHealthLedger {
    pub fn new() -> Self {
        Self::default()
    }

    fn now_ms() -> u128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    }

    /// Ingests the per-request [`ProviderRuntimeSummary`] emitted by governance.
    pub fn record_provider_runtime_summary(&self, summary: &ProviderRuntimeSummary) {
        let now = Self::now_ms();
        let mut map = self.rows.write().expect("ledger poisoned");
        for row in &summary.invocations {
            map.insert(
                row.provider_id.clone(),
                ProviderHealthSnapshot {
                    outcome: row.outcome.clone(),
                    latency_ms: row.latency_ms,
                    last_seen_epoch_ms: now,
                    implementation_version: row.implementation_version.clone(),
                },
            );
        }
    }

    /// Operator / HTTP diagnostics clone — small map keyed by provider id.
    pub fn snapshot(&self) -> HashMap<String, ProviderHealthSnapshot> {
        self.rows.read().expect("ledger poisoned").clone()
    }
}
