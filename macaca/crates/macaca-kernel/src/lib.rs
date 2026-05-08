//! `aos-kernel` — agent runtime, scheduler, and orchestrator.

pub mod a2a;
pub mod a2a_event;
pub mod alert;
pub mod audit;
pub mod capability_registry;
pub mod evm;
pub mod evm_adapter;
pub mod evm_event;
#[cfg(test)]
mod evm_tests;
pub mod executor;
pub mod facade;
pub mod kernel;
pub mod kernel_builder;
pub mod logging;
pub mod orchestrator;
pub mod payment_policy;
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
pub mod services;
pub mod status;
pub mod status_transition;
pub mod system_service;
pub mod trace_service_adapter;
pub mod web3;
pub mod web3_event;
#[cfg(test)]
mod web3_tests;

pub use capability_registry::{CapabilityRegistry, InMemoryCapabilityRegistry};
pub use evm::{DefaultEvmPolicyEngine, EvmAdapter, EvmFacade, EvmPolicyEngine};
pub use evm_adapter::{MockEvmAdapter, UnavailableEvmAdapter};
pub use evm_event::{
    EvmTraceEvent, EvmTraceEventSink, InMemoryEvmTraceEventSink, NoopEvmTraceEventSink,
};
pub use facade::{
    DefaultKernelFacade, InMemoryTraceEventBus, KernelFacade, KernelTraceEvent, TraceEventBus,
};
pub use kernel::Kernel;
pub use kernel_builder::KernelBuilder;
pub use orchestrator::AgentOrchestrator;
pub use payment_policy::{DefaultPaymentPolicyEngine, PaymentPolicyDecision, PaymentPolicyEngine};
pub use plugin_registry::{
    is_valid_transition as is_valid_plugin_lifecycle_transition, PluginRegistry,
    PluginRegistryEntry, PluginRegistrySnapshot,
};
pub use policy::{DefaultAllowPolicyEngine, PolicyEngine, StaticDenyPolicyEngine};
pub use registry::{AgentEntry, AgentRegistry};
pub use resource::{InMemoryResourceManager, ResourceManager};
#[allow(deprecated)]
pub use scheduler::{Scheduler, SimpleScheduler};
pub use scheduler_factory::{SchedulerFactory, SchedulerKind};
pub use service_bus_bridge::SystemServiceBusHandler;
pub use service_call::{
    ServiceCallContext, ServiceCallExecutor, ServiceCallMiddleware, TraceRequiredMiddleware,
};
pub use service_lifecycle::{DefaultServiceLifecycleController, ServiceLifecycleController};
pub use service_registry::{InMemorySystemServiceRegistry, SystemServiceRegistry};
pub use services::{IpcServiceAdapter, MemoryServiceAdapter, PersistServiceAdapter};
pub use status::AgentStatusTracker;
pub use status_transition::AgentStatusTransitionPolicy;
pub use system_service::{MockSystemService, SystemService};
pub use trace_service_adapter::trace_service_descriptor;
pub use web3::{
    DefaultWeb3PolicyEngine, MockWeb3Adapter, UnavailableWeb3Adapter, Web3Adapter, Web3Facade,
    Web3PolicyEngine,
};
pub use web3_event::{
    InMemoryWeb3TraceEventSink, NoopWeb3TraceEventSink, Web3TraceEvent, Web3TraceEventSink,
};

// Re-export executor types
pub use a2a::{
    local_simulated_terms, A2ACoordinator, A2APaymentFacade, A2AProtocolAdapter,
    LocalSimulatedA2AAdapter,
};
pub use a2a_event::{
    A2APaymentEvent, A2APaymentEventSink, InMemoryA2APaymentEventSink, NoopA2APaymentEventSink,
};
pub use executor::{
    AgentInfo, AgentRunner, ApplicationExecutor, ApplicationExecutorConfig,
    ApplicationExecutorRegistry, CallbackDispatcher, DelegateResult, DelegatedTask, EventBus,
    ExecutionQueue, ExecutorCommand, ExecutorEvent, ExecutorEventFactory, ForkContext, ForkManager,
    HookEvent, MergeResult, RoutingDecision, SystemEvent, TaskContext, TaskExecutor, TaskId,
    TaskResult, TaskRouter, TaskStatus, TokenUsage,
};
