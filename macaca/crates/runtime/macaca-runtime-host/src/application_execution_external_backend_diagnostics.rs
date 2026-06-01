//! Heartbeat diagnostics for external application backend providers.
//!
//! This module keeps timeout classification separate from transport delivery.
//! The provider owns leases, but this helper owns the State/Memento conversion:
//! active leases become bounded health and snapshot diagnostics without reading
//! backend-private memory, raw provider payloads, or application workflow data.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use macaca_proto::{
    ApplicationExecutionLifecycleState, ApplicationExecutionProviderHealth,
    ApplicationExecutionProviderKind, ApplicationExecutionProviderLease,
    ApplicationExecutionSnapshot,
};

/// Provider-neutral diagnosis for the current external backend lease set.
pub(crate) struct ExternalBackendHeartbeatDiagnostics {
    now: DateTime<Utc>,
    heartbeat_required: bool,
    leases: Vec<ApplicationExecutionProviderLease>,
}

impl ExternalBackendHeartbeatDiagnostics {
    /// Create diagnostics from a point-in-time lease snapshot.
    ///
    /// The caller supplies `now` to make tests deterministic and to ensure all
    /// health/snapshot fields in one evaluation describe the same observation.
    pub(crate) fn new(
        now: DateTime<Utc>,
        heartbeat_required: bool,
        leases: Vec<ApplicationExecutionProviderLease>,
    ) -> Self {
        Self {
            now,
            heartbeat_required,
            leases,
        }
    }

    /// Convert heartbeat deadline state into provider health.
    ///
    /// A required heartbeat makes a stale active lease unavailable because the
    /// backend can no longer be trusted as a live execution authority. Optional
    /// heartbeats degrade health instead, preserving future providers that can
    /// operate without heartbeat enforcement.
    pub(crate) fn health(&self) -> ApplicationExecutionProviderHealth {
        let stale_count = self.stale_count();
        if stale_count == 0 {
            return ApplicationExecutionProviderHealth::Healthy;
        }
        let reason = format!(
            "external backend heartbeat deadline expired for {stale_count} active lease(s)"
        );
        if self.heartbeat_required {
            ApplicationExecutionProviderHealth::Unavailable { reason }
        } else {
            ApplicationExecutionProviderHealth::Degraded { reason }
        }
    }

    /// Build a bounded provider snapshot for the most urgent active lease.
    ///
    /// The returned Memento contains only protocol identifiers, timestamps, and
    /// status labels. `failure_event_required=true` tells the service layer that
    /// observing this stale provider should be paired with a structured
    /// `ExecutionFailed` event append when an EventLog boundary is available.
    pub(crate) fn snapshot(&self, provider_id: &str) -> Option<ApplicationExecutionSnapshot> {
        let lease = self
            .leases
            .iter()
            .min_by_key(|lease| lease.heartbeat_deadline)?;
        let stale = lease.heartbeat_deadline <= self.now;
        let mut metadata = BTreeMap::new();
        metadata.insert("lease_id".into(), lease.lease_id.clone());
        metadata.insert(
            "heartbeat_deadline".into(),
            lease.heartbeat_deadline.to_rfc3339(),
        );
        metadata.insert("lease_expires_at".into(), lease.expires_at.to_rfc3339());
        metadata.insert(
            "heartbeat_status".into(),
            if stale { "stale" } else { "healthy" }.into(),
        );
        metadata.insert(
            "heartbeat_required".into(),
            self.heartbeat_required.to_string(),
        );
        metadata.insert("stale_lease_count".into(), self.stale_count().to_string());
        metadata.insert(
            "failure_event_required".into(),
            (stale && self.heartbeat_required).to_string(),
        );
        metadata.insert("failure_event_type".into(), "ExecutionFailed".into());
        Some(ApplicationExecutionSnapshot {
            scope: lease.scope.clone(),
            lifecycle_state: if stale && self.heartbeat_required {
                ApplicationExecutionLifecycleState::Failed
            } else {
                ApplicationExecutionLifecycleState::Running
            },
            provider_id: Some(provider_id.into()),
            provider_kind: ApplicationExecutionProviderKind::ExternalAppBackend,
            latest_event_cursor: None,
            latest_checkpoint_ref: None,
            metadata,
        })
    }

    fn stale_count(&self) -> usize {
        self.leases
            .iter()
            .filter(|lease| lease.heartbeat_deadline <= self.now)
            .count()
    }
}
