//! Additive microkernel facade for primitive boundaries.
//!
//! The facade is the narrow entry point that later system services, SDKs, and
//! application runtimes should depend on.  It groups discovery, policy,
//! resources, and trace emission without moving existing orchestration logic
//! into the kernel.

use std::sync::RwLock;

use macaca_proto::{
    CapabilityDescriptor, CapabilityId, KernelPrimitiveError, KernelPrimitiveResult,
    KernelServiceId, PolicyDecision, PolicyRequest, ResourceScope, ServiceScope, TraceContext,
};

use crate::capability_registry::{CapabilityRegistry, InMemoryCapabilityRegistry};
use crate::policy::{DefaultAllowPolicyEngine, PolicyEngine};
use crate::resource::{InMemoryResourceManager, ResourceManager};
use crate::service_registry::{InMemorySystemServiceRegistry, SystemServiceRegistry};

/// Presentation-neutral trace event captured at the kernel primitive boundary.
///
/// The payload is JSON because trace consumers already need schema-flexible
/// metadata.  Transport-specific conversion to EventLog, SSE, CLI, or service
/// bus messages remains outside the kernel.
#[derive(Debug, Clone, PartialEq)]
pub struct KernelTraceEvent {
    pub context: TraceContext,
    pub event_type: String,
    pub payload: serde_json::Value,
}

/// Observer boundary for kernel primitive trace and audit events.
pub trait TraceEventBus: Send + Sync {
    /// Emit one primitive trace event.
    fn emit_trace(&self, event: KernelTraceEvent) -> KernelPrimitiveResult<()>;
}

/// In-memory trace sink used for tests and additive diagnostics.
///
/// This implementation proves the observer boundary without coupling kernel
/// primitives to the Web UI.  Production consumers can replace it with an
/// EventLog or service-bus bridge while preserving the trait.
#[derive(Default)]
pub struct InMemoryTraceEventBus {
    events: RwLock<Vec<KernelTraceEvent>>,
}

impl InMemoryTraceEventBus {
    /// Create an empty trace sink.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return a snapshot of emitted events for diagnostics and tests.
    pub fn events(&self) -> KernelPrimitiveResult<Vec<KernelTraceEvent>> {
        let events = self
            .events
            .read()
            .map_err(|_| KernelPrimitiveError::Unavailable("trace bus lock poisoned".into()))?;
        Ok(events.clone())
    }
}

impl TraceEventBus for InMemoryTraceEventBus {
    fn emit_trace(&self, event: KernelTraceEvent) -> KernelPrimitiveResult<()> {
        let mut events = self
            .events
            .write()
            .map_err(|_| KernelPrimitiveError::Unavailable("trace bus lock poisoned".into()))?;
        events.push(event);
        Ok(())
    }
}

/// Stable facade for the Phase 01 microkernel primitive set.
///
/// The trait intentionally composes smaller contracts instead of exposing
/// concrete maps or provider handles.  This keeps consumers dependent on the
/// microkernel invariants, not on the current in-memory skeleton.
pub trait KernelFacade:
    CapabilityRegistry + SystemServiceRegistry + PolicyEngine + ResourceManager + TraceEventBus
{
}

impl<T> KernelFacade for T where
    T: CapabilityRegistry + SystemServiceRegistry + PolicyEngine + ResourceManager + TraceEventBus
{
}

/// Default additive facade backed by in-memory primitive components.
///
/// This is the default implementation. It is safe for
/// tests and SDK discovery, but it should not be mistaken for a future
/// distributed service registry, production policy engine, or durable trace
/// store.
pub struct DefaultKernelFacade {
    capabilities: InMemoryCapabilityRegistry,
    services: InMemorySystemServiceRegistry,
    policy: DefaultAllowPolicyEngine,
    resources: InMemoryResourceManager,
    traces: InMemoryTraceEventBus,
}

impl Default for DefaultKernelFacade {
    fn default() -> Self {
        Self {
            capabilities: InMemoryCapabilityRegistry::new(),
            services: InMemorySystemServiceRegistry::new(),
            policy: DefaultAllowPolicyEngine::new(),
            resources: InMemoryResourceManager::new(),
            traces: InMemoryTraceEventBus::new(),
        }
    }
}

impl DefaultKernelFacade {
    /// Create a default additive facade.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return a snapshot of emitted primitive trace events.
    pub fn trace_events(&self) -> KernelPrimitiveResult<Vec<KernelTraceEvent>> {
        self.traces.events()
    }
}

impl CapabilityRegistry for DefaultKernelFacade {
    fn register_capability(&self, descriptor: CapabilityDescriptor) -> KernelPrimitiveResult<()> {
        self.capabilities.register_capability(descriptor)
    }

    fn get_capability(
        &self,
        id: &CapabilityId,
    ) -> KernelPrimitiveResult<Option<CapabilityDescriptor>> {
        self.capabilities.get_capability(id)
    }

    fn list_capabilities(&self) -> KernelPrimitiveResult<Vec<CapabilityDescriptor>> {
        self.capabilities.list_capabilities()
    }
}

impl SystemServiceRegistry for DefaultKernelFacade {
    fn register_service(
        &self,
        id: KernelServiceId,
        scope: ServiceScope,
    ) -> KernelPrimitiveResult<()> {
        self.services.register_service(id, scope)
    }

    fn get_service_scope(
        &self,
        id: &KernelServiceId,
    ) -> KernelPrimitiveResult<Option<ServiceScope>> {
        self.services.get_service_scope(id)
    }

    fn list_services(&self) -> KernelPrimitiveResult<Vec<(KernelServiceId, ServiceScope)>> {
        self.services.list_services()
    }
}

impl PolicyEngine for DefaultKernelFacade {
    fn evaluate(&self, request: &PolicyRequest) -> KernelPrimitiveResult<PolicyDecision> {
        self.policy.evaluate(request)
    }
}

impl ResourceManager for DefaultKernelFacade {
    fn register_resource(&self, scope: ResourceScope) -> KernelPrimitiveResult<()> {
        self.resources.register_resource(scope)
    }

    fn has_resource(&self, scope: &ResourceScope) -> KernelPrimitiveResult<bool> {
        self.resources.has_resource(scope)
    }

    fn list_resources(&self) -> KernelPrimitiveResult<Vec<ResourceScope>> {
        self.resources.list_resources()
    }
}

impl TraceEventBus for DefaultKernelFacade {
    fn emit_trace(&self, event: KernelTraceEvent) -> KernelPrimitiveResult<()> {
        self.traces.emit_trace(event)
    }
}
