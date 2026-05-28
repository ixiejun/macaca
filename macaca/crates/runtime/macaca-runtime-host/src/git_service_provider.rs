//! Runtime-host provider for `service.git`.
//!
//! The provider is a host-owned Adapter around the local `git` executable.  It
//! applies repository path policy, records pre-change Memento data, returns
//! patch provenance, and keeps all concrete process usage out of kernel, SDK,
//! shells, and applications.

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::workbench::git::*;
use macaca_proto::{
    BoundedSummary, CleanupPolicy, ServiceCallResult, ServiceCommand, ServiceDescriptor,
    ServiceError, ServiceHealth, ServiceResult, TraceContext, WorkbenchCommandResult,
    WorkbenchCommandStatus,
};
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    ServiceProviderFactoryContext, ServiceProviderInstance, ServiceRuntime,
    StaticServiceProviderFactory,
};

#[async_trait]
pub trait GitProvider: Send + Sync {
    async fn status(&self, request: GitRepositoryRequest) -> ServiceResult<GitServiceResponse>;
    async fn diff(&self, request: GitDiffRequest) -> ServiceResult<GitServiceResponse>;
    async fn apply_patch(
        &self,
        request: GitApplyPatchRequest,
        trace: &TraceContext,
    ) -> ServiceResult<GitServiceResponse>;
    async fn rollback_marker(
        &self,
        request: GitRepositoryRequest,
    ) -> ServiceResult<GitServiceResponse>;
    async fn rollback_replay(
        &self,
        request: GitRollbackMarkerReplayRequest,
    ) -> ServiceResult<GitServiceResponse>;
}

#[derive(Debug, Default)]
pub struct LocalGitProvider;

pub struct GitSystemServiceProvider {
    descriptor: ServiceDescriptor,
    provider: Option<Arc<dyn GitProvider>>,
    unavailable_reason: Option<String>,
}

pub async fn bootstrap_local_git_service(
    runtime: Arc<ServiceRuntime>,
    trace_id: impl Into<String>,
) -> macaca_proto::MacacaResult<macaca_proto::KernelServiceId> {
    let service: Arc<dyn SystemService> = Arc::new(GitSystemServiceProvider::local());
    let descriptor = service.descriptor();
    let service_id = descriptor.id.clone();
    let trace = TraceContext::new(trace_id);
    info!(service_id = %service_id, trace_id = %trace.trace_id, "git service registering local provider");
    runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(descriptor, service)),
            ServiceProviderFactoryContext::new(),
        )
        .await
        .map_err(runtime_error)?;
    runtime
        .start(&service_id, trace.clone())
        .await
        .map_err(runtime_error)?;
    info!(service_id = %service_id, trace_id = %trace.trace_id, "git service local provider started");
    Ok(service_id)
}

impl GitSystemServiceProvider {
    pub fn local() -> Self {
        Self::new(Arc::new(LocalGitProvider))
    }

    pub fn mock() -> Self {
        Self::local()
    }

    pub fn new(provider: Arc<dyn GitProvider>) -> Self {
        Self {
            descriptor: descriptor().base,
            provider: Some(provider),
            unavailable_reason: None,
        }
    }

    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            descriptor: descriptor().base,
            provider: None,
            unavailable_reason: Some(reason.into()),
        }
    }

    fn provider(&self) -> ServiceResult<Arc<dyn GitProvider>> {
        self.provider.clone().ok_or_else(|| {
            ServiceError::ServiceUnavailable(
                self.unavailable_reason
                    .clone()
                    .unwrap_or_else(|| "git provider is not configured".into()),
            )
        })
    }

    fn trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
        command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)
    }

    fn result(
        response: GitServiceResponse,
        trace: TraceContext,
        status: WorkbenchCommandStatus,
    ) -> ServiceResult<ServiceCallResult> {
        let result = WorkbenchCommandResult {
            trace: trace.clone(),
            status,
            output: Some(response),
            summary: None,
            audit_ref: Some(format!("git:{}", trace.trace_id)),
            completed_at: chrono::Utc::now(),
        };
        Ok(ServiceCallResult {
            output: serde_json::to_value(result)
                .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?,
            trace,
            status: "ok".into(),
            metadata: BTreeMap::new(),
            cleanup_hint: Some(CleanupPolicy::None),
        })
    }

    async fn unavailable_result(&self, trace: TraceContext) -> ServiceResult<ServiceCallResult> {
        Self::result(
            GitServiceResponse::Status {
                branch: None,
                head: None,
                files: Vec::new(),
                clean: false,
            },
            trace,
            WorkbenchCommandStatus::Unavailable {
                reason: self
                    .unavailable_reason
                    .clone()
                    .unwrap_or_else(|| "git provider is not configured".into()),
            },
        )
    }
}

