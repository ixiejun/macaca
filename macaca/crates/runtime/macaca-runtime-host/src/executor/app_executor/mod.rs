//! Application Executor — facade module (P5 allowlist convergence).
//!
//! Provides an isolated execution environment for one application, containing all
//! components needed for agent-to-agent task delegation. Persistence is injected
//! as a kernel port; the executor owns scheduling, restart, and fork recovery.
//!
//! ## Design patterns
//!
//! - **Facade** — this module re-exports the public application executor API unchanged.
//! - **Supervisor** — automatic worker restart with cooldown reset (`supervisor.rs`).
//! - **Template Method** — service-backed vs worker-command delegation paths share lifecycle.
//! - **Observer** — broadcast channels for executor events and hook forwarding.
//! - **Registry** — `ApplicationExecutorRegistry` maps `ApplicationId` → executor instance.
//! - **Memento** — optional `KernelPersistencePort` restore for queue and fork state.
//!
//! ## Module layout
//!
//! | File | Responsibility |
//! |------|----------------|
//! | `types.rs` | Config + worker health value objects |
//! | `executor.rs` | `ApplicationExecutor` struct + constructors |
//! | `delegation.rs` | Task admission and routing |
//! | `queries.rs` | Agent/task queries and event subscriptions |
//! | `health.rs` | Shutdown and liveness checks |
//! | `supervisor.rs` | Worker spawn/restart loop |
//! | `worker_loop.rs` | Command processing and agent invocation |
//! | `registry.rs` | Multi-application executor registry |
//! | `tests.rs` | Unit tests |

mod delegation;
mod executor;
mod health;
mod queries;
mod registry;
mod supervisor;
mod types;
mod worker_loop;

#[cfg(test)]
mod tests;

pub use executor::ApplicationExecutor;
pub use registry::ApplicationExecutorRegistry;
pub use types::{ApplicationExecutorConfig, WorkerHealth, WorkerState, WorkerSupervisorConfig};
