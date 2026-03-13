//! `aos-kernel` — agent runtime, scheduler, and orchestrator.

pub mod executor;
pub mod registry;
pub mod scheduler;
pub mod kernel;
pub mod services;
pub mod status;
pub mod orchestrator;

pub use registry::{AgentEntry, AgentRegistry};
pub use scheduler::{Scheduler, SimpleScheduler};
pub use kernel::Kernel;
pub use services::{MemoryServiceAdapter, IpcServiceAdapter, PersistServiceAdapter};
pub use status::AgentStatusTracker;
pub use orchestrator::AgentOrchestrator;

// Re-export executor types
pub use executor::{
    TaskId, TaskStatus, DelegatedTask, TaskContext, TaskResult, TokenUsage,
    RoutingDecision, AgentInfo, EventBus, SystemEvent, CallbackDispatcher,
    ExecutionQueue, TaskRouter, TaskExecutor, ExecutorCommand, ExecutorEvent,
};
