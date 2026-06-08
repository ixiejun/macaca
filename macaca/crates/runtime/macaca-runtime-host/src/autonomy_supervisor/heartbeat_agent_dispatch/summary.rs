//! Heartbeat agent dispatch outcome — bounded value object for lane logs and tests.
//!
//! Counts and reason codes are intentionally plain types so summaries can be
//! logged, asserted in contract tests, and forwarded across async boundaries
//! without cloning runtime handles.

use std::collections::BTreeMap;

use macaca_proto::HeartbeatRunState;

/// Bounded dispatch summary recorded by HeartbeatLane logs and contract tests.
///
/// Each counter reflects one phase of the manifest-driven dispatch pipeline:
/// queried declarations, enabled entries, successful dispatches, skipped
/// entries (disabled or diagnostic), and failures (query or execution).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct HeartbeatAgentDispatchSummary {
    /// Total declarations returned by the Application Service query.
    pub queried: usize,
    /// Declarations that passed enablement and diagnostic checks.
    pub enabled: usize,
    /// Agent executions successfully handed off and verified.
    pub dispatched: usize,
    /// Declarations skipped (disabled, diagnostics, or no match).
    pub skipped: usize,
    /// Failures during declaration query or agent execution.
    pub failed: usize,
    /// Terminal Heartbeat run state derived from dispatch counters.
    pub completion_state: Option<HeartbeatRunState>,
    /// Stable reason code for audit and downstream lane transitions.
    pub reason_code: Option<String>,
    /// Sanitized metadata stamped during dispatch (evidence keys, counts).
    pub metadata: BTreeMap<String, String>,
}
