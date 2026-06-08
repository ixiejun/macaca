//! Agent abstraction and hook system (**Facade** module tree).
//!
//! The [`Agent`] trait defines the core interface for all framework agents.
//! [`HookedAgent`] wraps any `Agent` to inject pre/post hooks on reply and observe.
//!
//! # Module tree (P4 iteration 103)
//! - `error` — [`AgentError`] / [`AgentResult`] taxonomy
//! - `core` — [`Agent`] trait (Strategy)
//! - `hooks` — [`Hook`] + [`HookRegistry`] (Chain of Responsibility)
//! - `static_hooks` — process-wide hook Singleton
//! - `hooked` — [`HookedAgent`] Decorator
//! - `tests` — hook ordering and boundary contract tests

mod core;
mod error;
mod hooked;
mod hooks;
mod static_hooks;

#[cfg(test)]
mod tests;

pub use core::Agent;
pub use error::{AgentError, AgentResult};
pub use hooked::HookedAgent;
pub use hooks::{Hook, HookRegistry};
pub use static_hooks::{clear_static_hooks, register_static_hook};
