//! Generic self-evolution evaluation contracts.
//!
//! This module owns provider-neutral DTOs for measuring whether governed skill
//! self-evolution actually occurred and whether it improved later task runs.
//! It deliberately stores references, counts, states, and bounded diagnostics
//! instead of raw task output or full skill bodies so evaluation reports remain
//! traceable, auditable, and safe for shell display.

pub mod model;

pub use model::*;
