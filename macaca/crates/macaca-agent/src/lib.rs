//! Agent OS — agent crate.
//!
//! Provides the [`Agent`] trait, [`AgentStateMachine`], [`BasicAgent`],
//! service injection types, and graceful-shutdown support.

pub mod agent;
pub mod basic;
pub mod shutdown;
pub mod state_machine;

pub use agent::{
    Agent, AgentServices, IpcService, MemoryService, NoopIpcService, NoopMemoryService,
    NoopPersistService, PersistService,
};
pub use basic::{
    AgentCapabilityNode, AgentCapabilitySet, BasicAgent, BasicAgentBuilder, CapabilitySource,
};
pub use shutdown::ShutdownHandle;
pub use state_machine::{
    AgentLifecyclePolicy, AgentStateMachine, AgentTransitionReason, DefaultAgentLifecyclePolicy,
};
