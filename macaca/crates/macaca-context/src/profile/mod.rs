//! Agent profile Markdown loaded through the context composer (OpenClaw-style persona files).
//!
//! This module is intentionally **not** named `persona` to avoid colliding with `macaca-sdk`
//! persona structs — here we only emit [`crate::composer::ContextCandidate`] values.

mod kinds;
mod loader;
mod policy;
mod provider;

#[cfg(test)]
mod tests;

pub use kinds::AgentProfileFileKind;
pub use loader::{canonical_root, load_profile_file, load_profile_file_at_canonical_root, ProfileLoadOutput, ProfileSkipReason};
pub use policy::{default_policy_for, ProfileKindPolicy};
pub use provider::{profile_provider_arc, ProfileFileContextProvider};
