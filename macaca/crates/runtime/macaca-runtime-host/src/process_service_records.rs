//! Record and output helpers for `service.process`.
//!
//! These helpers are pure DTO builders.  Keeping them outside the local
//! provider keeps the provider focused on process lifecycle while preserving a
//! single implementation for hashes, artifact-backed output deltas, and audit
//! summaries.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;

use base64::Engine;
use macaca_proto::workbench::process::*;
use macaca_proto::ArtifactRef;
use uuid::Uuid;

pub(crate) fn push_output_delta(
    deltas: &mut Vec<ProcessOutputDelta>,
    sequence: u64,
    stream: ProcessOutputStream,
    bytes: Vec<u8>,
    budget: u64,
) {
    if bytes.is_empty() {
        return;
    }
    let over_budget = bytes.len() as u64 > budget.max(1);
    let text = String::from_utf8(bytes.clone()).ok();
    let binary = text.is_none();
    let artifact_ref = (over_budget || binary).then(|| ArtifactRef {
        id: format!("process-artifact-{}", Uuid::new_v4()),
        media_type: "application/octet-stream".into(),
        sha256: None,
        byte_len: bytes.len() as u64,
        redaction_profile: "process.output".into(),
    });
    let data = if over_budget {
        None
    } else if let Some(text) = text {
        Some(text)
    } else {
        Some(base64::engine::general_purpose::STANDARD.encode(&bytes))
    };
    deltas.push(ProcessOutputDelta {
        sequence,
        stream,
        data,
        base64: binary && !over_budget,
        byte_len: bytes.len() as u64,
        truncated: over_budget,
        artifact_ref,
        captured_at: chrono::Utc::now(),
    });
}

pub(crate) fn running_record(
    handle: ProcessHandleRef,
    request: &ProcessCommandRequest,
    cwd: &Path,
) -> ProcessRecord {
    ProcessRecord {
        handle,
        command_hash: command_hash(request),
        executable_summary: executable_summary(request),
        cwd_scope: cwd.to_string_lossy().to_string(),
        sandbox_ref: request.sandbox.sandbox_profile_ref.clone(),
        permission_profile_ref: request.sandbox.permission_profile_ref.clone(),
        resource_lease_refs: request.sandbox.resource_lease_refs.clone(),
        state: ProcessLifecycleState::Running,
        exit_code: None,
        signal: None,
        duration_ms: None,
        stdout_byte_count: 0,
        stderr_byte_count: 0,
        pty_rows: request.allocate_pty.then_some(24),
        pty_cols: request.allocate_pty.then_some(80),
        audit_refs: vec!["process:spawn".into()],
        started_at: chrono::Utc::now(),
        completed_at: None,
    }
}

pub(crate) fn finished_record(
    handle: ProcessHandleRef,
    request: &ProcessCommandRequest,
    cwd: &Path,
    started_at: chrono::DateTime<chrono::Utc>,
    duration_ms: u64,
    exit_code: Option<i32>,
    stdout_byte_count: u64,
    stderr_byte_count: u64,
) -> ProcessRecord {
    let mut record = running_record(handle, request, cwd);
    record.state = ProcessLifecycleState::Exited;
    record.exit_code = exit_code;
    record.duration_ms = Some(duration_ms);
    record.stdout_byte_count = stdout_byte_count;
    record.stderr_byte_count = stderr_byte_count;
    record.started_at = started_at;
    record.completed_at = Some(chrono::Utc::now());
    record.audit_refs = vec!["process:exec".into()];
    record
}

pub(crate) fn byte_count(deltas: &[ProcessOutputDelta], stream: ProcessOutputStream) -> u64 {
    deltas
        .iter()
        .filter(|delta| delta.stream == stream)
        .map(|delta| delta.byte_len)
        .sum()
}

fn command_hash(request: &ProcessCommandRequest) -> String {
    let mut hasher = DefaultHasher::new();
    request.executable.hash(&mut hasher);
    request.args.hash(&mut hasher);
    request.sandbox.workspace_root.hash(&mut hasher);
    request.sandbox.cwd.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn executable_summary(request: &ProcessCommandRequest) -> String {
    format!("{} argc={}", request.executable, request.args.len())
}
