//! Shared **Adapter** helpers for memory recall injection.
//!
//! Builds trace contexts, source report rows, and inserts synthetic system
//! snippets below the leading system block so recalled memory never overrides
//! core instructions.

use macaca_sdk::context::{ContextSourceKind, ContextSourceReport};
use macaca_proto::{LlmMessage, MemoryEntry, TraceContext};

/// Build a provider-neutral trace for memory prefetch commands.
pub(crate) fn memory_trace(session_id: Option<&str>, agent_name: Option<&str>) -> TraceContext {
    let mut trace = TraceContext::new(uuid::Uuid::new_v4().to_string());
    trace.session_id = session_id.map(str::to_owned);
    trace.agent = agent_name.map(str::to_owned);
    trace
}

/// Map a recalled memory row into the legacy context source report shape.
pub(crate) fn legacy_memory_source_report(
    entry: &MemoryEntry,
    label: &str,
    estimated_tokens: u32,
    byte_size: usize,
    render_mode: &str,
) -> ContextSourceReport {
    ContextSourceReport::included(
        entry.id.0.to_string(),
        ContextSourceKind::Memory,
        label,
        estimated_tokens,
        byte_size,
    )
    .with_rendering(
        render_mode,
        "untrusted",
        Some(format!("memory:{}", entry.id.0)),
        0,
    )
    .with_recall_metadata(
        "workspace-memory",
        entry.id.0.to_string(),
        85,
        "workspace",
        true,
    )
}

/// Insert a synthetic system snippet after the leading block of system messages.
///
/// Keeping recall below the leading system block prevents recalled memory from
/// silently overriding the core instruction hierarchy, while still making it
/// visible before normal conversation history.
pub(crate) fn insert_after_leading_system(messages: &mut Vec<LlmMessage>, snippet: LlmMessage) {
    let mut pos = 0usize;
    for message in messages.iter() {
        if message.role == macaca_proto::LlmRole::System {
            pos += 1;
        } else {
            break;
        }
    }
    messages.insert(pos, snippet);
}

/// Truncate a string by character count and append a diagnostic marker.
pub(crate) fn truncate_chars(mut s: String, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s;
    }
    s = s.chars().take(max_chars).collect();
    s.push_str("\n...[preflight truncated]");
    s
}
