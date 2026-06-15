//! Application Service snapshot, unavailable diagnostics, and result aliases.
//!
//! Snapshots follow the Memento pattern: providers capture sanitized runtime
//! state for audit replay without leaking secrets.  Result type aliases keep
//! SDK and provider signatures concise across the service boundary.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{ApplicationHostCommandResult, TraceContext};

use super::app_views::{
    ApplicationHeartbeatAgentView, ApplicationServiceAppView, ApplicationServiceSessionView,
};
use super::constants::APPLICATION_SERVICE_ID;
use super::metadata_views::ApplicationMetadataView;

/// Structured unavailable result used by service providers and null clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationServiceUnavailable {
    pub service_id: String,
    pub operation: String,
    pub reason: String,
    pub trace_id: Option<String>,
    pub captured_at: DateTime<Utc>,
}

impl ApplicationServiceUnavailable {
    /// Create an auditable unavailable result.
    pub fn new(
        operation: impl Into<String>,
        reason: impl Into<String>,
        trace: Option<&TraceContext>,
    ) -> Self {
        Self {
            service_id: APPLICATION_SERVICE_ID.into(),
            operation: operation.into(),
            reason: reason.into(),
            trace_id: trace.map(|trace| trace.trace_id.clone()),
            captured_at: Utc::now(),
        }
    }
}

/// Memento snapshot for Application Service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationServiceSnapshot {
    pub service_id: String,
    pub healthy: bool,
    pub discovered: Vec<ApplicationServiceAppView>,
    pub running: Vec<ApplicationServiceAppView>,
    pub sessions: Vec<ApplicationServiceSessionView>,
    pub diagnostics: Vec<ApplicationServiceUnavailable>,
    pub captured_at: DateTime<Utc>,
}

impl ApplicationServiceSnapshot {
    /// Build a healthy snapshot from sanitized views.
    pub fn healthy(
        discovered: Vec<ApplicationServiceAppView>,
        running: Vec<ApplicationServiceAppView>,
        sessions: Vec<ApplicationServiceSessionView>,
    ) -> Self {
        Self {
            service_id: APPLICATION_SERVICE_ID.into(),
            healthy: true,
            discovered,
            running,
            sessions,
            diagnostics: Vec::new(),
            captured_at: Utc::now(),
        }
    }

    /// Build an unavailable snapshot for hosts without an Application Service.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            service_id: APPLICATION_SERVICE_ID.into(),
            healthy: false,
            discovered: Vec::new(),
            running: Vec::new(),
            sessions: Vec::new(),
            diagnostics: vec![ApplicationServiceUnavailable::new(
                "application.snapshot",
                reason,
                None,
            )],
            captured_at: Utc::now(),
        }
    }
}

/// Result aliases keep SDK/provider signatures concise.
pub type ApplicationDiscoverResult = Vec<ApplicationServiceAppView>;
pub type ApplicationStartResult = ApplicationServiceAppView;
pub type ApplicationStatusResult = Vec<ApplicationServiceAppView>;
pub type ApplicationSessionResult = ApplicationServiceSessionView;
pub type ApplicationHostDispatchResult = ApplicationHostCommandResult;
pub type ApplicationMetadataResult = ApplicationMetadataView;
pub type ApplicationHeartbeatAgentsResult = Vec<ApplicationHeartbeatAgentView>;
