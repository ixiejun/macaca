//! Hook timeout and failure Strategy implementations.
//!
//! Policies are deliberately separated from the registry and runner so hosts can
//! replace them later for stricter deployments, test fixtures, or certified
//! plugin store channels without changing hook dispatch code.

use macaca_proto::{
    PluginHookDecision, PluginHookDescriptor, PluginHookFailurePolicy, PluginHookKind,
};

/// Strategy that resolves descriptor timeout metadata into a bounded duration.
pub trait PluginHookTimeoutStrategy: Send + Sync {
    /// Return the effective timeout in milliseconds, or `None` when a trusted
    /// host explicitly disables timeout enforcement.
    fn timeout_millis(&self, descriptor: &PluginHookDescriptor) -> Option<u64>;
}

/// Default timeout strategy used by the in-memory Hook Bus.
pub struct DefaultPluginHookTimeoutStrategy {
    default_timeout_millis: u64,
    max_timeout_millis: u64,
}

impl Default for DefaultPluginHookTimeoutStrategy {
    fn default() -> Self {
        Self {
            default_timeout_millis: 250,
            max_timeout_millis: 5_000,
        }
    }
}

impl PluginHookTimeoutStrategy for DefaultPluginHookTimeoutStrategy {
    fn timeout_millis(&self, descriptor: &PluginHookDescriptor) -> Option<u64> {
        descriptor
            .timeout_policy
            .as_millis(self.default_timeout_millis)
            .map(|value| value.min(self.max_timeout_millis))
    }
}

/// Strategy that maps errors/timeouts/unavailable hosts to service decisions.
pub trait PluginHookFailureStrategy: Send + Sync {
    /// Convert one descriptor failure into the decision returned to the owning
    /// service.  The owning service still interprets that decision through its
    /// normal permission, resource, approval, and entitlement policies.
    fn decision_for_failure(&self, descriptor: &PluginHookDescriptor) -> PluginHookDecision;
}

/// Default failure strategy.
///
/// Observer hooks fail open by default.  Blocking hooks are more conservative:
/// explicit fail-closed blocks, explicit approval requires approval, and all
/// other cases keep the host operation moving with a no-op decision.
pub struct DefaultPluginHookFailureStrategy;

impl PluginHookFailureStrategy for DefaultPluginHookFailureStrategy {
    fn decision_for_failure(&self, descriptor: &PluginHookDescriptor) -> PluginHookDecision {
        match descriptor.failure_policy {
            PluginHookFailurePolicy::FailOpen => PluginHookDecision::Noop,
            PluginHookFailurePolicy::FailClosed => PluginHookDecision::Block,
            PluginHookFailurePolicy::RequireApproval => PluginHookDecision::RequireApproval,
        }
    }
}

/// Return true when an invocation expecting `expected` may use `descriptor`.
pub fn kind_matches(descriptor: &PluginHookDescriptor, expected: Option<&PluginHookKind>) -> bool {
    expected.is_none_or(|expected| &descriptor.kind == expected)
}
