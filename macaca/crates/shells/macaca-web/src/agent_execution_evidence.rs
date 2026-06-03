//! Agent Execution evidence helpers for Web's service backend.
//!
//! The Web shell is the local adapter that can observe framework tool events
//! while `service.agent_execution` runs. This module keeps that observation
//! isolated from the backend orchestration code: it records bounded tool events,
//! forwards them to the existing executor event stream, and extracts sanitized
//! artifact evidence from successful file-writing tools. It never reads
//! application prompts, parses `HEARTBEAT.md`, or hardcodes application-specific
//! artifact names.

use std::collections::BTreeMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::time::UNIX_EPOCH;

use macaca_kernel::executor::{ApplicationExecutor, ExecutorEvent};
use macaca_proto::{AgentExecutionEvent, TaskId};
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::application_execution_agent_event_bridge::ApplicationExecutionAgentEventMirror;

const MAX_RECORDED_AGENT_EVENTS: usize = 256;

/// Bounded observer for one Agent Execution run.
///
/// The observer is an in-process Memento of semantic execution events. It
/// exists only long enough to build sanitized result metadata for autonomy
/// dispatchers; durable replay still flows through EventLog/executor events.
pub(crate) struct AgentExecutionEvidenceObserver {
    events: Arc<Mutex<Vec<AgentExecutionEvent>>>,
    forwarder: tokio::task::JoinHandle<()>,
    expected_artifact: Option<ExpectedArtifactProbe>,
}

impl AgentExecutionEvidenceObserver {
    /// Wait for the event forwarder to drain and return safe artifact metadata.
    ///
    /// Callers should drop the runtime agent before invoking this method so all
    /// toolkit senders close. If a forwarder panics, the method still returns an
    /// empty metadata map because evidence extraction must never turn a completed
    /// agent run into a process crash.
    pub(crate) async fn finish(self) -> BTreeMap<String, String> {
        let _ = self.forwarder.await;
        let events = self
            .events
            .lock()
            .map(|events| events.clone())
            .unwrap_or_default();
        let mut metadata = artifact_metadata_from_events(
            &events,
            self.expected_artifact
                .as_ref()
                .map(|probe| probe.path.as_str()),
        );
        if metadata.is_empty() {
            if let Some(probe) = self.expected_artifact {
                metadata = probe.finish();
            }
        }
        metadata
    }
}

/// Pre/post execution probe for a manifest-declared durable artifact.
///
/// This is intentionally generic: Runtime Host can pass an expected artifact
/// path through `evidence.*` metadata, and Web can validate only file-system
/// facts before/after the agent run. The probe does not inspect prompt content,
/// app identifiers, or business semantics, so it stays within the shell adapter
/// boundary while still proving that a fixed sentinel was actually refreshed.
struct ExpectedArtifactProbe {
    path: String,
    before: Option<ArtifactSnapshot>,
}

impl ExpectedArtifactProbe {
    /// Capture the artifact state before the model/tool execution starts.
    fn capture(path: &str) -> Self {
        let before = ArtifactSnapshot::capture(path);
        info!(
            artifact_path_hash = %stable_text_hash(path),
            existed_before = before.is_some(),
            "captured expected agent-execution artifact baseline"
        );
        Self {
            path: path.into(),
            before,
        }
    }

    /// Return sanitized metadata only when the artifact changed during the run.
    fn finish(self) -> BTreeMap<String, String> {
        let after = ArtifactSnapshot::capture(&self.path);
        let Some(after) = after else {
            warn!(
                artifact_path_hash = %stable_text_hash(&self.path),
                "expected agent-execution artifact was not present after run"
            );
            return BTreeMap::new();
        };
        if self.before.as_ref() == Some(&after) {
            warn!(
                artifact_path_hash = %stable_text_hash(&self.path),
                "expected agent-execution artifact did not change during run"
            );
            return BTreeMap::new();
        }

        let mut metadata = BTreeMap::new();
        metadata.insert(
            "artifact_ref".into(),
            format!("fs:expected:{}", stable_text_hash(&self.path)),
        );
        metadata.insert("artifact_digest".into(), after.digest.clone());
        metadata.insert("artifact_kind".into(), "expected_file".into());
        metadata.insert("artifact_bytes".into(), after.len.to_string());
        metadata.insert("artifact_path_hash".into(), stable_text_hash(&self.path));
        metadata.insert(
            "artifact_mtime_unix_nanos".into(),
            after.modified_nanos.to_string(),
        );
        info!(
            artifact_path_hash = %stable_text_hash(&self.path),
            artifact_bytes = after.len,
            "expected agent-execution artifact changed and became completion evidence"
        );
        metadata
    }
}