#[async_trait]
impl SystemService for GitSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, configured = self.provider.is_some(), "git service provider started");
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        info!(service_id = %self.descriptor.id, command = %command.name, trace_id = %trace.trace_id, "git service command accepted");
        if self.provider.is_none() {
            warn!(service_id = %self.descriptor.id, trace_id = %trace.trace_id, "git unavailable provider returned structured unavailable");
            return self.unavailable_result(trace).await;
        }
        let response = match command.name.as_str() {
            STATUS_COMMAND => self.provider()?.status(decode(command.payload)?).await?,
            DIFF_COMMAND => self.provider()?.diff(decode(command.payload)?).await?,
            APPLY_PATCH_COMMAND => {
                self.provider()?
                    .apply_patch(decode(command.payload)?, &trace)
                    .await?
            }
            ROLLBACK_MARKER_CREATE_COMMAND => {
                self.provider()?
                    .rollback_marker(decode(command.payload)?)
                    .await?
            }
            ROLLBACK_MARKER_REPLAY_COMMAND => {
                self.provider()?
                    .rollback_replay(decode(command.payload)?)
                    .await?
            }
            "service.snapshot" => self
                .provider()?
                .status(GitRepositoryRequest {
                    workspace_root: ".".into(),
                })
                .await
                .unwrap_or(GitServiceResponse::Status {
                    branch: None,
                    head: None,
                    files: Vec::new(),
                    clean: false,
                }),
            other => return Err(ServiceError::UnsupportedCommand(other.into())),
        };
        Self::result(response, trace, WorkbenchCommandStatus::Completed)
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "git service provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "git service cleanup completed");
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        if let Some(reason) = &self.unavailable_reason {
            Ok(ServiceHealth::Unavailable {
                reason: reason.clone(),
            })
        } else {
            Ok(ServiceHealth::Healthy)
        }
    }
}

#[async_trait]
impl GitProvider for LocalGitProvider {
    async fn status(&self, request: GitRepositoryRequest) -> ServiceResult<GitServiceResponse> {
        let root = workspace_root(&request.workspace_root)?;
        let branch = git(&root, &["branch", "--show-current"]).ok();
        let head = git(&root, &["rev-parse", "HEAD"]).ok();
        let porcelain = git(&root, &["status", "--porcelain"])?;
        let files = porcelain
            .lines()
            .filter_map(|line| {
                let (status, path) = line.split_at(line.len().min(3));
                let path = path.trim();
                (!path.is_empty()).then(|| GitFileStatus {
                    path: path.into(),
                    status: status.trim().into(),
                })
            })
            .collect::<Vec<_>>();
        Ok(GitServiceResponse::Status {
            branch: branch
                .map(|item| item.trim().into())
                .filter(|item: &String| !item.is_empty()),
            head: head
                .map(|item| item.trim().into())
                .filter(|item: &String| !item.is_empty()),
            clean: files.is_empty(),
            files,
        })
    }

    async fn diff(&self, request: GitDiffRequest) -> ServiceResult<GitServiceResponse> {
        let root = workspace_root(&request.workspace_root)?;
        for pathspec in &request.pathspecs {
            reject_unsafe_relative_path(pathspec)?;
        }
        let mut args = vec!["diff", "--binary", "--"];
        args.extend(request.pathspecs.iter().map(String::as_str));
        let diff = git(&root, &args)?;
        Ok(GitServiceResponse::Diff {
            diff: summary(&diff, request.inline_budget_bytes),
            pathspecs: request.pathspecs,
        })
    }

