//! Compression threshold configuration for working-memory summarization.

// ---------------------------------------------------------------------------
// CompressionConfig
// ---------------------------------------------------------------------------

/// Configuration for automatic working-memory compression.
///
/// When the estimated token count exceeds `trigger_threshold`, the framework
/// compresses old messages into a summary, keeping the `keep_recent` most
/// recent messages intact.
#[derive(Debug, Clone)]
pub struct CompressionConfig {
    /// Token count that triggers compression.
    pub trigger_threshold: usize,
    /// Target token count after compression.
    pub target_tokens: usize,
    /// Number of recent messages to keep uncompressed.
    pub keep_recent: usize,
    /// Optional model name to use for summary generation.
    /// Falls back to the agent's default model when `None`.
    pub summary_model: Option<String>,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            trigger_threshold: 8000,
            target_tokens: 4000,
            keep_recent: 5,
            summary_model: None,
        }
    }
}
