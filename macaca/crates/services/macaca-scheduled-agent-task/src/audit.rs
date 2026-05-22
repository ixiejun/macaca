//! Sanitized audit helpers for the local Scheduled Agent Task provider.
//!
//! The local provider does not yet write to a durable audit database.  It still
//! produces deterministic, replay-safe audit ids and bounded metadata so every
//! important transition can be correlated with Scheduler and Agent Execution
//! evidence.  Raw prompts and raw delegated context are intentionally excluded.

use std::collections::BTreeMap;

/// Bounded audit memento retained by the local provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledAgentTaskAuditRecord {
    pub audit_id: String,
    pub action: String,
    pub trace_id: String,
    pub task_id: Option<String>,
    pub scheduler_job_id: Option<String>,
    pub scheduler_run_id: Option<String>,
    pub payload_digest: Option<String>,
    pub target_agent: Option<String>,
    pub reason_code: String,
    pub metadata: BTreeMap<String, String>,
}

/// Small audit id generator used by the in-memory provider.
#[derive(Debug, Default)]
pub struct LocalAuditRecorder {
    next_sequence: u64,
    records: Vec<ScheduledAgentTaskAuditRecord>,
}

impl LocalAuditRecorder {
    /// Record a sanitized transition and return its audit id.
    pub fn record(&mut self, mut record: ScheduledAgentTaskAuditRecord) -> String {
        self.next_sequence += 1;
        record.audit_id = format!(
            "audit.scheduled_agent_task.{}.{}",
            record.action, self.next_sequence
        );
        let audit_id = record.audit_id.clone();
        self.records.push(record);
        if self.records.len() > 128 {
            let overflow = self.records.len() - 128;
            self.records.drain(0..overflow);
        }
        audit_id
    }

    /// Return recent safe audit ids for snapshots.
    pub fn recent_ids(&self, limit: usize) -> Vec<String> {
        self.records
            .iter()
            .rev()
            .take(limit)
            .map(|record| record.audit_id.clone())
            .collect()
    }
}

impl ScheduledAgentTaskAuditRecord {
    /// Create a sanitized audit record with no raw prompt-bearing fields.
    pub fn new(
        action: impl Into<String>,
        trace_id: impl Into<String>,
        reason_code: impl Into<String>,
    ) -> Self {
        Self {
            audit_id: String::new(),
            action: action.into(),
            trace_id: trace_id.into(),
            task_id: None,
            scheduler_job_id: None,
            scheduler_run_id: None,
            payload_digest: None,
            target_agent: None,
            reason_code: reason_code.into(),
            metadata: BTreeMap::new(),
        }
    }
}
