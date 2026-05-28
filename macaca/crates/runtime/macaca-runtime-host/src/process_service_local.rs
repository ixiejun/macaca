//! Local process Strategy used by `service.process`.
//!
//! This module owns host process mechanics for the built-in provider only.
//! Service lifecycle, command envelopes, trace, audit refs, and service.tool
//! routing stay in `process_service_provider`.  The Strategy launches commands
//! without invoking a shell, resolves cwd against an explicit workspace root,
//! bounds output, and records replayable process state without exposing raw OS
//! handles or environment values.

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use macaca_proto::workbench::process::*;
use macaca_proto::workbench::sandbox::resolve_builtin_permission_profile;
use macaca_proto::workbench::sandbox::{SandboxNetworkMode, SandboxWriteScope};
use macaca_proto::{ServiceError, ServiceResult};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{broadcast, Mutex, RwLock};
use tokio::time::{timeout, Duration};
use tracing::{info, warn};
use uuid::Uuid;

use crate::process_service_records::{
    byte_count, finished_record, push_output_delta, running_record,
};

/// Replaceable process provider Strategy.
#[async_trait]
pub trait ProcessProvider: Send + Sync {
    async fn exec(&self, request: ProcessCommandRequest) -> ServiceResult<ProcessProviderOutput>;
    async fn spawn(&self, request: ProcessCommandRequest) -> ServiceResult<ProcessRecord>;
    async fn write_stdin(&self, request: ProcessStdinWriteRequest) -> ServiceResult<u64>;
    async fn resize_pty(&self, request: ProcessPtyResizeRequest) -> ServiceResult<ProcessRecord>;
    async fn terminate(&self, request: ProcessTerminateRequest) -> ServiceResult<ProcessRecord>;
    async fn subscribe_output(
        &self,
        request: ProcessOutputSubscribeRequest,
    ) -> ServiceResult<Vec<ProcessOutputDelta>>;
    async fn status(&self, request: ProcessStatusRequest) -> ServiceResult<ProcessRecord>;
    async fn cleanup(&self, request: ProcessCleanupRequest) -> ServiceResult<ProcessCleanupOutput>;
}

pub struct ProcessProviderOutput {
    pub record: ProcessRecord,
    pub output: Vec<ProcessOutputDelta>,
}

pub struct ProcessCleanupOutput {
    pub terminated_handles: Vec<ProcessHandleRef>,
    pub retained_handles: Vec<ProcessHandleRef>,
}

/// Local provider backed by `tokio::process::Command`.
#[derive(Clone)]
pub struct LocalProcessProvider {
    records: Arc<RwLock<HashMap<String, Arc<ProcessEntry>>>>,
    output_tx: broadcast::Sender<ProcessOutputDelta>,
}

struct ProcessEntry {
    ownership: ProcessOwnership,
    child: Mutex<Option<Child>>,
    stdin: Mutex<Option<ChildStdin>>,
    record: RwLock<ProcessRecord>,
    output: RwLock<Vec<ProcessOutputDelta>>,
}

#[derive(Clone)]
struct ProcessOwnership {
    application_id: Option<String>,
    session_id: Option<String>,
    thread_ref: Option<String>,
}

impl Default for LocalProcessProvider {
    fn default() -> Self {
        let (output_tx, _) = broadcast::channel(1024);
        Self {
            records: Arc::new(RwLock::new(HashMap::new())),
            output_tx,
        }
    }
}

impl LocalProcessProvider {
    pub fn subscribe(&self) -> broadcast::Receiver<ProcessOutputDelta> {
        self.output_tx.subscribe()
    }

    async fn entry(&self, handle: &ProcessHandleRef) -> ServiceResult<Arc<ProcessEntry>> {
        self.records
            .read()
            .await
            .get(&handle.id)
            .cloned()
            .ok_or_else(|| ServiceError::InvalidArgument("unknown process handle".into()))
    }
}

#[async_trait]
impl ProcessProvider for LocalProcessProvider {
    async fn exec(&self, request: ProcessCommandRequest) -> ServiceResult<ProcessProviderOutput> {
        validate_policy_before_spawn(&request)?;
        let started = chrono::Utc::now();
        let timer = Instant::now();
        let cwd = resolve_cwd(&request.sandbox)?;
        let mut command = build_command(&request, &cwd);
        info!(
            executable = %request.executable,
            cwd = %cwd.display(),
            "process local provider starting bounded exec"
        );
        let future = command.output();
        let output = if let Some(timeout_ms) = request.timeout_ms {
            timeout(Duration::from_millis(timeout_ms), future)
                .await
                .map_err(|_| ServiceError::AdapterFailure("process.exec timed out".into()))?
                .map_err(io_error)?
        } else {
            future.await.map_err(io_error)?
        };
        let handle = ProcessHandleRef {
            id: format!("process-exec-{}", Uuid::new_v4()),
        };
        let mut deltas = Vec::new();
        push_output_delta(
            &mut deltas,
            1,
            ProcessOutputStream::Stdout,
            output.stdout,
            request.inline_output_budget_bytes,
        );
        push_output_delta(
            &mut deltas,
            2,
            ProcessOutputStream::Stderr,
            output.stderr,
            request.inline_output_budget_bytes,
        );
        let record = finished_record(
            handle,
            &request,
            &cwd,
            started,
            timer.elapsed().as_millis() as u64,
            output.status.code(),
            byte_count(&deltas, ProcessOutputStream::Stdout),
            byte_count(&deltas, ProcessOutputStream::Stderr),
        );
        Ok(ProcessProviderOutput {
            record,
            output: deltas,
        })
    }

