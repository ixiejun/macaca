//! `aos-kernel` — agent runtime, scheduler, and orchestrator.

pub mod alert;
pub mod audit;
pub mod capability_registry;
pub mod executor;
pub mod facade;
pub mod kernel;
pub mod kernel_builder;
pub mod logging;
pub mod orchestrator;
pub mod policy;
pub mod registry;
pub mod resource;
pub mod scheduler;
pub mod scheduler_factory;
pub mod service_call;
pub mod service_lifecycle;
pub mod service_registry;
pub mod services;
pub mod status;
pub mod status_transition;
pub mod system_service;
pub mod trace_service_adapter;

pub use capability_registry::{CapabilityRegistry, InMemoryCapabilityRegistry};
pub use facade::{
    DefaultKernelFacade, InMemoryTraceEventBus, KernelFacade, KernelTraceEvent, TraceEventBus,
};
pub use kernel::Kernel;
pub use kernel_builder::KernelBuilder;
pub use orchestrator::AgentOrchestrator;
pub use policy::{DefaultAllowPolicyEngine, PolicyEngine, StaticDenyPolicyEngine};
pub use registry::{AgentEntry, AgentRegistry};
pub use resource::{InMemoryResourceManager, ResourceManager};
#[allow(deprecated)]
pub use scheduler::{Scheduler, SimpleScheduler};
pub use scheduler_factory::{SchedulerFactory, SchedulerKind};
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

// Re-export executor types
pub use executor::{
    AgentInfo, AgentRunner, ApplicationExecutor, ApplicationExecutorConfig,
    ApplicationExecutorRegistry, CallbackDispatcher, DelegateResult, DelegatedTask, EventBus,
    ExecutionQueue, ExecutorCommand, ExecutorEvent, ExecutorEventFactory, ForkContext, ForkManager,
    HookEvent, MergeResult, RoutingDecision, SystemEvent, TaskContext, TaskExecutor, TaskId,
    TaskResult, TaskRouter, TaskStatus, TokenUsage,
};
