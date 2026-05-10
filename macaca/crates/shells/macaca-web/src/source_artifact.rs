//! Source artifact retrieval boundary for context-report diagnostics.
//!
//! Context pruning is intentionally non-destructive: model-visible snippets may be shortened, but
//! operators still need a scoped way to inspect the canonical EventLog row that produced a snippet.
//! This module is the Repository/Adapter boundary for that lookup. HTTP routes and UI code should
//! call this boundary instead of constructing storage keys directly.

use std::sync::Arc;

use macaca_context::ContextReport;
use macaca_persist::{AppendEventCommand, EventLog};
use serde::{Deserialize, Serialize};

const DEFAULT_PREVIEW_BYTES: usize = 64 * 1024;

/// Query shape for `/api/sessions/:id/source-artifact?ref=...`.
#[derive(Debug, Deserialize)]
pub struct SourceArtifactQuery {
    #[serde(rename = "ref")]
    pub source_ref: String,
}

/// Bounded diagnostic payload returned for a source artifact lookup.
#[derive(Debug, Serialize)]
pub struct SourceArtifactResponse {
    pub requested_ref: String,
    pub resolution_kind: String,
    pub available: bool,
    pub canonical_ref: Option<String>,
    pub event_seq: Option<u64>,
    pub event_type: Option<String>,
    pub source: Option<String>,
    pub timestamp: Option<String>,
    pub payload: Option<serde_json::Value>,
    pub payload_preview: Option<serde_json::Value>,
    pub payload_bytes: Option<usize>,
    pub preview_truncated: bool,
    pub unavailable_reason: Option<String>,
}

/// Canonical EventLog reference after validating the requested session scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedEventArtifactRef {
    pub canonical_ref: String,
    pub session_id: String,
    pub seq: u64,
}

/// Repository-style adapter for retrieving pruned source originals.
pub struct ContextSourceArtifactRepository {
    event_log: Arc<EventLog>,
    preview_bytes: usize,
}

/// Persist canonical originals for pruned model-message sources and attach retrievable refs.
///
/// Context engines only see the model-call input snapshot, so their reports can identify a pruned
/// row as `message/{idx}` but cannot themselves persist the original payload. The web hot path owns
/// both the original framework message values and the [`EventLog`]; this helper is the small
/// Repository adapter that bridges those two worlds before the final `context_report` event is
/// written. Reports then carry `events/{session}/{seq}` refs that the UI can resolve through
/// [`ContextSourceArtifactRepository`] without knowing EventLog key internals.
pub async fn persist_pruned_source_artifacts(
    event_log: &EventLog,
    session_id: &str,
    agent_name: &str,
    app_id: Option<String>,
    report: &mut ContextReport,
    original_messages: &[serde_json::Value],
) {
    for source in &mut report.sources {
        if source.source_ref.is_some() || !source_needs_artifact(source) {
            continue;
        }
        let Some(idx) = message_source_index(&source.id) else {
            continue;
        };
        let Some(original_message) = original_messages.get(idx) else {
            source.source_ref = Some(format!("unavailable/message/{idx}"));
            continue;
        };

        let mut command = AppendEventCommand::new(
            session_id,
            "context_source_artifact",
            agent_name,
            serde_json::json!({
                "agent": agent_name,
                "source_id": source.id,
                "source_kind": source.kind,
                "source_label": source.label,
                "render_mode": source.render_mode,
                "pruned_tokens": source.pruned_tokens,
                "original_message": original_message,
            }),
        )
        .with_agent_name(agent_name.to_string());
        if let Some(app_id) = app_id.as_ref() {
            command = command.with_app_id(app_id.clone());
        }

        let seq = event_log.append_command(command).await;
        source.source_ref = Some(format!("events/{session_id}/{seq:08}"));
    }
}

impl ContextSourceArtifactRepository {
    /// Create a repository with bounded default previews.
    pub fn new(event_log: Arc<EventLog>) -> Self {
        Self {
            event_log,
            preview_bytes: DEFAULT_PREVIEW_BYTES,
        }
    }

    /// Override preview size for tests or stricter deployments.
    #[cfg(test)]
    fn with_preview_bytes(mut self, preview_bytes: usize) -> Self {
        self.preview_bytes = preview_bytes.max(1);
        self
    }

