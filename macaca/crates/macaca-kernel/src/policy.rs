//! Policy strategy primitives for the microkernel facade.
//!
//! Policy evaluation is modeled as a replaceable strategy.  Phase 01 keeps a
//! permissive default for compatibility, but the API already returns explicit
//! decisions so later phases can enforce permissions, budgets, regional rules,
//! approvals, and entitlement checks without changing callers.

use macaca_proto::{KernelPrimitiveResult, PolicyDecision, PolicyRequest};

/// Strategy interface for evaluating kernel policy requests.
pub trait PolicyEngine: Send + Sync {
    /// Evaluate a policy request and return a structured decision.
    fn evaluate(&self, request: &PolicyRequest) -> KernelPrimitiveResult<PolicyDecision>;
}

/// Compatibility policy used while existing call paths migrate to policy.
///
/// This strategy deliberately allows every request so Phase 01 remains
/// additive and does not break current applications.  Production policy must
/// replace this strategy in later Route C phases.
#[derive(Debug, Default)]
pub struct DefaultAllowPolicyEngine;

impl DefaultAllowPolicyEngine {
    /// Create a default compatibility policy engine.
    pub fn new() -> Self {
        Self
    }
}

impl PolicyEngine for DefaultAllowPolicyEngine {
    fn evaluate(&self, request: &PolicyRequest) -> KernelPrimitiveResult<PolicyDecision> {
        Ok(PolicyDecision::Allow {
            reason: format!(
                "compatibility allow for subject '{}' action '{}'",
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
