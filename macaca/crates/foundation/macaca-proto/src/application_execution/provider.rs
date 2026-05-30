//! Provider descriptors and leases for application execution.
//!
//! This submodule keeps provider strategy metadata separate from command DTOs.
//! It is still provider-neutral: concrete hosted runtimes, external backends,
//! and remote agents adapt into these descriptors instead of leaking transport
//! or business logic into foundation contracts.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::CapabilityId;

use super::{
    ApplicationExecutionControlKind, ApplicationExecutionEventType,
    ApplicationExecutionProviderKind, ApplicationExecutionScope,
};

/// Descriptor exported by every provider strategy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationExecutionProviderDescriptor {
    pub provider_id: String,
    pub provider_kind: ApplicationExecutionProviderKind,
    pub protocol_version: String,
    pub supported_commands: Vec<ApplicationExecutionControlKind>,
    pub supported_events: Vec<ApplicationExecutionEventType>,
    pub checkpoint_support: bool,
    pub heartbeat_policy: ApplicationExecutionHeartbeatPolicy,
    pub control_delivery: String,
    pub capability_declarations: Vec<CapabilityId>,
    pub resource_profile: BTreeMap<String, String>,
    pub transport_kind: String,
    pub health_state: ApplicationExecutionProviderHealth,
}

/// Heartbeat contract used by external backends and remote agents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationExecutionHeartbeatPolicy {
    pub interval_ms: u64,
    pub timeout_ms: u64,
    pub required: bool,
}

/// Provider health is reported as data so absence/degradation is explicit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApplicationExecutionProviderHealth {
    Healthy,
    Degraded { reason: String },
    Unavailable { reason: String },
}

/// Scoped lease for external backend and remote agent callbacks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationExecutionProviderLease {
    pub lease_id: String,
    pub provider_id: String,
    pub scope: ApplicationExecutionScope,
    pub expires_at: DateTime<Utc>,
    pub heartbeat_deadline: DateTime<Utc>,
    pub callback_identity_ref: String,
    pub allowed_event_types: Vec<ApplicationExecutionEventType>,
    pub allowed_controls: Vec<ApplicationExecutionControlKind>,
}
