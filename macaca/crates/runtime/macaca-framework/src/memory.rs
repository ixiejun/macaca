//! Memory system — working memory with tag-based filtering and long-term memory (**Facade**).
//!
//! Provides two complementary memory abstractions for agent sessions:
//! - [`WorkingMemory`]: in-session message store with tag-based filtering,
//!   delete-by-mark, mark updates, and optional compression summary.
//! - [`LongTermMemory`]: persistent store with record/retrieve semantics,
//!   useful for cross-session recall.
//!
//! # Module tree (P4 iteration 109)
//! - `types` — [`TaggedMsg`] value object
//! - `error` — [`MemoryError`] / [`CompressError`] taxonomy
//! - `working` — [`WorkingMemory`] Strategy + [`InMemoryWorkingMemory`] Memento
//! - `long_term` — [`LongTermMemory`] Strategy + [`InMemoryLongTermMemory`]
//! - `config` — [`CompressionConfig`] threshold policy
//! - `tokens` — token estimation helpers
//! - `compressor` — [`MemoryCompressor`] Template Method summarization
//! - `tests` — contract tests per memory family

mod compressor;
mod config;
mod error;
mod long_term;
mod tokens;
mod types;
mod working;

#[cfg(test)]
mod tests;

pub use compressor::MemoryCompressor;
pub use config::CompressionConfig;
pub use error::{CompressError, MemoryError};
pub use long_term::{
    InMemoryLongTermMemory, LongTermMemory, LongTermMemoryMode,
};
pub use tokens::{estimate_messages_tokens, estimate_tokens};
pub use types::TaggedMsg;
pub use working::{InMemoryWorkingMemory, WorkingMemory};
