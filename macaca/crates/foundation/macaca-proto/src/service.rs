//! Protocol-level system service contracts for Macaca Agent OS.
//!
//! These contracts describe replaceable OS capabilities without binding the
//! kernel to concrete providers, presentation shells, or application-specific
//! workflows.  Service descriptors are data-only so they can cross process,
//! plugin, IPC, WASM ABI, and future package boundaries.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{CapabilityId, KernelPrimitiveError, KernelServiceId, ServiceScope, TraceContext};

/// Extensible service classification.
///
/// This is intentionally a string-backed value object rather than a closed
/// enum.  Third-party services must be able to declare new service types
/// without requiring a kernel source edit.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ServiceType(String);

impl ServiceType {
    /// Create a service type after trimming surrounding whitespace.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into().trim().to_string())
    }

    /// Return the raw service type string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ServiceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Named service command.
///
/// The command name is also extensible because service-specific verbs will be
/// introduced by plugins and optional modules over time.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ServiceCommandName(String);

impl ServiceCommandName {
    /// Create a command name after trimming surrounding whitespace.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into().trim().to_string())
    }

    /// Return the raw command name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ServiceCommandName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Reference to the trace schema a service emits.
///
/// Schema references let trace viewers and audit processors evolve without
/// hardcoding service-specific event payloads inside the kernel.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TraceSchemaRef(String);

impl TraceSchemaRef {
    /// Create a trace schema reference after trimming surrounding whitespace.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into().trim().to_string())
    }

    /// Return the raw schema reference.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TraceSchemaRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Capability advertised by a system service.
///
/// The capability references Phase 01 `CapabilityId` so service discovery and
/// policy can share one vocabulary.  Required permissions stay as data because
/// permission backends will be implemented by later phases.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceCapability {
    pub id: CapabilityId,
    pub description: String,
    pub required_permissions: Vec<String>,
}

impl ServiceCapability {
    /// Build a capability descriptor with no required permissions.
    pub fn new(id: CapabilityId, description: impl Into<String>) -> Self {
        Self {
            id,
            description: description.into(),
            required_permissions: Vec::new(),
        }
    }
}

/// Explicit service lifecycle state.
///
/// The state model mirrors Route C's install/register/authorize/start/call/
/// trace/stop/cleanup path.  It is intentionally generic so every service
/// category can share one lifecycle contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceLifecycleState {
    Installed,
    Registered,
    Authorized,
    Starting,
    Running,
    Calling,
    Stopping,
    Stopped,
    CleaningUp,
    CleanedUp,
    Failed { reason: String },
}

/// Health reported by a system service.
///
/// Optional module absence, policy disablement, and degraded operation are
/// represented as data rather than panics or hangs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceHealth {
    Healthy,
    Degraded { reason: String },
    Unavailable { reason: String },
    DisabledByPolicy { reason: String },
    Unknown,
}

/// Cleanup behavior requested by a service descriptor or call result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CleanupPolicy {
    None,
    OnStop,
    OnFailure,
    Always,
    Custom(String),
}

/// Provider-neutral descriptor exported by every system service.
///
/// Descriptors describe what a service can do and what trace/policy shape it
/// requires.  They do not contain provider credentials or application-specific
/// routing rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceDescriptor {
    pub id: KernelServiceId,
    pub service_type: ServiceType,
    pub capabilities: Vec<ServiceCapability>,
    pub lifecycle_state: ServiceLifecycleState,
    pub health: ServiceHealth,
    pub required_permissions: Vec<String>,
    pub supported_scopes: Vec<ServiceScope>,
    pub trace_schema: TraceSchemaRef,
    pub cleanup_policy: CleanupPolicy,
    pub metadata: BTreeMap<String, String>,
}

impl ServiceDescriptor {
    /// Build a minimal descriptor for adapter skeletons and tests.
    pub fn new(
        id: KernelServiceId,
        service_type: ServiceType,
        trace_schema: TraceSchemaRef,
    ) -> Self {
        Self {
            id,
            service_type,
            capabilities: Vec::new(),
            lifecycle_state: ServiceLifecycleState::Registered,
            health: ServiceHealth::Unknown,
            required_permissions: Vec::new(),
            supported_scopes: vec![ServiceScope::Global],
            trace_schema,
            cleanup_policy: CleanupPolicy::None,
            metadata: BTreeMap::new(),
        }
    }
}