    async fn spawn(&self, request: ProcessCommandRequest) -> ServiceResult<ProcessRecord> {
        validate_policy_before_spawn(&request)?;
        let cwd = resolve_cwd(&request.sandbox)?;
        let mut command = build_command(&request, &cwd);
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        let mut child = command.spawn().map_err(io_error)?;
        let stdin = child.stdin.take();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let handle = ProcessHandleRef {
            id: format!("process-bg-{}", Uuid::new_v4()),
        };
        let record = running_record(handle.clone(), &request, &cwd);
        let entry = Arc::new(ProcessEntry {
            ownership: ProcessOwnership {
                application_id: request.application_id.clone(),
                session_id: request.session_id.clone(),
                thread_ref: request.thread_ref.clone(),
            },
            child: Mutex::new(Some(child)),
            stdin: Mutex::new(stdin),
            record: RwLock::new(record.clone()),
            output: RwLock::new(Vec::new()),
        });
        self.records
            .write()
            .await
            .insert(handle.id.clone(), Arc::clone(&entry));
        spawn_output_reader(
            stdout,
            Arc::clone(&entry),
            self.output_tx.clone(),
            ProcessOutputStream::Stdout,
        );
        spawn_output_reader(
            stderr,
            Arc::clone(&entry),
            self.output_tx.clone(),
            ProcessOutputStream::Stderr,
        );
        info!(handle = %handle.id, "process local provider spawned background process");
        Ok(record)
    }

    async fn write_stdin(&self, request: ProcessStdinWriteRequest) -> ServiceResult<u64> {
        let entry = self.entry(&request.handle).await?;
        let mut stdin = entry.stdin.lock().await;
        let Some(stdin) = stdin.as_mut() else {
            return Err(ServiceError::InvalidArgument(
                "process stdin is closed or unavailable".into(),
            ));
        };
        stdin
            .write_all(request.data.as_bytes())
            .await
            .map_err(io_error)?;
        stdin.flush().await.map_err(io_error)?;
        Ok(request.data.len() as u64)
    }

    async fn resize_pty(&self, request: ProcessPtyResizeRequest) -> ServiceResult<ProcessRecord> {
        if request.rows == 0 || request.cols == 0 {
            return Err(ServiceError::InvalidArgument(
                "PTY resize requires non-zero rows and cols".into(),
            ));
        }
        let entry = self.entry(&request.handle).await?;
        let mut record = entry.record.write().await;
        record.pty_rows = Some(request.rows);
        record.pty_cols = Some(request.cols);
        Ok(record.clone())
    }

    async fn terminate(&self, request: ProcessTerminateRequest) -> ServiceResult<ProcessRecord> {
        let entry = self.entry(&request.handle).await?;
        if let Some(mut child) = entry.child.lock().await.take() {
            child.kill().await.map_err(io_error)?;
            let _ = child.wait().await;
        }
        let mut record = entry.record.write().await;
        record.state = ProcessLifecycleState::Terminated;
        record.signal = Some(request.reason);
        record.completed_at = Some(chrono::Utc::now());
        Ok(record.clone())
    }

    async fn subscribe_output(
        &self,
        request: ProcessOutputSubscribeRequest,
    ) -> ServiceResult<Vec<ProcessOutputDelta>> {
        let entry = self.entry(&request.handle).await?;
        let deltas = entry.output.read().await;
        let selected = deltas
            .iter()
            .filter(|delta| delta.sequence >= request.since_sequence)
            .cloned()
            .collect();
        Ok(selected)
    }

    async fn status(&self, request: ProcessStatusRequest) -> ServiceResult<ProcessRecord> {
        Ok(self
            .entry(&request.handle)
            .await?
            .record
            .read()
            .await
            .clone())
    }

