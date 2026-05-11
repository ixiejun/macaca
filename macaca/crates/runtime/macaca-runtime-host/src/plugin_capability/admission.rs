//! Capability-call admission strategies.
//!
//! Admission is intentionally separate from registry lookup so permission and
//! resource policy can grow without turning the registry into an execution
//! router.  The default v1 chain validates trace, ownership, permission hints,
//! and resource hints as descriptor-level preconditions before the unavailable
//! execution skeleton is reached.

use async_trait::async_trait;
use macaca_proto::{PluginCapabilityCallCommand, PluginCapabilityOwnership, PluginError};
use tracing::{info, warn};

/// Immutable facts evaluated by one capability-call admission check.
pub struct CapabilityCallAdmissionContext<'a> {
    pub command: &'a PluginCapabilityCallCommand,
    pub ownership: &'a PluginCapabilityOwnership,
}

/// Structured decision emitted by an admission check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityCallAdmissionDecision {
    pub check: &'static str,
    pub accepted: bool,
    pub reason: Option<String>,
}

impl CapabilityCallAdmissionDecision {
    /// Build an accepted decision.
    pub fn accepted(check: &'static str) -> Self {
        Self {
            check,
            accepted: true,
            reason: None,
        }
    }

    /// Build a rejected decision.
    pub fn rejected(check: &'static str, reason: impl Into<String>) -> Self {
        Self {
            check,
            accepted: false,
            reason: Some(reason.into()),
        }
    }
}

/// Replaceable admission check boundary.
#[async_trait]
pub trait CapabilityCallAdmissionCheck: Send + Sync {
    /// Evaluate one descriptor-level call precondition.
    async fn evaluate(
        &self,
        context: &CapabilityCallAdmissionContext<'_>,
    ) -> CapabilityCallAdmissionDecision;
}

/// Ensures the call target is active and owner-scoped.
#[derive(Debug, Default)]
pub struct CapabilityOwnershipAdmissionCheck;

#[async_trait]
impl CapabilityCallAdmissionCheck for CapabilityOwnershipAdmissionCheck {
    async fn evaluate(
        &self,
        context: &CapabilityCallAdmissionContext<'_>,
    ) -> CapabilityCallAdmissionDecision {
        if !context.ownership.active {
            return CapabilityCallAdmissionDecision::rejected(
                "capability_ownership",
                "capability is inactive",
            );
        }
        if context
            .command
            .plugin_id
            .as_ref()
            .is_some_and(|plugin_id| plugin_id != &context.ownership.plugin_id)
        {
            return CapabilityCallAdmissionDecision::rejected(
                "capability_ownership",
                "capability plugin owner mismatch",
            );
        }
        CapabilityCallAdmissionDecision::accepted("capability_ownership")
    }
}

/// Ensures descriptor-level permission and resource hints are explicit enough
/// for later policy engines to make auditable decisions.
#[derive(Debug, Default)]
pub struct CapabilityHintAdmissionCheck;

#[async_trait]
impl CapabilityCallAdmissionCheck for CapabilityHintAdmissionCheck {
    async fn evaluate(
        &self,
        context: &CapabilityCallAdmissionContext<'_>,
    ) -> CapabilityCallAdmissionDecision {
        if context
            .ownership
            .descriptor
            .permission_hints
            .iter()
            .any(|hint| hint.required && hint.permission.trim().is_empty())
        {
            return CapabilityCallAdmissionDecision::rejected(
                "capability_hints",
                "required permission hint is empty",
            );
        }
        if context
            .ownership
            .descriptor
            .resource_hints
            .iter()
            .any(|hint| hint.required && hint.resource.trim().is_empty())
        {
            return CapabilityCallAdmissionDecision::rejected(
                "capability_hints",
                "required resource hint is empty",
            );
        }
        CapabilityCallAdmissionDecision::accepted("capability_hints")
    }
}

/// Ordered chain of capability-call admission checks.
pub struct CapabilityCallAdmissionChain {
    checks: Vec<Box<dyn CapabilityCallAdmissionCheck>>,
}

impl Default for CapabilityCallAdmissionChain {
    fn default() -> Self {
        Self {
            checks: vec![
                Box::<CapabilityOwnershipAdmissionCheck>::default(),
                Box::<CapabilityHintAdmissionCheck>::default(),
            ],
        }
    }
}

impl CapabilityCallAdmissionChain {
    /// Build the default fail-closed chain.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a custom admission Strategy.
    pub fn with_check(mut self, check: Box<dyn CapabilityCallAdmissionCheck>) -> Self {
        self.checks.push(check);
        self
    }

    /// Authorize a call attempt before execution routing is considered.
    pub async fn authorize(
        &self,
        context: &CapabilityCallAdmissionContext<'_>,
    ) -> Result<(), PluginError> {
        for check in &self.checks {
            let decision = check.evaluate(context).await;
            if decision.accepted {
                info!(
                    check = decision.check,
                    plugin_id = %context.ownership.plugin_id,
                    capability_id = %context.ownership.capability_id,
                    trace_id = %context.command.trace.trace_id,
                    "plugin capability call admission accepted"
                );
                continue;
            }
            let reason = decision
                .reason
                .unwrap_or_else(|| "capability call admission rejected".into());
            warn!(
                check = decision.check,
                plugin_id = %context.ownership.plugin_id,
                capability_id = %context.ownership.capability_id,
                trace_id = %context.command.trace.trace_id,
                reason = %reason,
                "plugin capability call admission rejected"
            );
            return Err(PluginError::Unavailable(reason));
        }
        Ok(())
    }
}
