//! `aos-kernel` — microkernel primitives, scheduler contracts, and service-call invariants.

pub mod alert;
pub mod audit;
pub mod capability_registry;
pub mod domain_pack_registration;
pub mod execution_port;
pub mod facade;
pub mod kernel;
pub mod kernel_builder;
pub mod logging;
pub mod persistence;
pub mod plugin_registry;
pub mod policy;
pub mod registry;
pub mod resource;
pub mod scheduler;
pub mod scheduler_factory;
pub mod service_bus_bridge;
pub mod service_call;
pub mod service_lifecycle;
pub mod service_registry;
pub mod status;
pub mod status_transition;
pub mod system_service;
pub mod trace_service_adapter;

pub use capability_registry::{CapabilityRegistry, InMemoryCapabilityRegistry};
pub use domain_pack_registration::DomainPackProviderRegistration;
pub use execution_port::{SwappableAgentExecutionPort, UnavailableAgentExecutionPort};
pub use facade::{
    DefaultKernelFacade, InMemoryTraceEventBus, KernelFacade, KernelTraceEvent, TraceEventBus,
};
pub use kernel::Kernel;
pub use kernel_builder::KernelBuilder;
pub use macaca_proto::AgentExecutionPort;
pub use persistence::{KernelPersistencePort, UnavailableKernelPersistencePort};
pub use plugin_registry::{
    is_valid_transition as is_valid_plugin_lifecycle_transition, PluginRegistry,
    PluginRegistryEntry, PluginRegistrySnapshot,
};
pub use policy::{DefaultAllowPolicyEngine, PolicyEngine, StaticDenyPolicyEngine};
pub use registry::AgentRegistry;
pub use resource::{InMemoryResourceManager, ResourceManager};
pub use scheduler::Scheduler;
pub use scheduler_factory::{SchedulerFactory, SchedulerKind};
pub use service_bus_bridge::SystemServiceBusHandler;
pub use service_call::{
    ServiceCallContext, ServiceCallExecutor, ServiceCallMiddleware, TraceRequiredMiddleware,
};
pub use service_lifecycle::{DefaultServiceLifecycleController, ServiceLifecycleController};
pub use service_registry::{InMemorySystemServiceRegistry, SystemServiceRegistry};
pub use status::AgentStatusTracker;
pub use status_transition::AgentStatusTransitionPolicy;
pub use system_service::{MockSystemService, SystemService};
pub use trace_service_adapter::trace_service_descriptor;