    async fn apply_patch(
        &self,
        request: GitApplyPatchRequest,
        trace: &TraceContext,
    ) -> ServiceResult<GitServiceResponse> {
        if request.require_approval && request.approval_ref.is_none() {
            return Err(ServiceError::DisabledByPolicy(
                "git.apply_patch requires approval_ref before side effect".into(),
            ));
        }
        if request.actor_ref.trim().is_empty() {
            return Err(ServiceError::InvalidArgument(
                "git.apply_patch requires non-empty actor_ref".into(),
            ));
        }
        let root = workspace_root(&request.workspace_root)?;
        let affected_paths = affected_paths(&request.patch_text)?;
        for path in &affected_paths {
            reject_unsafe_relative_path(path)?;
        }
        let pre_change_marker = marker_for(&root, 4096)?;
        let patch_path = root.join(format!(".macaca-patch-{}.diff", Uuid::new_v4()));
        std::fs::write(&patch_path, request.patch_text.as_bytes()).map_err(io_error)?;
        let patch_path_string = patch_path.to_string_lossy().to_string();
        let check = git(&root, &["apply", "--check", &patch_path_string]);
        if check.is_err() {
            let _ = std::fs::remove_file(&patch_path);
        }
        check.map_err(|error| {
            ServiceError::InvalidArgument(format!("patch conflict or invalid patch: {error}"))
        })?;
        git(&root, &["apply", &patch_path_string])?;
        let _ = std::fs::remove_file(&patch_path);
        let post_change_hash = workspace_hash(&root)?;
        let provenance = GitPatchProvenance {
            provenance_ref: format!("git-provenance-{}", Uuid::new_v4()),
            actor_ref: request.actor_ref,
            affected_paths,
            pre_change_marker,
            post_change_hash,
            applied_at: chrono::Utc::now(),
            audit_ref: format!("audit://git/{}", trace.trace_id),
        };
        info!(provenance_ref = %provenance.provenance_ref, affected_count = provenance.affected_paths.len(), "git patch applied with provenance");
        Ok(GitServiceResponse::PatchApplied(provenance))
    }

    async fn rollback_marker(
        &self,
        request: GitRepositoryRequest,
    ) -> ServiceResult<GitServiceResponse> {
        Ok(GitServiceResponse::RollbackMarker(marker_for(
            &workspace_root(&request.workspace_root)?,
            4096,
        )?))
    }

    async fn rollback_replay(
        &self,
        request: GitRollbackMarkerReplayRequest,
    ) -> ServiceResult<GitServiceResponse> {
        let _root = workspace_root(&request.workspace_root)?;
        Ok(GitServiceResponse::RollbackReplay {
            marker_ref: request.marker_ref,
            applied: false,
            audit_ref: format!("audit://git/rollback/{}", Uuid::new_v4()),
        })
    }
}

fn marker_for(root: &Path, budget: usize) -> ServiceResult<GitRollbackMarker> {
    let diff = git(root, &["diff", "--binary", "HEAD", "--"])?;
    Ok(GitRollbackMarker {
        marker_ref: format!("git-rollback-{}", Uuid::new_v4()),
        workspace_hash: workspace_hash(root)?,
        inverse_patch: summary(&diff, budget),
        created_at: chrono::Utc::now(),
    })
}

fn affected_paths(patch_text: &str) -> ServiceResult<Vec<String>> {
    let mut paths = Vec::new();
    for line in patch_text.lines() {
        if let Some(rest) = line
            .strip_prefix("+++ b/")
            .or_else(|| line.strip_prefix("--- a/"))
        {
            if rest != "/dev/null" && !paths.iter().any(|item| item == rest) {
                paths.push(rest.to_string());
            }
        }
    }
    if paths.is_empty() {
        return Err(ServiceError::InvalidArgument(
            "git.apply_patch could not determine affected paths".into(),
        ));
    }
    Ok(paths)
}

fn git(root: &Path, args: &[&str]) -> ServiceResult<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(io_error)?;
    if !output.status.success() {
        return Err(ServiceError::AdapterFailure(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn workspace_root(workspace: &str) -> ServiceResult<PathBuf> {
    Path::new(workspace).canonicalize().map_err(io_error)
}

fn workspace_hash(root: &Path) -> ServiceResult<String> {
    let status = git(root, &["status", "--porcelain"])?;
    let head = git(root, &["rev-parse", "HEAD"]).unwrap_or_default();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    head.hash(&mut hasher);
    status.hash(&mut hasher);
    Ok(format!("{:016x}", hasher.finish()))
}

fn reject_unsafe_relative_path(path: &str) -> ServiceResult<()> {
    if path.trim().is_empty() {
        return Err(ServiceError::InvalidArgument("git path is empty".into()));
    }
    for component in Path::new(path).components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(ServiceError::DisabledByPolicy(
                    "absolute paths and traversal are denied".into(),
                ));
            }
        }
    }
    Ok(())
}

fn summary(text: &str, budget: usize) -> BoundedSummary {
    let budget = budget.max(1);
    let truncated = text.len() > budget;
    BoundedSummary {
        text: text.chars().take(budget).collect(),
        truncated,
        original_byte_len: Some(text.len() as u64),
        artifact_ref: None,
    }
}

fn decode<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> ServiceResult<T> {
    serde_json::from_value(value).map_err(|error| ServiceError::InvalidArgument(error.to_string()))
}

fn io_error(error: std::io::Error) -> ServiceError {
    ServiceError::AdapterFailure(error.to_string())
}

fn runtime_error(error: crate::ServiceRuntimeError) -> macaca_proto::MacacaError {
    macaca_proto::MacacaError::Config(error.to_string())
}
