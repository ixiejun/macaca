//! Working memory — in-session message store with mark-based lifecycle (**Strategy**).

use async_trait::async_trait;
use serde_json::Value;

use crate::message::Msg;
use crate::state::{StateError, StateModule};

use super::types::TaggedMsg;

/// In-session message store with tag-based lifecycle management.
///
/// Implementors hold the current conversation history and expose fine-grained
/// access through marks (tags): you can retrieve, delete, or relabel messages
/// without iterating the store manually.
///
/// Requires `StateModule` so memory contents survive agent restarts.
#[async_trait]
pub trait WorkingMemory: StateModule + Send + Sync {
    /// Append `msg` to the memory with the given marks.
    async fn add(&mut self, msg: Msg, marks: Vec<String>);

    /// Return messages, optionally filtered by mark.
    ///
    /// - `mark = Some(m)` → only messages that have mark `m`
    /// - `exclude_mark = Some(m)` → only messages that do **not** have mark `m`
    /// - Both `None` → all messages
    ///
    /// When both are provided, `mark` filter is applied first,
    /// then `exclude_mark` is ignored (mark wins).
    async fn get_memory(&self, mark: Option<&str>, exclude_mark: Option<&str>) -> Vec<Msg>;

    /// Remove the message with the given `msg_id` (no-op if not found).
    async fn delete(&mut self, msg_id: &str);

    /// Remove all messages that carry `mark`.
    async fn delete_by_mark(&mut self, mark: &str);

    /// In all messages whose IDs are in `msg_ids`, replace `old_mark` with `new_mark`.
    ///
    /// Messages that do not carry `old_mark` are left unchanged.
    async fn update_mark(&mut self, msg_ids: &[String], old_mark: &str, new_mark: &str);

    /// Number of messages currently in memory.
    async fn size(&self) -> usize;

    /// Remove all messages and clear the summary.
    async fn clear(&mut self);

    /// Replace the compression summary with `summary`.
    async fn update_summary(&mut self, summary: Msg);

    /// Return messages with the summary (if any) prepended.
    ///
    /// Use this when building the LLM context window: the summary covers
    /// compressed history, followed by the full recent messages.
    async fn get_with_summary(&self) -> Vec<Msg>;
}

// ---------------------------------------------------------------------------
// InMemoryWorkingMemory — default heap-backed implementation
// ---------------------------------------------------------------------------

/// Heap-backed `WorkingMemory` implementation.
///
/// All messages are kept in a `Vec<TaggedMsg>`. This is the default
/// implementation suitable for single-process deployments.
pub struct InMemoryWorkingMemory {
    content: Vec<TaggedMsg>,
    summary: Option<Msg>,
}

impl InMemoryWorkingMemory {
    /// Create an empty working memory.
    pub fn new() -> Self {
        Self {
            content: Vec::new(),
            summary: None,
        }
    }
}

impl Default for InMemoryWorkingMemory {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WorkingMemory for InMemoryWorkingMemory {
    async fn add(&mut self, msg: Msg, marks: Vec<String>) {
        tracing::debug!(
            target = "macaca_framework::memory::working",
            msg_id = %msg.id,
            mark_count = marks.len(),
            "appending message to working memory"
        );
        self.content.push(TaggedMsg { msg, marks });
    }

    async fn get_memory(&self, mark: Option<&str>, exclude_mark: Option<&str>) -> Vec<Msg> {
        self.content
            .iter()
            .filter(|tm| {
                if let Some(m) = mark {
                    tm.marks.iter().any(|t| t == m)
                } else if let Some(ex) = exclude_mark {
                    !tm.marks.iter().any(|t| t == ex)
                } else {
                    true
                }
            })
            .map(|tm| tm.msg.clone())
            .collect()
    }

    async fn delete(&mut self, msg_id: &str) {
        let before = self.content.len();
        self.content.retain(|tm| tm.msg.id != msg_id);
        tracing::debug!(
            target = "macaca_framework::memory::working",
            msg_id = msg_id,
            removed = before.saturating_sub(self.content.len()),
            "deleted message by id from working memory"
        );
    }

    async fn delete_by_mark(&mut self, mark: &str) {
        let before = self.content.len();
        self.content
            .retain(|tm| !tm.marks.iter().any(|t| t == mark));
        tracing::debug!(
            target = "macaca_framework::memory::working",
            mark = mark,
            removed = before.saturating_sub(self.content.len()),
            "deleted messages by mark from working memory"
        );
    }

    async fn update_mark(&mut self, msg_ids: &[String], old_mark: &str, new_mark: &str) {
        for tm in self.content.iter_mut() {
            if msg_ids.contains(&tm.msg.id) {
                for t in tm.marks.iter_mut() {
                    if t == old_mark {
                        *t = new_mark.to_string();
                    }
                }
            }
        }
    }

    async fn size(&self) -> usize {
        self.content.len()
    }

    async fn clear(&mut self) {
        let cleared = self.content.len();
        self.content.clear();
        self.summary = None;
        tracing::debug!(
            target = "macaca_framework::memory::working",
            cleared_messages = cleared,
            "cleared working memory and summary"
        );
    }

    async fn update_summary(&mut self, summary: Msg) {
        tracing::debug!(
            target = "macaca_framework::memory::working",
            summary_id = %summary.id,
            "updated compression summary in working memory"
        );
        self.summary = Some(summary);
    }

    async fn get_with_summary(&self) -> Vec<Msg> {
        let mut result = Vec::with_capacity(self.content.len() + 1);
        if let Some(ref s) = self.summary {
            result.push(s.clone());
        }
        for tm in &self.content {
            result.push(tm.msg.clone());
        }
        result
    }
}

// StateModule for InMemoryWorkingMemory
//
// Serialization schema:
//   {
//     "content": [
//       { "msg": <Msg as JSON>, "marks": ["tag1", "tag2"] },
//       ...
//     ],
//     "summary": <Msg as JSON> | null
//   }
impl StateModule for InMemoryWorkingMemory {
    fn state_dict(&self) -> Value {
        let content: Vec<Value> = self
            .content
            .iter()
            .map(|tm| {
                serde_json::json!({
                    "msg": serde_json::to_value(&tm.msg).unwrap_or(Value::Null),
                    "marks": tm.marks,
                })
            })
            .collect();

        let summary = self
            .summary
            .as_ref()
            .map(|s| serde_json::to_value(s).unwrap_or(Value::Null))
            .unwrap_or(Value::Null);

        serde_json::json!({
            "content": content,
            "summary": summary,
        })
    }

    fn load_state_dict(&mut self, state: Value) -> Result<(), StateError> {
        // Restore content
        if let Some(arr) = state.get("content").and_then(|v| v.as_array()) {
            self.content.clear();
            for item in arr {
                let msg_val = item
                    .get("msg")
                    .cloned()
                    .ok_or_else(|| StateError::MissingField("msg".into()))?;
                let msg: Msg = serde_json::from_value(msg_val)
                    .map_err(|e| StateError::DeserializeFailed(e.to_string()))?;

                let marks: Vec<String> = item
                    .get("marks")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();

                self.content.push(TaggedMsg { msg, marks });
            }
        }

        // Restore summary
        self.summary = match state.get("summary") {
            Some(v) if !v.is_null() => {
                let msg: Msg = serde_json::from_value(v.clone())
                    .map_err(|e| StateError::DeserializeFailed(e.to_string()))?;
                Some(msg)
            }
            _ => None,
        };

        Ok(())
    }

    fn module_name(&self) -> &str {
        "working_memory"
    }
}