    async fn cleanup(&self, request: ProcessCleanupRequest) -> ServiceResult<ProcessCleanupOutput> {
        let entries: Vec<_> = self
            .records
            .read()
            .await
            .values()
            .filter(|entry| ownership_matches(&entry.ownership, &request))
            .cloned()
            .collect();
        let mut terminated_handles = Vec::new();
        for entry in entries {
            let handle = entry.record.read().await.handle.clone();
            let record = self
                .terminate(ProcessTerminateRequest {
                    handle: handle.clone(),
                    reason: request.reason.clone(),
                })
                .await?;
            terminated_handles.push(record.handle);
        }
        let retained_entries: Vec<_> = self.records.read().await.values().cloned().collect();
        let mut retained_handles = Vec::new();
        for entry in retained_entries {
            retained_handles.push(entry.record.read().await.handle.clone());
        }
        Ok(ProcessCleanupOutput {
            terminated_handles,
            retained_handles,
        })
    }
}

fn build_command(request: &ProcessCommandRequest, cwd: &Path) -> Command {
    let mut command = Command::new(&request.executable);
    command.args(&request.args).current_dir(cwd);
    for env in &request.env {
        if !env.secret {
            command.env(&env.key, &env.value);
        }
    }
    command
}

fn validate_policy_before_spawn(request: &ProcessCommandRequest) -> ServiceResult<()> {
    if request.executable.trim().is_empty() {
        return Err(ServiceError::InvalidArgument(
            "process executable is required".into(),
        ));
    }
    if request.sandbox.sandbox_profile_ref.trim().is_empty()
        || request.sandbox.permission_profile_ref.trim().is_empty()
    {
        return Err(ServiceError::DisabledByPolicy(
            "process spawn requires resolved sandbox and permission profiles".into(),
        ));
    }
    let profile = resolve_builtin_permission_profile(&request.sandbox.permission_profile_ref)
        .ok_or_else(|| {
            ServiceError::DisabledByPolicy("unknown process permission profile".into())
        })?;
    if !profile.can_spawn_process {
        return Err(ServiceError::DisabledByPolicy(
            "permission profile denies process spawn".into(),
        ));
    }
    let privileged = matches!(profile.write_scope, SandboxWriteScope::FullAccess)
        || matches!(profile.network_mode, SandboxNetworkMode::Allowed);
    if privileged && request.sandbox.approval_ref.is_none() {
        return Err(ServiceError::DisabledByPolicy(
            "process spawn requires approval_ref for privileged permission profiles".into(),
        ));
    }
    Ok(())
}

fn resolve_cwd(sandbox: &ProcessSandboxRequest) -> ServiceResult<PathBuf> {
    let root = Path::new(&sandbox.workspace_root)
        .canonicalize()
        .map_err(io_error)?;
    let cwd = sandbox.cwd.as_deref().unwrap_or(".");
    reject_unsafe_relative_path(cwd)?;
    let candidate = root.join(cwd);
    let absolute = candidate.canonicalize().map_err(io_error)?;
    if !absolute.starts_with(&root) {
        return Err(ServiceError::DisabledByPolicy(
            "process cwd escapes workspace root".into(),
        ));
    }
    Ok(absolute)
}

fn reject_unsafe_relative_path(path: &str) -> ServiceResult<()> {
    for component in Path::new(path).components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(ServiceError::DisabledByPolicy(
                    "absolute cwd and traversal are denied".into(),
                ));
            }
        }
    }
    Ok(())
}

fn spawn_output_reader(
    output: Option<impl AsyncReadExt + Unpin + Send + 'static>,
    entry: Arc<ProcessEntry>,
    tx: broadcast::Sender<ProcessOutputDelta>,
    stream: ProcessOutputStream,
) {
    if let Some(mut output) = output {
        tokio::spawn(async move {
            let mut buf = [0_u8; 4096];
            let mut sequence = 1_u64;
            loop {
                let read = match output.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(read) => read,
                    Err(error) => {
                        warn!(error = %error, "process output reader failed");
                        break;
                    }
                };
                let mut deltas = Vec::new();
                push_output_delta(
                    &mut deltas,
                    sequence,
                    stream.clone(),
                    buf[..read].to_vec(),
                    4096,
                );
                if let Some(delta) = deltas.pop() {
                    {
                        let mut record = entry.record.write().await;
                        match delta.stream {
                            ProcessOutputStream::Stdout | ProcessOutputStream::PseudoPty => {
                                record.stdout_byte_count += delta.byte_len
                            }
                            ProcessOutputStream::Stderr => {
                                record.stderr_byte_count += delta.byte_len
                            }
                        }
                    }
                    entry.output.write().await.push(delta.clone());
                    let _ = tx.send(delta);
                }
                sequence += 1;
            }
        });
    }
}

fn ownership_matches(ownership: &ProcessOwnership, request: &ProcessCleanupRequest) -> bool {
    request.application_id.as_ref().map_or(true, |value| {
        ownership.application_id.as_ref() == Some(value)
    }) && request
        .session_id
        .as_ref()
        .map_or(true, |value| ownership.session_id.as_ref() == Some(value))
        && request
            .thread_ref
            .as_ref()
            .map_or(true, |value| ownership.thread_ref.as_ref() == Some(value))
}

fn io_error(error: std::io::Error) -> ServiceError {
    ServiceError::AdapterFailure(error.to_string())
}
