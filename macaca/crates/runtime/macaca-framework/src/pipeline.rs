//! Pipeline orchestration (**Facade** module tree).
//!
//! Composes multiple agents into structured communication patterns:
//! - [`SequentialPipeline`] — chain agents so each output feeds the next input
//! - [`FanoutPipeline`] — broadcast one message to multiple agents in parallel or sequence
//! - [`MsgHub`] — multi-agent round-table where each reply is broadcast to all others
//!
//! # Module tree (P4 iteration 104)
//! - `core` — [`Pipeline`] trait (Strategy)
//! - `sequential` — [`SequentialPipeline`] (Chain of Responsibility)
//! - `fanout` — [`FanoutPipeline`] (Strategy + Composite)
//! - `msg_hub` — [`MsgHub`] (Mediator + Observer)
//! - `tests` — pipeline ordering and boundary contract tests

mod core;
mod fanout;
mod msg_hub;
mod sequential;

#[cfg(test)]
mod tests;

pub use core::Pipeline;
pub use fanout::FanoutPipeline;
pub use msg_hub::MsgHub;
pub use sequential::SequentialPipeline;
