//! Pipeline dry-run stage commands (Template Method steps).
//!
//! Each submodule implements one isolated verification step against OS primitives.

pub mod agentic_create;
pub mod agentic_worker_review;
pub mod depends_on_review;
pub mod session_claim;