    /// Resolve and fetch one source ref under the requested session scope.
    pub async fn resolve(
        &self,
        requested_session_id: &str,
        source_ref: &str,
    ) -> SourceArtifactResponse {
        let requested_ref = source_ref.to_string();
        let resolved = match resolve_source_artifact_ref(requested_session_id, source_ref) {
            Ok(resolved) => resolved,
            Err(reason) => return unavailable(requested_ref, "unsupported", None, None, reason),
        };

        let event = self
            .event_log
            .get_by_seq(&resolved.session_id, resolved.seq)
            .await;
        let Some(event) = event else {
            return unavailable(
                requested_ref,
                "event_log",
                Some(resolved.canonical_ref),
                Some(resolved.seq),
                "canonical EventLog row not found",
            );
        };

        let payload_bytes = serde_json::to_vec(&event.payload)
            .map(|payload| payload.len())
            .unwrap_or(0);
        let (payload_preview, preview_truncated) =
            bounded_payload_preview(&event.payload, self.preview_bytes);

        SourceArtifactResponse {
            requested_ref,
            resolution_kind: "event_log".into(),
            available: true,
            canonical_ref: Some(resolved.canonical_ref),
            event_seq: Some(event.seq),
            event_type: Some(event.event_type),
            source: Some(event.source),
            timestamp: Some(event.timestamp.to_rfc3339()),
            payload: Some(payload_preview.clone()),
            payload_preview: Some(payload_preview),
            payload_bytes: Some(payload_bytes),
            preview_truncated,
            unavailable_reason: None,
        }
    }
}

/// Decide whether a context report source needs a persisted source-artifact event.
///
/// Full sources are already visible in the assembled prompt, but excerpted/summarized/dropped
/// sources need an EventLog-backed canonical payload so diagnostics can prove pruning was
/// non-destructive. The check intentionally mirrors the UI's “show retrieval button” predicate.
fn source_needs_artifact(source: &macaca_context::ContextSourceReport) -> bool {
    source.pruned_tokens > 0
        || source
            .render_mode
            .as_deref()
            .map(|mode| mode != "full")
            .unwrap_or(false)
}

/// Parse `message/{idx}` source ids emitted by `PruningContextEngine`.
fn message_source_index(source_id: &str) -> Option<usize> {
    let idx = source_id.strip_prefix("message/")?;
    idx.parse().ok()
}

/// Resolve a report source ref into an EventLog row while enforcing session scope.
pub fn resolve_source_artifact_ref(
    requested_session_id: &str,
    source_ref: &str,
) -> Result<ResolvedEventArtifactRef, String> {
    let trimmed = source_ref.trim().trim_start_matches('/');
    let parts: Vec<_> = trimmed.split('/').collect();
    match parts.as_slice() {
        ["event", seq] => {
            let seq = seq
                .parse::<u64>()
                .map_err(|_| format!("invalid event sequence in ref: {source_ref}"))?;
            Ok(ResolvedEventArtifactRef {
                canonical_ref: format!("events/{requested_session_id}/{seq:08}"),
                session_id: requested_session_id.to_string(),
                seq,
            })
        }
        ["events", session_id, seq] => {
            if *session_id != requested_session_id {
                return Err(format!(
                    "cross-session source ref is not allowed: requested session {requested_session_id}, ref targets {session_id}"
                ));
            }
            let seq = seq
                .parse::<u64>()
                .map_err(|_| format!("invalid event sequence in ref: {source_ref}"))?;
            Ok(ResolvedEventArtifactRef {
                canonical_ref: format!("events/{session_id}/{seq:08}"),
                session_id: (*session_id).to_string(),
                seq,
            })
        }
        _ => Err(format!(
            "source ref is not backed by EventLog retrieval yet: {source_ref}"
        )),
    }
}

fn unavailable(
    requested_ref: String,
    resolution_kind: &str,
    canonical_ref: Option<String>,
    event_seq: Option<u64>,
    reason: impl Into<String>,
) -> SourceArtifactResponse {
    SourceArtifactResponse {
        requested_ref,
        resolution_kind: resolution_kind.into(),
        available: false,
        canonical_ref,
        event_seq,
        event_type: None,
        source: None,
        timestamp: None,
        payload: None,
        payload_preview: None,
        payload_bytes: None,
        preview_truncated: false,
        unavailable_reason: Some(reason.into()),
    }
}

fn bounded_payload_preview(
    payload: &serde_json::Value,
    max_bytes: usize,
) -> (serde_json::Value, bool) {
    let text = serde_json::to_string_pretty(payload).unwrap_or_else(|_| payload.to_string());
    if text.len() <= max_bytes {
        return (payload.clone(), false);
    }
    let preview = truncate_utf8(&text, max_bytes);
    (
        serde_json::json!({
            "truncated": true,
            "original_bytes": text.len(),
            "preview": preview,
        }),
        true,
    )
}

