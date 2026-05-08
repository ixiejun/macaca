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
