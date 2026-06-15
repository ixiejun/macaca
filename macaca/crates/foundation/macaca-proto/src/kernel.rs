//! Protocol-level microkernel primitives for Macaca Agent OS.
//!
//! This module contains data-only contracts that can be shared by kernel,
//! system services, SDK consumers, and future WASM/application ABI layers.
//! The types deliberately avoid depending on runtime crates such as
//! `macaca-web`, provider implementations, or application frameworks.  That
//! keeps the microkernel boundary stable while allowing services and plugins
//! to evolve behind explicit contracts.

use std::collections::BTreeMap;
use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Stable identifier for a system service known to the microkernel.
///
/// The identifier is a value object rather than an enum so third-party
/// services can be registered without changing kernel source code.  Callers
/// should use reverse-DNS or similarly scoped strings when they need global
/// uniqueness.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct KernelServiceId(String);

impl KernelServiceId {
    /// Create a service id after trimming surrounding whitespace.
    ///
    /// Empty identifiers are allowed to be detected by higher-level validators
    /// through `is_empty`; construction stays infallible so descriptors remain
    /// easy to deserialize from manifests and persisted records.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into().trim().to_string())
    }

    /// Return the raw service identifier string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Report whether the identifier carries no usable content.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Display for KernelServiceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Stable identifier for a capability advertised by a service or plugin.
///
/// Capabilities are intentionally provider-neutral.  A capability names what
/// can be requested; policy and service routing decide whether a concrete
/// implementation may handle that request.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CapabilityId(String);

impl CapabilityId {
    /// Create a capability id after trimming surrounding whitespace.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into().trim().to_string())
    }

    /// Return the raw capability identifier string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Report whether the identifier carries no usable content.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Display for CapabilityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Scope that describes where a service can be used.
///
/// Kernel code stores the scope but does not interpret provider-specific
/// semantics.  Later policy and service-bus phases can use the same enum to
/// decide whether a service is global, application-local, session-local, or
/// package-local.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ServiceScope {
    /// A base OS service available to all applications subject to policy.
    Global,
    /// A service bound to one application id.
    Application(String),
    /// A service bound to one session id.
    Session(String),
    /// A service contributed by an installed package.
    Package(String),
}

/// Provider-neutral descriptor for a registered capability.
///
/// Metadata is modeled as a map so future package manifests can add optional
/// fields without requiring a kernel type change.  Required routing semantics
/// must still be expressed through first-class fields such as `id`, `service`,
/// and `scope`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityDescriptor {
    pub id: CapabilityId,
    pub service: KernelServiceId,
    pub scope: ServiceScope,
    pub description: String,
    pub metadata: BTreeMap<String, String>,
}

impl CapabilityDescriptor {
    /// Build a descriptor with empty metadata for simple registrations.
    pub fn new(
        id: CapabilityId,
        service: KernelServiceId,
        scope: ServiceScope,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id,
            service,
            scope,
            description: description.into(),
            metadata: BTreeMap::new(),
        }
    }
}

/// Trace correlation context carried across kernel primitive calls.
///
/// The context is presentation-neutral.  Web SSE, EventLog, CLI output, and
/// future service-bus subscribers can all render the same correlation data
/// without the kernel depending on any of those transport layers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceContext {
    pub trace_id: String,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub agent: Option<String>,
    pub emitted_at: DateTime<Utc>,
}

impl TraceContext {
    /// Create a trace context with the current timestamp and no optional links.
    pub fn new(trace_id: impl Into<String>) -> Self {
        Self {
            trace_id: trace_id.into(),
            session_id: None,
            task_id: None,
            agent: None,
            emitted_at: Utc::now(),
        }
    }
}

/// Policy evaluation request for a capability or service action.
///
/// The request acts as a Specification input: policy strategies evaluate facts
/// in this struct rather than branching on hardcoded application or provider
/// names.  Additional facts can be added through `metadata` as policy phases
/// become stricter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyRequest {
    pub subject: String,
    pub action: String,
    pub capability: Option<CapabilityId>,
    pub service: Option<KernelServiceId>,
    pub resource: Option<ResourceScope>,
    pub trace: Option<TraceContext>,
    pub metadata: BTreeMap<String, String>,
}

impl PolicyRequest {
    /// Create a minimal policy request for one subject and action.
    pub fn new(subject: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            subject: subject.into(),
            action: action.into(),
            capability: None,
            service: None,
            resource: None,
            trace: None,
            metadata: BTreeMap::new(),
        }
    }
}

/// Structured result of a policy evaluation.
///
/// Denial is represented as data so callers can surface it through trace,
/// retry, approval, or UI flows without relying on panics or string parsing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyDecision {
    Allow { reason: String },
    Deny { reason: String },
}

impl PolicyDecision {
    /// Report whether the decision allows the requested action.
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allow { .. })
    }
}

/// Kernel-visible resource scope used by the resource manager facade.
///
/// The enum names resource categories that every application may need.  It
/// does not own the concrete browser, driver process, workspace directory, or
/// storage implementation; those remain service responsibilities.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ResourceScope {
    Workspace(String),
    Browser(String),
    DriverProcess(String),
    Network(String),
    Storage(String),
    Custom { kind: String, name: String },
}

/// Structured primitive errors shared by kernel facade components.
///
/// These errors intentionally describe invariant failures at the primitive
/// boundary.  Provider-specific failures should be adapted into service-level
/// errors and only cross this boundary as structured data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum KernelPrimitiveError {
    #[error("invalid identifier: {0}")]
    InvalidIdentifier(String),

    #[error("capability not found: {0}")]
    CapabilityNotFound(String),

    #[error("service not found: {0}")]
    ServiceNotFound(String),

    #[error("resource already registered: {0:?}")]
    ResourceAlreadyRegistered(ResourceScope),

    #[error("policy denied: {0}")]
    PolicyDenied(String),

    #[error("primitive unavailable: {0}")]
    Unavailable(String),
}

/// Result alias used by microkernel primitive contracts.
pub type KernelPrimitiveResult<T> = Result<T, KernelPrimitiveError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_descriptor_round_trips_through_json() {
        let mut descriptor = CapabilityDescriptor::new(
            CapabilityId::new("capability.search"),
            KernelServiceId::new("service.research"),
            ServiceScope::Global,
            "Search capability advertised by a service.",
        );
        descriptor
            .metadata
            .insert("version".to_string(), "1".to_string());

        let encoded = serde_json::to_string(&descriptor).unwrap();
        let decoded: CapabilityDescriptor = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded, descriptor);
    }

    #[test]
    fn policy_request_round_trips_with_resource_scope() {
        let mut request = PolicyRequest::new("app:test", "resource.lock");
        request.resource = Some(ResourceScope::Workspace("shared".into()));
        request.trace = Some(TraceContext::new("trace-1"));

        let encoded = serde_json::to_vec(&request).unwrap();
        let decoded: PolicyRequest = serde_json::from_slice(&encoded).unwrap();

        assert_eq!(decoded.subject, "app:test");
        assert_eq!(
            decoded.resource,
            Some(ResourceScope::Workspace("shared".into()))
        );
        assert!(decoded.trace.is_some());
    }

    #[test]
    fn policy_decision_exposes_allow_state() {
        let allow = PolicyDecision::Allow {
            reason: "default_allow".into(),
        };
        let deny = PolicyDecision::Deny {
            reason: "budget exceeded".into(),
        };

        assert!(allow.is_allowed());
        assert!(!deny.is_allowed());
    }
}
