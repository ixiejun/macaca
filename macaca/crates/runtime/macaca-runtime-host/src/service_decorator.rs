//! Runtime admission decorators for ServiceRuntime calls.
//!
//! Decorators implement a Chain of Responsibility around service bus dispatch.
//! S1 enforces two real requirements: every call must be traceable, and every
//! call must pass a policy Strategy.  Resource, entitlement, and metering hooks
//! are represented as extension points so later phases can add real enforcement
//! without changing provider factories or presentation shells.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{KernelServiceId, ServiceBusSource, ServiceCommand, ServiceDescriptor};

use crate::service_runtime_error::ServiceRuntimeError;

/// Borrowed context passed to runtime decorators before bus dispatch.
///
/// The context exposes service identity, source identity, command data, and the
/// descriptor.  It intentionally does not expose concrete provider internals.
pub struct ServiceRuntimeCallContext<'a> {
    pub service_id: &'a KernelServiceId,
    pub source: &'a ServiceBusSource,
    pub command: &'a ServiceCommand,
    pub descriptor: &'a ServiceDescriptor,
}

/// Decorator boundary for ServiceRuntime admission checks.
#[async_trait]
pub trait ServiceRuntimeDecorator: Send + Sync {
    /// Return a stable audit name for this decorator.
    ///
    /// The value is intentionally short and static because it is emitted into
    /// service runtime trace events before provider dispatch.  Operators can
    /// therefore replay which admission stages ran without serializing
    /// provider-specific state or raw command payloads.
    fn name(&self) -> &'static str;

    /// Validate or observe a call before it reaches the service bus.
    async fn before_dispatch(
        &self,
        context: &ServiceRuntimeCallContext<'_>,
    ) -> Result<(), ServiceRuntimeError>;
}

/// Enforces the Route C "no trace, no call" invariant at runtime admission.
pub struct TraceRequiredRuntimeDecorator;

#[async_trait]
impl ServiceRuntimeDecorator for TraceRequiredRuntimeDecorator {
    fn name(&self) -> &'static str {
        "trace_required"
    }

    async fn before_dispatch(
        &self,
        context: &ServiceRuntimeCallContext<'_>,
    ) -> Result<(), ServiceRuntimeError> {
        if context.command.trace.is_none() {
            tracing::warn!(
                service_id = %context.service_id,
                source = %context.source,
                command = %context.command.name,
                "service runtime call rejected before bus dispatch: missing trace context"
            );
            return Err(ServiceRuntimeError::MissingTraceContext);
        }
        Ok(())
    }
}

/// Structured decision returned by the runtime policy Strategy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceRuntimePolicyDecision {
    Allow,
    Deny { reason: String },
}

/// Replaceable policy Strategy for runtime service calls.
///
/// Real policy engines can evaluate application permissions, plugin
/// permissions, budgets, region rules, entitlement state, and optional-module
/// availability here.  S1 ships deterministic strategies for tests.
#[async_trait]
pub trait ServiceRuntimePolicy: Send + Sync {
    /// Decide whether the call may proceed to the service bus.
    async fn evaluate(
        &self,
        context: &ServiceRuntimeCallContext<'_>,
    ) -> ServiceRuntimePolicyDecision;
}

/// Policy Strategy that allows every call while still making policy explicit.
pub struct AllowAllServiceRuntimePolicy;

#[async_trait]
impl ServiceRuntimePolicy for AllowAllServiceRuntimePolicy {
    async fn evaluate(
        &self,
        _context: &ServiceRuntimeCallContext<'_>,
    ) -> ServiceRuntimePolicyDecision {
        ServiceRuntimePolicyDecision::Allow
    }
}

/// Policy Strategy that denies every call with a stable reason.
pub struct DenyAllServiceRuntimePolicy {
    reason: String,
}

impl DenyAllServiceRuntimePolicy {
    /// Create a deterministic deny policy for tests and diagnostics.
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

#[async_trait]
impl ServiceRuntimePolicy for DenyAllServiceRuntimePolicy {
    async fn evaluate(
        &self,
        _context: &ServiceRuntimeCallContext<'_>,
    ) -> ServiceRuntimePolicyDecision {
        ServiceRuntimePolicyDecision::Deny {
            reason: self.reason.clone(),
        }
    }
}

/// Decorator that applies the configured policy Strategy before dispatch.
pub struct PolicyRuntimeDecorator {
    policy: Arc<dyn ServiceRuntimePolicy>,
}

impl PolicyRuntimeDecorator {
    /// Wrap a policy Strategy in a runtime decorator.
    pub fn new(policy: Arc<dyn ServiceRuntimePolicy>) -> Self {
        Self { policy }
    }
}

#[async_trait]
impl ServiceRuntimeDecorator for PolicyRuntimeDecorator {
    fn name(&self) -> &'static str {
        "policy"
    }