fn truncate_utf8(input: &str, max_bytes: usize) -> String {
    input
        .chars()
        .scan(0usize, |used, ch| {
            let next = *used + ch.len_utf8();
            if next > max_bytes {
                None
            } else {
                *used = next;
                Some(ch)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_persist::{AppendEventCommand, RedbStore};

    #[test]
    fn resolves_short_event_ref_against_requested_session() {
        let resolved = resolve_source_artifact_ref("session-a", "event/42").unwrap();
        assert_eq!(resolved.session_id, "session-a");
        assert_eq!(resolved.seq, 42);
        assert_eq!(resolved.canonical_ref, "events/session-a/00000042");
    }

    #[test]
    fn resolves_canonical_event_ref_when_session_matches() {
        let resolved =
            resolve_source_artifact_ref("session-a", "events/session-a/00000007").unwrap();
        assert_eq!(resolved.session_id, "session-a");
        assert_eq!(resolved.seq, 7);
        assert_eq!(resolved.canonical_ref, "events/session-a/00000007");
    }

    #[test]
    fn rejects_cross_session_refs() {
        let error =
            resolve_source_artifact_ref("session-a", "events/session-b/00000007").unwrap_err();
        assert!(error.contains("cross-session"));
    }

    #[test]
    fn unsupported_provider_refs_return_error_message() {
        let error = resolve_source_artifact_ref("session-a", "workspace-memory").unwrap_err();
        assert!(error.contains("not backed by EventLog retrieval"));
    }

    #[tokio::test]
    async fn repository_returns_bounded_preview_without_mutating_event_log() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(RedbStore::open(dir.path().join("events.redb")).unwrap());
        let log = Arc::new(EventLog::new(store));
        let original = "abcdef".repeat(100);
        let seq = log
            .append_command(AppendEventCommand::new(
                "session-a",
                "tool_result",
                "agent",
                serde_json::json!({ "output": original }),
            ))
            .await;

        let repo = ContextSourceArtifactRepository::new(Arc::clone(&log)).with_preview_bytes(64);
        let response = repo.resolve("session-a", &format!("event/{seq}")).await;
        assert!(response.available);
        assert!(response.preview_truncated);
        assert!(response.payload_bytes.unwrap_or_default() > 64);

        let event = log.get_by_seq("session-a", seq).await.unwrap();
        assert_eq!(event.payload["output"].as_str(), Some(original.as_str()));
    }

    #[tokio::test]
    async fn repository_reports_missing_event_explicitly() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(RedbStore::open(dir.path().join("events.redb")).unwrap());
        let repo = ContextSourceArtifactRepository::new(Arc::new(EventLog::new(store)));
        let response = repo.resolve("session-a", "event/99").await;
        assert!(!response.available);
        assert_eq!(
            response.unavailable_reason.as_deref(),
            Some("canonical EventLog row not found")
        );
    }

    #[tokio::test]
    async fn pruned_message_sources_are_persisted_and_refilled_with_event_refs() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(RedbStore::open(dir.path().join("events.redb")).unwrap());
        let log = Arc::new(EventLog::new(store));
        let mut report = macaca_context::ContextReportBuilder::new("pruning")
            .source(
                macaca_context::ContextSourceReport::included(
                    "message/0",
                    macaca_context::ContextSourceKind::ToolResult,
                    "tool result",
                    12,
                    2048,
                )
                .with_rendering("excerpt", "untrusted", None, 32),
            )
            .build();
        let originals = vec![serde_json::json!({
            "role": "tool",
            "content": "canonical tool output that must remain retrievable",
        })];

        persist_pruned_source_artifacts(
            &log,
            "session-a",
            "agent-a",
            Some("app-a".into()),
            &mut report,
            &originals,
        )
        .await;

        let source_ref = report.sources[0]
            .source_ref
            .as_deref()
            .expect("pruned source should receive EventLog ref");
        assert!(source_ref.starts_with("events/session-a/"));

        let repo = ContextSourceArtifactRepository::new(Arc::clone(&log));
        let response = repo.resolve("session-a", source_ref).await;
        assert!(response.available);
        assert_eq!(
            response.payload_preview.unwrap()["original_message"]["content"].as_str(),
            Some("canonical tool output that must remain retrievable")
        );
    }
}