/// Command-style invocation of a system service.
///
/// Every command carries optional trace context at the protocol layer so
/// deserialization can represent invalid input.  Kernel call executors must
/// reject commands where `trace` is missing before dispatching to service code.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServiceCommand {
    pub name: ServiceCommandName,
    pub payload: serde_json::Value,
    pub trace: Option<TraceContext>,
    pub metadata: BTreeMap<String, String>,
}

impl ServiceCommand {
    /// Create a command with no trace context.
    ///
    /// This constructor is useful for tests that prove the kernel rejects
    /// untraceable calls before service dispatch.
    pub fn without_trace(name: ServiceCommandName, payload: serde_json::Value) -> Self {
        Self {
            name,
            payload,
            trace: None,
            metadata: BTreeMap::new(),
        }
    }

    /// Create a command with trace context for valid service calls.
    pub fn with_trace(
        name: ServiceCommandName,
        payload: serde_json::Value,
        trace: TraceContext,
    ) -> Self {
        Self {
            name,
            payload,
            trace: Some(trace),
            metadata: BTreeMap::new(),
        }
    }
}

/// Structured result returned by a service call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServiceCallResult {
    pub output: serde_json::Value,
    pub trace: TraceContext,
    pub status: String,
    pub metadata: BTreeMap<String, String>,
    pub cleanup_hint: Option<CleanupPolicy>,
}

/// Structured errors used at the system service boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum ServiceError {
    #[error("missing trace context")]
    MissingTraceContext,

    #[error("invalid lifecycle transition: {from:?} -> {to:?}")]
    InvalidLifecycleTransition {
        from: ServiceLifecycleState,
        to: ServiceLifecycleState,
    },

    #[error("service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("service disabled by policy: {0}")]
    DisabledByPolicy(String),

    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    #[error("unsupported command: {0}")]
    UnsupportedCommand(String),

    #[error("health check failed: {0}")]
    HealthCheckFailed(String),

    #[error("adapter failure: {0}")]
    AdapterFailure(String),

    #[error("kernel primitive error: {0}")]
    KernelPrimitive(String),
}

impl From<KernelPrimitiveError> for ServiceError {
    fn from(value: KernelPrimitiveError) -> Self {
        Self::KernelPrimitive(value.to_string())
    }
}

/// Result alias for system service contract operations.
pub type ServiceResult<T> = Result<T, ServiceError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_descriptor_round_trips() {
        let mut descriptor = ServiceDescriptor::new(
            KernelServiceId::new("service.test"),
            ServiceType::new("custom.third.party"),
            TraceSchemaRef::new("trace.service.test.v1"),
        );
        descriptor.capabilities.push(ServiceCapability::new(
            CapabilityId::new("capability.test"),
            "Test capability",
        ));
        descriptor.health = ServiceHealth::Healthy;
        descriptor.lifecycle_state = ServiceLifecycleState::Running;
        descriptor.cleanup_policy = CleanupPolicy::OnStop;

        let encoded = serde_json::to_string(&descriptor).unwrap();
        let decoded: ServiceDescriptor = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded, descriptor);
        assert_eq!(decoded.service_type.as_str(), "custom.third.party");
    }

    #[test]
    fn service_command_round_trips_with_trace() {
        let command = ServiceCommand::with_trace(
            ServiceCommandName::new("call"),
            serde_json::json!({ "input": true }),
            TraceContext::new("trace-service"),
        );

        let encoded = serde_json::to_vec(&command).unwrap();
        let decoded: ServiceCommand = serde_json::from_slice(&encoded).unwrap();

        assert_eq!(decoded.name.as_str(), "call");
        assert!(decoded.trace.is_some());
        assert_eq!(decoded.payload["input"], true);
    }

    #[test]
    fn service_error_round_trips() {
        let err = ServiceError::UnsupportedCommand("unknown".into());
        let encoded = serde_json::to_string(&err).unwrap();
        let decoded: ServiceError = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded, err);
    }
}
