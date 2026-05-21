//! In-memory mementos for the local Heartbeat provider.
//!
//! This module owns replay-safe state snapshots for local development and
//! tests. It deliberately stores bounded identifiers and sanitized summaries
//! only; raw command payloads, prompts, provider payloads, manifests, package
//! bytes, credentials, and unbounded output must stay outside this store.

use std::collections::{BTreeMap, VecDeque};
use std::sync::RwLock;

use chrono::{DateTime, Duration, Utc};
use macaca_proto::{
    AutonomyScope, HeartbeatCadencePolicy, HeartbeatGateDecision, HeartbeatProfile,
    HeartbeatProfileId, HeartbeatProfileSummary, HeartbeatRunId, HeartbeatRunSummary,
    HeartbeatScopeIdentity,
};

use super::{DEFAULT_COOLDOWN_MS, DEFAULT_NATIVE_PROFILE_ID, DEFAULT_NATIVE_SCOPE_KEY};

const AUDIT_RETENTION_LIMIT: usize = 128;

#[derive(Default)]
pub(super) struct InMemoryHeartbeatStore {
    state: RwLock<LocalHeartbeatState>,
}

impl InMemoryHeartbeatStore {
    pub(super) fn read<R>(&self, f: impl FnOnce(&LocalHeartbeatState) -> R) -> R {
        let guard = self
            .state
            .read()
            .expect("local heartbeat state lock poisoned");
        f(&guard)
    }

    pub(super) fn write<R>(&self, f: impl FnOnce(&mut LocalHeartbeatState) -> R) -> R {
        let mut guard = self
            .state
            .write()
            .expect("local heartbeat state lock poisoned");
        f(&mut guard)
    }
}

#[derive(Default)]
pub(super) struct LocalHeartbeatState {
    next_run_sequence: u64,
    next_audit_sequence: u64,
    pub(super) runs: BTreeMap<HeartbeatRunId, StoredHeartbeatRun>,
    pub(super) history: VecDeque<HeartbeatRunId>,
    pub(super) pending_by_scope: BTreeMap<String, HeartbeatRunId>,
    pub(super) last_gate_decisions: Vec<HeartbeatGateDecision>,
    pub(super) scheduler_ticks_active: bool,
    pub(super) last_accepted_by_scope: BTreeMap<String, DateTime<Utc>>,
    pub(super) profiles: BTreeMap<HeartbeatProfileId, StoredHeartbeatProfile>,
    pub(super) audit_ids: Vec<String>,
}

impl LocalHeartbeatState {
    pub(super) fn next_run_id(&mut self) -> HeartbeatRunId {
        self.next_run_sequence += 1;
        HeartbeatRunId::new(format!("heartbeat-run-{}", self.next_run_sequence))
            .expect("generated heartbeat run id is always non-empty")
    }

    /// Append a bounded sanitized audit id and return it to the caller.
    ///
    /// The in-memory provider is not a durable audit sink. It records only
    /// payload-free correlation ids so snapshots and run mementos can prove the
    /// audit chain without expanding the public Heartbeat contract.
    pub(super) fn record_audit(&mut self, action: &'static str) -> String {
        self.next_audit_sequence += 1;
        let audit_id = format!("audit.heartbeat.{action}.{}", self.next_audit_sequence);
        self.audit_ids.push(audit_id.clone());
        if self.audit_ids.len() > AUDIT_RETENTION_LIMIT {
            let overflow = self.audit_ids.len() - AUDIT_RETENTION_LIMIT;
            self.audit_ids.drain(0..overflow);
        }
        audit_id
    }
}

pub(super) struct StoredHeartbeatRun {
    pub(super) summary: HeartbeatRunSummary,
}

impl StoredHeartbeatRun {
    pub(super) fn new(summary: HeartbeatRunSummary) -> Self {
        Self { summary }
    }
}

pub(super) struct StoredHeartbeatProfile {
    pub(super) profile: HeartbeatProfile,
    pub(super) last_tick_at: Option<DateTime<Utc>>,
    pub(super) last_run_id: Option<HeartbeatRunId>,
    pub(super) safe_status: String,
}

impl StoredHeartbeatProfile {
    pub(super) fn new(profile: HeartbeatProfile) -> Self {
        Self {
            profile,
            last_tick_at: None,
            last_run_id: None,
            safe_status: "native heartbeat profile registered".into(),
        }
    }

    pub(super) fn due_at(&self, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        if !self.profile.enabled {
            return None;
        }
        let interval = Duration::milliseconds(self.profile.cadence.interval_ms());
        let last_tick_at = self.last_tick_at.or(match self.profile.cadence {
            HeartbeatCadencePolicy::FixedInterval { anchor, .. } => anchor,
        })?;
        let next = last_tick_at + interval;
        (next <= now).then_some(next)
    }

    pub(super) fn summary(&self) -> HeartbeatProfileSummary {
        let next_eligible_at = match self.profile.cadence {
            HeartbeatCadencePolicy::FixedInterval {
                interval_ms,
                anchor,
            } => self
                .last_tick_at
                .or(anchor)
                .map(|last| last + Duration::milliseconds(interval_ms.min(i64::MAX as u64) as i64)),
        };
        HeartbeatProfileSummary {
            profile_id: self.profile.profile_id.clone(),
            scope_key: self.profile.scope_identity.scope_key.clone(),
            enabled: self.profile.enabled,
            next_eligible_at,
            last_tick_at: self.last_tick_at,
            last_run_id: self.last_run_id.clone(),
            safe_status: self.safe_status.clone(),
            metadata: self.profile.metadata.clone(),
        }
    }
}

impl Default for StoredHeartbeatProfile {
    fn default() -> Self {
        Self::new(
            HeartbeatProfile::new(
                HeartbeatProfileId::new(DEFAULT_NATIVE_PROFILE_ID)
                    .expect("default heartbeat profile id is non-empty"),
                HeartbeatScopeIdentity::new(AutonomyScope::global(), DEFAULT_NATIVE_SCOPE_KEY)
                    .expect("default heartbeat scope key is non-empty"),
                HeartbeatCadencePolicy::FixedInterval {
                    interval_ms: DEFAULT_COOLDOWN_MS as u64,
                    anchor: Some(Utc::now()),
                },
            )
            .expect("default heartbeat profile is valid"),
        )
    }
}
