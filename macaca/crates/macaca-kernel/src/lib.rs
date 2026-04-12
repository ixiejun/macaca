//! `aos-kernel` — agent runtime, scheduler, and orchestrator.

pub mod alert;
pub mod audit;
pub mod executor;
pub mod kernel;
pub mod logging;
pub mod orchestrator;
pub mod registry;
pub mod scheduler;
pub mod services;
pub mod status;

pub use kernel::Kernel;
pub use orchestrator::AgentOrchestrator;
pub use registry::{AgentEntry, AgentRegistry};
pub use scheduler::{Scheduler, SimpleScheduler};
pub use services::{IpcServiceAdapter, MemoryServiceAdapter, PersistServiceAdapter};
pub use status::AgentStatusTracker;

// Re-export executor types
pub use executor::{
    AgentInfo, AgentRunner, ApplicationExecutor, ApplicationExecutorConfig,
    ApplicationExecutorRegistry, CallbackDispatcher, DelegateResult, DelegatedTask, EventBus,
    ExecutionQueue, ExecutorCommand, ExecutorEvent, ForkContext, ForkManager, HookEvent,
    MergeResult, RoutingDecision, SystemEvent, TaskContext, TaskExecutor, TaskId, TaskResult,
    TaskRouter, TaskStatus, TokenUsage,
};