    async fn before_dispatch(
        &self,
        context: &ServiceRuntimeCallContext<'_>,
    ) -> Result<(), ServiceRuntimeError> {
        match self.policy.evaluate(context).await {
            ServiceRuntimePolicyDecision::Allow => {
                tracing::info!(
                    service_id = %context.service_id,
                    source = %context.source,
                    command = %context.command.name,
                    "service runtime policy allowed call"
                );
                Ok(())
            }
            ServiceRuntimePolicyDecision::Deny { reason } => {
                tracing::warn!(
                    service_id = %context.service_id,
                    source = %context.source,
                    command = %context.command.name,
                    reason = %reason,
                    "service runtime policy denied call"
                );
                Err(ServiceRuntimeError::PolicyDenied(reason))
            }
        }
    }
}

/// Extension marker for future resource-lock decorators.
pub trait ResourceRuntimeDecorator: ServiceRuntimeDecorator {}

/// Extension marker for future entitlement decorators.
pub trait EntitlementRuntimeDecorator: ServiceRuntimeDecorator {}

/// Extension marker for future metering decorators.
pub trait MeteringRuntimeDecorator: ServiceRuntimeDecorator {}

/// Resource admission placeholder for future shared runtime resource controls.
///
/// The decorator follows the Decorator pattern even though S1 does not enforce
/// concrete resource locks yet.  Keeping it in the admission chain now gives
/// every service call a stable audit point where later implementations can
/// reserve CPU, memory, session slots, or external handles without changing
/// service providers or presentation shells.
pub struct ResourcePlaceholderRuntimeDecorator;

#[async_trait]
impl ServiceRuntimeDecorator for ResourcePlaceholderRuntimeDecorator {
    fn name(&self) -> &'static str {
        "resource_placeholder"
    }

    async fn before_dispatch(
        &self,
        context: &ServiceRuntimeCallContext<'_>,
    ) -> Result<(), ServiceRuntimeError> {
        tracing::debug!(
            service_id = %context.service_id,
            source = %context.source,
            command = %context.command.name,
            descriptor = %context.descriptor.id,
            "service runtime resource placeholder admitted call"
        );
        Ok(())
    }
}

impl ResourceRuntimeDecorator for ResourcePlaceholderRuntimeDecorator {}

/// Entitlement admission placeholder for future shared entitlement checks.
///
/// This no-op stage is intentionally provider-neutral.  It records that the
/// entitlement extension point was evaluated before side effects while avoiding
/// any application-owned business rules, package names, provider names, or raw
/// command metadata in logs.
pub struct EntitlementPlaceholderRuntimeDecorator;

#[async_trait]
impl ServiceRuntimeDecorator for EntitlementPlaceholderRuntimeDecorator {
    fn name(&self) -> &'static str {
        "entitlement_placeholder"
    }

    async fn before_dispatch(
        &self,
        context: &ServiceRuntimeCallContext<'_>,
    ) -> Result<(), ServiceRuntimeError> {
        tracing::debug!(
            service_id = %context.service_id,
            source = %context.source,
            command = %context.command.name,
            descriptor = %context.descriptor.id,
            "service runtime entitlement placeholder admitted call"
        );
        Ok(())
    }
}

impl EntitlementRuntimeDecorator for EntitlementPlaceholderRuntimeDecorator {}

/// Metering admission placeholder for future usage and budget accounting.
///
/// The placeholder makes metering visible in trace replay before any provider
/// work begins.  Later phases can replace the no-op body with quota accounting
/// or token/runtime-cost metering while preserving the same ServiceRuntime
/// decorator boundary.
pub struct MeteringPlaceholderRuntimeDecorator;

#[async_trait]
impl ServiceRuntimeDecorator for MeteringPlaceholderRuntimeDecorator {
    fn name(&self) -> &'static str {
        "metering_placeholder"
    }

    async fn before_dispatch(
        &self,
        context: &ServiceRuntimeCallContext<'_>,
    ) -> Result<(), ServiceRuntimeError> {
        tracing::debug!(
            service_id = %context.service_id,
            source = %context.source,
            command = %context.command.name,
            descriptor = %context.descriptor.id,
            "service runtime metering placeholder admitted call"
        );
        Ok(())
    }
}

impl MeteringRuntimeDecorator for MeteringPlaceholderRuntimeDecorator {}

/// Audit admission placeholder that marks the final pre-dispatch audit stage.
///
/// Audit is modeled as a decorator so it can run after trace, policy, resource,
/// entitlement, and metering stages but before the service bus performs side
/// effects.  The stage logs only stable identifiers and leaves durable audit
/// persistence to the runtime event sink, keeping the microkernel boundary
/// clean and application-agnostic.
pub struct AuditPlaceholderRuntimeDecorator;

#[async_trait]
impl ServiceRuntimeDecorator for AuditPlaceholderRuntimeDecorator {
    fn name(&self) -> &'static str {
        "audit_placeholder"
    }

    async fn before_dispatch(
        &self,
        context: &ServiceRuntimeCallContext<'_>,
    ) -> Result<(), ServiceRuntimeError> {
        tracing::info!(
            service_id = %context.service_id,
            source = %context.source,
            command = %context.command.name,
            descriptor = %context.descriptor.id,
            "service runtime audit placeholder admitted call"
        );
        Ok(())
    }
}
