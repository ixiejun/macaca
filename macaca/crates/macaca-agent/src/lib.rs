//! Agent OS — agent crate.
//!
//! Provides the [`Agent`] trait, [`AgentStateMachine`], [`BasicAgent`],
//! service injection types, and graceful-shutdown support.

pub mod agent;
pub mod basic;
pub mod shutdown;
pub mod state_machine;

pub use agent::{Agent, AgentServices, MemoryService, IpcService, PersistService};
pub use basic::BasicAgent;
pub use shutdown::ShutdownHandle;
pub use state_machine::AgentStateMachine;
