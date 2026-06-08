//! Long-term memory — cross-session record/retrieve store (**Strategy**).

use async_trait::async_trait;
use serde_json::Value;

use crate::message::Msg;
use crate::state::{StateError, StateModule};

use super::error::MemoryError;

// ---------------------------------------------------------------------------
// LongTermMemory trait
// ---------------------------------------------------------------------------

/// Cross-session memory store with record/retrieve semantics.
///
/// Implementations may use vector databases, keyword indices, or simple
/// file storage. The framework calls `record` after each turn and `retrieve`
/// before generating the next reply (in `StaticControl` mode), or exposes
/// these as agent tools (in `AgentControl` mode).
#[async_trait]
pub trait LongTermMemory: Send + Sync {
    /// Persist `msgs` into long-term storage.
    async fn record(&mut self, msgs: &[Msg]) -> Result<(), MemoryError>;

    /// Retrieve up to `limit` messages relevant to `query`.
    async fn retrieve(&self, query: &str, limit: usize) -> Result<Vec<Msg>, MemoryError>;

    /// Record specific content strings to long-term memory.
    /// Default implementation converts each string to a user Msg and calls record().
    async fn record_to_memory(&mut self, content: &[String]) -> Result<(), MemoryError> {
        let msgs: Vec<Msg> = content
            .iter()
            .map(|c| Msg::user("memory", c.as_str()))
            .collect();
        self.record(&msgs).await
    }

    /// Retrieve memories matching any of the given keywords.
    /// Default implementation joins keywords with spaces and calls retrieve().
    async fn retrieve_from_memory(
        &self,
        keywords: &[String],
        limit: usize,
    ) -> Result<Vec<Msg>, MemoryError> {
        let query = keywords.join(" ");
        self.retrieve(&query, limit).await
    }
}

/// Controls how the framework integrates `LongTermMemory` into the agent loop.
#[derive(Debug, Clone)]
pub enum LongTermMemoryMode {
    /// Framework automatically calls `record`/`retrieve` around each reply.
    StaticControl,
    /// Long-term memory is registered as agent tools; the LLM decides when to use them.
    AgentControl,
    /// Both automatic control and tool registration are active.
    Both,
}

// ---------------------------------------------------------------------------
// InMemoryLongTermMemory
// ---------------------------------------------------------------------------

/// Heap-backed `LongTermMemory` implementation with keyword-based retrieval.
///
/// Messages are stored in a `Vec<Msg>` and retrieved by splitting the query
/// into whitespace-delimited keywords and scoring each entry by the number of
/// matching keywords found in its text.
pub struct InMemoryLongTermMemory {
    entries: Vec<Msg>,
}

impl InMemoryLongTermMemory {
    /// Create an empty long-term memory.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl Default for InMemoryLongTermMemory {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LongTermMemory for InMemoryLongTermMemory {
    async fn record(&mut self, msgs: &[Msg]) -> Result<(), MemoryError> {
        tracing::debug!(
            target = "macaca_framework::memory::long_term",
            batch_size = msgs.len(),
            total_after = self.entries.len() + msgs.len(),
            "recording messages into long-term memory"
        );
        self.entries.extend(msgs.iter().cloned());
        Ok(())
    }

    async fn retrieve(&self, query: &str, limit: usize) -> Result<Vec<Msg>, MemoryError> {
        let keywords: Vec<&str> = query.split_whitespace().collect();
        if keywords.is_empty() {
            tracing::debug!(
                target = "macaca_framework::memory::long_term",
                "long-term retrieve skipped: empty query"
            );
            return Ok(Vec::new());
        }

        let mut scored: Vec<(usize, &Msg)> = self
            .entries
            .iter()
            .filter_map(|msg| {
                let text = msg.get_text().to_lowercase();
                let score = keywords
                    .iter()
                    .filter(|kw| text.contains(&kw.to_lowercase()))
                    .count();
                if score > 0 {
                    Some((score, msg))
                } else {
                    None
                }
            })
            .collect();

        // Sort by score descending
        scored.sort_by(|a, b| b.0.cmp(&a.0));

        let results: Vec<Msg> = scored
            .into_iter()
            .take(limit)
            .map(|(_, msg)| msg.clone())
            .collect();
        tracing::debug!(
            target = "macaca_framework::memory::long_term",
            keyword_count = keywords.len(),
            limit = limit,
            matched = results.len(),
            "retrieved messages from long-term memory"
        );
        Ok(results)
    }
}

impl StateModule for InMemoryLongTermMemory {
    fn state_dict(&self) -> Value {
        let entries: Vec<Value> = self
            .entries
            .iter()
            .map(|msg| serde_json::to_value(msg).unwrap_or(Value::Null))
            .collect();
        Value::Array(entries)
    }

    fn load_state_dict(&mut self, state: Value) -> Result<(), StateError> {
        if let Some(arr) = state.as_array() {
            self.entries.clear();
            for item in arr {
                let msg: Msg = serde_json::from_value(item.clone())
                    .map_err(|e| StateError::DeserializeFailed(e.to_string()))?;
                self.entries.push(msg);
            }
        }
        Ok(())
    }

    fn module_name(&self) -> &str {
        "long_term_memory"
    }
}
