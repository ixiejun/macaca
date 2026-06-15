//! Event payloads emitted by the Plan scheduling loop (Observer pattern).
//!
//! `PlanEvent` variants cross the async channel to Plan Agent consumers.
//! The loop never calls LLM directly — it only signals work that needs attention.

pub use macaca_proto::PlanEvent;
