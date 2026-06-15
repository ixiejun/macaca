//! Policy strategy primitives for the microkernel facade.
//!
//! Policy evaluation is modeled as a replaceable strategy. The default strategy
//! is intentionally permissive, but the API always returns explicit decisions so
//! stricter permission, budget, regional, approval, and entitlement policies can
//! be installed by the runtime composition root without changing callers.

use macaca_proto::{KernelPrimitiveResult, PolicyDecision, PolicyRequest};

/// Strategy interface for evaluating kernel policy requests.
pub trait PolicyEngine: Send + Sync {
    /// Evaluate a policy request and return a structured decision.
    fn evaluate(&self, request: &PolicyRequest) -> KernelPrimitiveResult<PolicyDecision>;
}

/// Permissive default policy used when no stricter policy provider is installed.
///
/// This strategy deliberately allows every request and returns a structured
/// reason. Production deployments can replace it with a stricter Strategy
/// without moving policy semantics into presentation shells.
#[derive(Debug, Default)]
pub struct DefaultAllowPolicyEngine;

impl DefaultAllowPolicyEngine {
    /// Create a default allow policy engine.
    pub fn new() -> Self {
        Self
    }
}

impl PolicyEngine for DefaultAllowPolicyEngine {
    fn evaluate(&self, request: &PolicyRequest) -> KernelPrimitiveResult<PolicyDecision> {
        Ok(PolicyDecision::Allow {
            reason: format!(
                "default allow for subject '{}' action '{}'",
                request.subject, request.action
            ),
        })
    }
}

/// Test and diagnostics policy that denies every request with one reason.
///
/// Keeping a deny implementation in production code makes it easy for tests
/// and future diagnostics to prove callers handle denial as data.
#[derive(Debug, Clone)]
pub struct StaticDenyPolicyEngine {
    reason: String,
}

impl StaticDenyPolicyEngine {
    /// Create a deny policy with a stable human-readable reason.
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

impl PolicyEngine for StaticDenyPolicyEngine {
    fn evaluate(&self, _request: &PolicyRequest) -> KernelPrimitiveResult<PolicyDecision> {
        Ok(PolicyDecision::Deny {
            reason: self.reason.clone(),
        })
    }
}
