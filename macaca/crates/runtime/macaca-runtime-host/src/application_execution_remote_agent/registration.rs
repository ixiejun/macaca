//! Registration DTOs for remote application-execution agents.
//!
//! These types describe transport metadata and capability declarations in a
//! provider-neutral way.  Validation happens before registry admission so invalid
//! registrations never occupy selection slots or emit misleading health signals.

use std::collections::BTreeMap;

use macaca_proto::{
    ApplicationExecutionControlKind, ApplicationExecutionEventType,
    ApplicationExecutionHeartbeatPolicy, ApplicationExecutionProviderHealth, CapabilityId,
    ServiceError,
};

/// Remote transport metadata declared by a remote-agent registration.
///
/// The value is intentionally opaque and provider-neutral.  A future plugin,
/// local IPC bridge, HTTP adapter, or message-bus adapter can interpret it
/// inside its transport implementation without changing service semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteAgentTransportMetadata {
    pub transport_kind: String,
    pub endpoint_ref: String,
    pub callback_identity_ref: String,
    pub region: Option<String>,
}

/// Registration DTO for one remote agent worker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteAgentRegistration {
    pub agent_id: String,
    pub protocol_version: String,
    pub transport: RemoteAgentTransportMetadata,
    pub supported_controls: Vec<ApplicationExecutionControlKind>,
    pub supported_events: Vec<ApplicationExecutionEventType>,
    pub heartbeat_policy: ApplicationExecutionHeartbeatPolicy,
    pub checkpoint_support: bool,
    pub capability_declarations: Vec<CapabilityId>,
    pub resource_profile: BTreeMap<String, String>,
    pub health_state: ApplicationExecutionProviderHealth,
    pub max_leases: usize,
    pub tenant_id: Option<String>,
}

impl RemoteAgentRegistration {
    /// Validate admission metadata before the remote agent can receive work.
    ///
    /// Required fields must be non-empty and the agent must declare at least one
    /// supported event type plus a positive lease budget.  Failures surface as
    /// invalid-argument errors so composition roots can audit bad registrations.
    pub fn validate(&self) -> Result<(), ServiceError> {
        if self.agent_id.trim().is_empty()
            || self.protocol_version.trim().is_empty()
            || self.transport.transport_kind.trim().is_empty()
            || self.transport.endpoint_ref.trim().is_empty()
            || self.transport.callback_identity_ref.trim().is_empty()
            || self.supported_events.is_empty()
            || self.max_leases == 0
        {
            return Err(ServiceError::InvalidArgument(
                "remote agent registration is incomplete".into(),
            ));
        }
        Ok(())
    }
}
