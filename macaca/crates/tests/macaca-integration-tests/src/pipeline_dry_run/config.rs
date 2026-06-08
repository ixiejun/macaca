//! Runtime configuration for the pipeline dry-run harness.
//!
//! **Design pattern:** Value Object — bundles verbosity toggles without embedding
//! application business rules. The harness itself lives in the integration-test crate
//! and exercises OS primitives with scripted LLM fixtures.

/// Controls stderr trace granularity for the full pipeline dry-run harness.
#[derive(Debug, Clone, Copy)]
pub struct PipelineDryRunConfig {
    /// When true, print stage boundaries and agent execution events to stderr.
    pub verbose: bool,
}

impl Default for PipelineDryRunConfig {
    fn default() -> Self {
        Self { verbose: false }
    }
}

impl PipelineDryRunConfig {
    /// Construct a config that enables human-readable stderr tracing.
    pub fn verbose() -> Self {
        Self { verbose: true }
    }
}