/// Minimal file snapshot used to prove that an expected artifact changed.
///
/// The digest is built from file metadata instead of file contents. That keeps
/// evidence extraction cheap and avoids leaking artifact text into service
/// metadata while still detecting the stale-sentinel failure mode: if mtime and
/// size remain fixed, the run produced no fresh durable artifact evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ArtifactSnapshot {
    len: u64,
    modified_nanos: u128,
    digest: String,
}

impl ArtifactSnapshot {
    fn capture(path: &str) -> Option<Self> {
        let metadata = fs::metadata(path).ok()?;
        let modified = metadata.modified().ok()?;
        let modified_nanos = modified.duration_since(UNIX_EPOCH).ok()?.as_nanos();
        let len = metadata.len();
        Some(Self {
            len,
            modified_nanos,
            digest: stable_text_hash(&format!("{path}:{len}:{modified_nanos}")),
        })
    }
}

/// Create an observed event channel for one Agent Execution run.
///
/// The returned sender is passed into `FrameworkRunner`, where framework
/// middleware emits semantic `AgentExecutionEvent` rows. The observer records a
/// bounded copy and optionally forwards the same events to the application
/// executor so UI traces keep their current behavior.
pub(crate) fn observed_agent_execution_events(
    executor: Option<Arc<ApplicationExecutor>>,
    task_id: TaskId,
    agent: String,
    expected_artifact_path: Option<&str>,
    application_execution_mirror: Option<ApplicationExecutionAgentEventMirror>,
) -> (
    mpsc::Sender<AgentExecutionEvent>,
    AgentExecutionEvidenceObserver,
) {
    let (tx, mut rx) = mpsc::channel::<AgentExecutionEvent>(64);
    let events = Arc::new(Mutex::new(Vec::new()));
    let recorded_events = Arc::clone(&events);
    let forwarder = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            if let Ok(mut guard) = recorded_events.lock() {
                if guard.len() < MAX_RECORDED_AGENT_EVENTS {
                    guard.push(event.clone());
                }
            }
            if let Some(executor) = executor.as_ref() {
                executor.broadcast_event(ExecutorEvent::AgentEvent {
                    task_id,
                    agent: agent.clone(),
                    event: event.clone(),
                });
            }
            if let Some(mirror) = application_execution_mirror.as_ref() {
                // Application-execution mirroring is best-effort telemetry
                // persistence. The authoritative agent execution must keep
                // running even if the application-execution gateway rejects a
                // duplicate, a stale callback, or a temporarily unavailable
                // service call. The mirror logs those failures with run scope.
                mirror.observe(&event).await;
            }
        }
    });
    (
        tx,
        AgentExecutionEvidenceObserver {
            events,
            forwarder,
            expected_artifact: expected_artifact_path.map(ExpectedArtifactProbe::capture),
        },
    )
}

/// Build a bounded output hash for audit correlation only.
///
/// The hash proves only that Agent Execution produced a result object. It is
/// deliberately not accepted as autonomous task-completion evidence by Runtime
/// Host because natural-language output can exist even when no requested
/// artifact was written.
pub(crate) fn stable_agent_output_hash(value: &serde_json::Value) -> String {
    let encoded = serde_json::to_string(value).unwrap_or_default();
    stable_text_hash(&encoded)
}

