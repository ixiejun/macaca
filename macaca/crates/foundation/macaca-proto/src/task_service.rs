//! Provider-neutral Task Service DTOs.
//!
//! These command and snapshot shapes are shared by SDK clients, runtime-host
//! providers, and task-service implementations. Keeping them in `macaca-proto`
//! prevents facade crates from importing concrete task-service internals just
//! to exchange protocol data.

mod commands;
mod constants;
mod events;
mod prompts;
mod queries;
mod snapshots;

pub use commands::*;
pub use constants::*;
pub use events::*;
pub use prompts::*;
pub use queries::*;
pub use snapshots::*;
