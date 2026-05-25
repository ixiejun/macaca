//! Service contract for Macaca's Autonomy Evolution Control Plane.
//!
//! This crate owns the provider-neutral contract for generic agent
//! self-evolution orchestration. It deliberately does not know how to rewrite a
//! Skill package, run a benchmark, mutate Macaca source code, or render shell
//! diagnostics. Those target-specific behaviors stay behind replaceable
//! service adapters so the control plane remains a traceable State machine
//! rather than another application workflow engine.

mod admission;
mod benchmark;
mod governance_ledger;
mod local_provider;
mod model;
mod release;
mod release_model;
mod service_contract;
mod state_machine;

pub use admission::*;
pub use benchmark::*;
pub use governance_ledger::*;
pub use local_provider::*;
pub use model::*;
pub use release::*;
pub use release_model::*;
pub use service_contract::*;
pub use state_machine::*;