fn artifact_metadata_from_events(
    events: &[AgentExecutionEvent],
    expected_artifact_path: Option<&str>,
) -> BTreeMap<String, String> {
    let mut metadata = BTreeMap::new();
    for event in events {
        let AgentExecutionEvent::ToolResult {
            tool_name,
            output,
            is_error,
        } = event
        else {
            continue;
        };
        if tool_name != "file_write" || is_error.unwrap_or(false) {
            continue;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(output) else {
            continue;
        };
        let Some(path) = value.get("path").and_then(|path| path.as_str()) else {
            continue;
        };
        if expected_artifact_path
            .map(|expected| expected != path)
            .unwrap_or(false)
        {
            continue;
        }
        let bytes_written = value
            .get("bytes_written")
            .and_then(|bytes| bytes.as_u64())
            .unwrap_or(0);
        if bytes_written == 0 {
            continue;
        }
        metadata.insert(
            "artifact_ref".into(),
            format!("tool:file_write:{}", stable_text_hash(path)),
        );
        metadata.insert(
            "artifact_digest".into(),
            stable_text_hash(&format!("{path}:{bytes_written}")),
        );
        metadata.insert("artifact_kind".into(), "file_write".into());
        metadata.insert("artifact_bytes".into(), bytes_written.to_string());
        metadata.insert("artifact_path_hash".into(), stable_text_hash(path));
        break;
    }
    metadata
}

fn stable_text_hash(value: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file_write_result(path: &str) -> AgentExecutionEvent {
        AgentExecutionEvent::ToolResult {
            tool_name: "file_write".into(),
            output: serde_json::json!({
                "path": path,
                "bytes_written": 42
            })
            .to_string(),
            is_error: Some(false),
        }
    }

    #[test]
    fn file_write_tool_result_becomes_sanitized_artifact_evidence() {
        let metadata = artifact_metadata_from_events(
            &[file_write_result("/workspace/agents/a/sentinel.md")],
            Some("/workspace/agents/a/sentinel.md"),
        );

        assert_eq!(
            metadata.get("artifact_kind").map(String::as_str),
            Some("file_write")
        );
        assert_eq!(
            metadata.get("artifact_bytes").map(String::as_str),
            Some("42")
        );
        assert!(metadata.contains_key("artifact_ref"));
        assert!(metadata.contains_key("artifact_digest"));
        assert!(!metadata.values().any(|value| value.contains("sentinel.md")));
    }

    #[test]
    fn file_write_to_unexpected_path_is_not_completion_evidence() {
        let metadata = artifact_metadata_from_events(
            &[file_write_result("/workspace/agents/a/alternate.md")],
            Some("/workspace/agents/a/sentinel.md"),
        );

        assert!(metadata.is_empty());
    }

    #[test]
    fn output_hash_is_audit_correlation_not_artifact_evidence() {
        let hash = stable_agent_output_hash(&serde_json::json!({"output": "done"}));

        assert_eq!(hash.len(), 16);
    }

    #[test]
    fn unchanged_expected_file_is_not_completion_evidence() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("sentinel.md");
        std::fs::write(&path, "old").expect("write old sentinel");
        let path = path.to_string_lossy().to_string();
        let probe = ExpectedArtifactProbe::capture(&path);

        let metadata = probe.finish();

        assert!(metadata.is_empty());
    }

    #[test]
    fn changed_expected_file_becomes_sanitized_completion_evidence() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("sentinel.md");
        std::fs::write(&path, "old").expect("write old sentinel");
        let path = path.to_string_lossy().to_string();
        let probe = ExpectedArtifactProbe::capture(&path);
        std::thread::sleep(std::time::Duration::from_millis(5));
        std::fs::write(&path, "new sentinel").expect("write new sentinel");

        let metadata = probe.finish();

        assert_eq!(
            metadata.get("artifact_kind").map(String::as_str),
            Some("expected_file")
        );
        assert!(metadata.contains_key("artifact_ref"));
        assert!(metadata.contains_key("artifact_digest"));
        assert!(!metadata.values().any(|value| value.contains("sentinel.md")));
    }
}
