//! Agent OS — agent crate.
//!
//! Provides the [`Agent`] trait, [`AgentStateMachine`], [`BasicAgent`],
//! service injection types, and graceful-shutdown support.

pub mod agent;
pub mod basic;
pub mod capability;
pub mod execution;
pub mod in_process_execution;
pub mod lifecycle;
pub mod services;
pub mod shutdown;
pub mod state_machine;

pub use agent::Agent;
pub use basic::{BasicAgent, BasicAgentBuilder};
pub use capability::{AgentCapabilityNode, AgentCapabilitySet, CapabilitySource};
pub use execution::{AgentExecutionDispatch, ServiceClientAgentExecutionAdapter};
pub use in_process_execution::{InProcessAgentExecutionPort, InProcessAgentSideRegistry};
pub use macaca_proto::AgentExecutionPort;
pub use lifecycle::{
    AgentLifecyclePolicy, AgentLifecycleTransition, AgentTransitionReason,
    DefaultAgentLifecyclePolicy,
};
pub use macaca_llm::LlmProvider;
pub use macaca_tools::ToolCatalog;
pub use services::{
    AgentServices, AgentServicesBuilder, IpcService, MemoryService, NoopIpcService,
    NoopMemoryService, NoopPersistService, PersistService,
};
pub use shutdown::ShutdownHandle;
pub use state_machine::AgentStateMachine;
