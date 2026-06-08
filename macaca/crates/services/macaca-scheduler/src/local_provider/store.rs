//! In-memory memento store for the in-process Scheduler provider.
//!
//! The store follows the Memento pattern: jobs and runs are provider-owned state
//! that can later be replaced by a durable repository without changing public
//! Scheduler DTOs.  `InMemorySchedulerStore` wraps the state in an `RwLock` so
//! async service methods can share one provider instance safely.

use std::collections::BTreeMap;
use std::sync::RwLock;

use chrono::{DateTime, Utc};
use macaca_proto::{
    SchedulerJobDefinition, SchedulerJobId, SchedulerJobSummary, SchedulerRunId,
    SchedulerRunSummary,
};

use super::support::AUDIT_RETENTION_LIMIT;

/// Thread-safe wrapper around [`LocalSchedulerState`].
#[derive(Default)]
pub(super) struct InMemorySchedulerStore {
    state: RwLock<LocalSchedulerState>,
}

impl InMemorySchedulerStore {
    /// Execute a read-only closure against the locked memento state.
    pub(super) fn read<R>(&self, f: impl FnOnce(&LocalSchedulerState) -> R) -> R {
        let guard = self
            .state
            .read()
            .expect("local scheduler state lock poisoned");
        f(&guard)
    }

    /// Execute a mutating closure against the locked memento state.
    pub(super) fn write<R>(&self, f: impl FnOnce(&mut LocalSchedulerState) -> R) -> R {
        let mut guard = self
            .state
            .write()
            .expect("local scheduler state lock poisoned");
        f(&mut guard)
    }
}

/// Provider-owned scheduler memento: jobs, runs, and bounded audit identifiers.
#[derive(Default)]
pub(super) struct LocalSchedulerState {
    pub(super) next_job_sequence: u64,
    pub(super) next_run_sequence: u64,
    next_audit_sequence: u64,
    pub(super) jobs: BTreeMap<SchedulerJobId, StoredJob>,
    pub(super) runs: BTreeMap<SchedulerRunId, StoredRun>,
    pub(super) audit_ids: Vec<String>,
}

impl LocalSchedulerState {
    /// Allocate the next monotonic job identifier for provider-owned registrations.
    pub(super) fn next_job_id(&mut self) -> SchedulerJobId {
        self.next_job_sequence += 1;
        SchedulerJobId::new(format!("job-{}", self.next_job_sequence))
            .expect("generated scheduler job id is always non-empty")
    }

    /// Allocate the next monotonic run identifier for materialized or manual runs.
    pub(super) fn next_run_id(&mut self) -> SchedulerRunId {
        self.next_run_sequence += 1;
        SchedulerRunId::new(format!("run-{}", self.next_run_sequence))
            .expect("generated scheduler run id is always non-empty")
    }

    /// Append a bounded sanitized audit id and return it to the caller.
    ///
    /// The local provider does not own a durable audit sink yet, so it records
    /// only replay-safe correlation identifiers in memory.  This memento is
    /// intentionally payload-free and can later be replaced by a persistent
    /// audit repository without changing the public Scheduler DTOs.
    pub(super) fn record_audit(&mut self, action: &'static str) -> String {
        self.next_audit_sequence += 1;
        let audit_id = format!("audit.scheduler.{action}.{}", self.next_audit_sequence);
        self.audit_ids.push(audit_id.clone());
        if self.audit_ids.len() > AUDIT_RETENTION_LIMIT {
            let overflow = self.audit_ids.len() - AUDIT_RETENTION_LIMIT;
            self.audit_ids.drain(0..overflow);
        }
        audit_id
    }
}

/// Provider-owned job memento including lifecycle timestamps.
pub(super) struct StoredJob {
    pub(super) definition: SchedulerJobDefinition,
    pub(super) created_at: DateTime<Utc>,
    pub(super) updated_at: DateTime<Utc>,
    pub(super) last_scheduled_at: Option<DateTime<Utc>>,
}

impl StoredJob {
    /// Create a new job memento anchored at `now`.
    pub(super) fn new(definition: SchedulerJobDefinition, now: DateTime<Utc>) -> Self {
        Self {
            definition,
            created_at: now,
            updated_at: now,
            last_scheduled_at: None,
        }
    }

    /// Convert provider-owned job state into the sanitized read model used by
    /// SDK and shell clients.
    pub(super) fn summary(
        &self,
        job_id: SchedulerJobId,
        trace_id: Option<String>,
        audit_id: Option<String>,
    ) -> SchedulerJobSummary {
        SchedulerJobSummary {
            job_id,
            scope: self.definition.scope.clone(),
            schedule: self.definition.schedule.clone(),
            target: self.definition.target.clone(),
            lifecycle: self.definition.lifecycle.clone(),
            metadata: self.definition.metadata.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
            last_scheduled_at: self.last_scheduled_at,
            trace_id,
            audit_id,
        }
    }
}

/// Provider-owned run memento wrapping the public run summary DTO.
pub(super) struct StoredRun {
    pub(super) summary: SchedulerRunSummary,
}
