//! `aos-kernel` — agent runtime, scheduler, and orchestrator.

pub mod alert;
pub mod audit;
pub mod executor;
pub mod kernel;
pub mod kernel_builder;
pub mod logging;
pub mod orchestrator;
pub mod registry;
pub mod scheduler;
pub mod scheduler_factory;
pub mod services;
pub mod status;
pub mod status_transition;

pub use kernel::Kernel;
pub use kernel_builder::KernelBuilder;
pub use orchestrator::AgentOrchestrator;
pub use registry::{AgentEntry, AgentRegistry};
#[allow(deprecated)]
pub use scheduler::{Scheduler, SimpleScheduler};
pub use scheduler_factory::{SchedulerFactory, SchedulerKind};
pub use services::{IpcServiceAdapter, MemoryServiceAdapter, PersistServiceAdapter};
pub use status::AgentStatusTracker;
pub use status_transition::AgentStatusTransitionPolicy;

// Re-export executor types
pub use executor::{
    AgentInfo, AgentRunner, ApplicationExecutor, ApplicationExecutorConfig,
    ApplicationExecutorRegistry, CallbackDispatcher, DelegateResult, DelegatedTask, EventBus,
    ExecutionQueue, ExecutorCommand, ExecutorEvent, ExecutorEventFactory, ForkContext, ForkManager,
    HookEvent, MergeResult, RoutingDecision, SystemEvent, TaskContext, TaskExecutor, TaskId,
    TaskResult, TaskRouter, TaskStatus, TokenUsage,
};
