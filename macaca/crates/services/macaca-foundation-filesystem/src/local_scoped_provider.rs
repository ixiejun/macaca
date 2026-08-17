//! Root-scoped local filesystem Strategy with no raw host-path exposure.
//!
//! The provider receives local root mappings only from a runtime-host composition
//! root. Calls carry provider-neutral DTOs; this module canonicalizes each root,
//! rejects traversal and symlink escapes, resolves opaque content references via
//! a separate adapter, and returns hashes/counters instead of bytes or paths.

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    CapabilityId, CleanupPolicy, DomainPackProviderCapabilityState, FilesystemAppendFileCommand,
    FilesystemContentRef, FilesystemCreateDirectoryCommand, FilesystemListDirectoryCommand,
    FilesystemMetadata, FilesystemPathRef, FilesystemProviderCapability,
    FilesystemProviderSnapshot, FilesystemReadFileCommand, FilesystemStatPathCommand,
    FilesystemWriteFileCommand, KernelServiceId, ServiceCallResult, ServiceCapability,
    ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, ServiceType,
    TraceSchemaRef, FOUNDATION_FILESYSTEM_SERVICE_ID,
};

use crate::FilesystemService;

const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_DIRECTORY_ENTRIES: u32 = 1_000;
const LOCAL_COMMANDS: &[&str] = &[
    "filesystem.read_file",
    "filesystem.write_file",
    "filesystem.append_file",
    "filesystem.list_directory",
    "filesystem.stat_path",
    "filesystem.create_directory",
];

/// Artifact/content adapter kept independent from host path sandboxing.
#[async_trait]
pub trait FilesystemContentResolver: Send + Sync {
    /// Resolve an opaque reference into bounded bytes inside the provider boundary.
    async fn resolve(
        &self,
        content: &FilesystemContentRef,
        max_bytes: u64,
    ) -> ServiceResult<Vec<u8>>;
}

/// Fail-closed resolver used until an artifact/content service is composed.
#[derive(Debug, Default)]
pub struct UnavailableFilesystemContentResolver;

#[async_trait]
impl FilesystemContentResolver for UnavailableFilesystemContentResolver {
    async fn resolve(
        &self,
        _content: &FilesystemContentRef,
        _max_bytes: u64,
    ) -> ServiceResult<Vec<u8>> {
        Err(ServiceError::ServiceUnavailable(
            "filesystem content resolver is not installed".into(),
        ))
    }
}

/// Local Strategy constructed only from declared logical root mappings.
pub struct LocalScopedWorkspaceFilesystemProvider {
    roots: BTreeMap<String, PathBuf>,
    content_resolver: Arc<dyn FilesystemContentResolver>,
}

impl LocalScopedWorkspaceFilesystemProvider {
    /// Canonicalize composition-root mappings before any application command arrives.
    pub fn new(
        roots: impl IntoIterator<Item = (String, PathBuf)>,
        content_resolver: Arc<dyn FilesystemContentResolver>,
    ) -> ServiceResult<Self> {
        let mut canonical_roots = BTreeMap::new();
        for (root_id, root) in roots {
            if !safe_reference(&root_id) || canonical_roots.contains_key(&root_id) {
                return Err(ServiceError::InvalidArgument(
                    "valid unique filesystem root id required".into(),
                ));
            }
            let canonical = root.canonicalize().map_err(io_error)?;
            if !canonical.is_dir() {
                return Err(ServiceError::InvalidArgument(
                    "filesystem root must be a directory".into(),
                ));
            }
            canonical_roots.insert(root_id, canonical);
        }
        if canonical_roots.is_empty() {
            return Err(ServiceError::InvalidArgument(
                "at least one filesystem root required".into(),
            ));
        }
        Ok(Self {
            roots: canonical_roots,
            content_resolver,
        })
    }

    /// Construct a local Strategy that fails closed for writes without artifact composition.
    pub fn with_unavailable_content_resolver(
        roots: impl IntoIterator<Item = (String, PathBuf)>,
    ) -> ServiceResult<Self> {
        Self::new(roots, Arc::new(UnavailableFilesystemContentResolver))
    }

    fn resolve_path(
        &self,
        path: &FilesystemPathRef,
        allow_missing: bool,
    ) -> ServiceResult<PathBuf> {
        let root = self.roots.get(&path.root.root_id).ok_or_else(|| {
            ServiceError::DisabledByPolicy(
                "filesystem root is not declared for this provider".into(),
            )
        })?;
        reject_unsafe_relative_path(&path.relative_path)?;
        let relative = Path::new(&path.relative_path);
        reject_symlinks(root, relative)?;
        let candidate = root.join(relative);
        let resolved = if candidate.exists() {
            candidate.canonicalize().map_err(io_error)?
        } else if allow_missing {
            let ancestor = canonical_existing_ancestor(&candidate)?;
            if !ancestor.starts_with(root) {
                return Err(ServiceError::DisabledByPolicy(
                    "filesystem path escapes declared root".into(),
                ));
            }
            candidate
        } else {
            return Err(ServiceError::InvalidArgument(
                "filesystem path does not exist".into(),
            ));
        };
        if !resolved.starts_with(root) {
            return Err(ServiceError::DisabledByPolicy(
                "filesystem path escapes declared root".into(),
            ));
        }
        Ok(resolved)
    }

    async fn read(&self, request: FilesystemReadFileCommand) -> ServiceResult<serde_json::Value> {
        let path = request.path.ok_or_else(|| {
            ServiceError::InvalidArgument("local provider requires a logical path".into())
        })?;
        let target = self.resolve_path(&path, false)?;
        let max_bytes = request.max_bytes.min(MAX_FILE_BYTES) as usize;
        let bytes = fs::read(target).map_err(io_error)?;
        let bounded = bytes.len().min(max_bytes);
        Ok(
            serde_json::json!({"status":"success","path_hash":stable_hash(&path.relative_path),"byte_count":bounded,"truncated":bytes.len()>bounded,"content_hash":stable_hash_bytes(&bytes[..bounded])}),
        )
    }

    async fn write(
        &self,
        request: FilesystemWriteFileCommand,
        append: bool,
    ) -> ServiceResult<serde_json::Value> {
        let target = self.resolve_path(&request.path, true)?;
        apply_file_conflict_policy(&target, request.conflict_mode)?;
        let bytes = self
            .content_resolver
            .resolve(&request.content, MAX_FILE_BYTES)
            .await?;
        if bytes.len() as u64 > MAX_FILE_BYTES {
            return Err(ServiceError::InvalidArgument(
                "filesystem content exceeds provider byte limit".into(),
            ));
        }
        if append {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&target)
                .and_then(|mut file| std::io::Write::write_all(&mut file, &bytes))
                .map_err(io_error)?;
        } else if request.atomic {
            atomic_write(&target, &bytes)?;
        } else {
            fs::write(&target, &bytes).map_err(io_error)?;
        }
        tracing::info!(service_id = FOUNDATION_FILESYSTEM_SERVICE_ID, path_hash = %stable_hash(&request.path.relative_path), byte_count = bytes.len(), "local scoped filesystem mutation completed");
        Ok(
            serde_json::json!({"status":"success","path_hash":stable_hash(&request.path.relative_path),"byte_count":bytes.len(),"atomic":request.atomic && !append}),
        )
    }

    fn list(&self, request: FilesystemListDirectoryCommand) -> ServiceResult<serde_json::Value> {
        if request.recursive {
            return Err(ServiceError::UnsupportedCommand(
                "filesystem.list_directory recursive local listing".into(),
            ));
        }
        let target = self.resolve_path(&request.path, false)?;
        let limit = request.page_size.clamp(1, MAX_DIRECTORY_ENTRIES) as usize;
        let mut entries = fs::read_dir(target).map_err(io_error)?.filter_map(Result::ok).take(limit + 1).map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            serde_json::json!({"path_hash":stable_hash(&name),"entry_kind":if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false){"directory"}else{"file"}})
        }).collect::<Vec<_>>();
        let truncated = entries.len() > limit;
        entries.truncate(limit);
        Ok(
            serde_json::json!({"status":if truncated{"partial_stream_page"}else{"success"},"entry_count":entries.len(),"entries":entries,"next_cursor":truncated.then(|| "opaque-next-page")}),
        )
    }

    fn stat(&self, request: FilesystemStatPathCommand) -> ServiceResult<serde_json::Value> {
        if request.follow_symlinks {
            return Err(ServiceError::DisabledByPolicy(
                "local scoped provider denies symlink traversal".into(),
            ));
        }
        let target = self.resolve_path(&request.path, false)?;
        let metadata = fs::symlink_metadata(target).map_err(io_error)?;
        let projection = FilesystemMetadata {
            path_hash: stable_hash(&request.path.relative_path),
            entry_kind: if metadata.is_dir() {
                "directory".into()
            } else {
                "file".into()
            },
            size_bytes: metadata.is_file().then_some(metadata.len()),
            revision_id: None,
        };
        serde_json::to_value(projection)
            .map_err(|error| ServiceError::AdapterFailure(error.to_string()))
    }
}

#[async_trait]
impl FilesystemService for LocalScopedWorkspaceFilesystemProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        descriptor()
    }
    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)?;
        let output = match command.name.as_str() {
            "filesystem.read_file" => self.read(decode(command.payload)?).await?,
            "filesystem.write_file" => self.write(decode(command.payload)?, false).await?,
            "filesystem.append_file" => {
                let append: FilesystemAppendFileCommand = decode(command.payload)?;
                self.write(
                    FilesystemWriteFileCommand {
                        path: append.path,
                        content: append.content,
                        conflict_mode: macaca_proto::FilesystemConflictMode::Overwrite,
                        atomic: false,
                    },
                    true,
                )
                .await?
            }
            "filesystem.list_directory" => self.list(decode(command.payload)?)?,
            "filesystem.stat_path" => self.stat(decode(command.payload)?)?,
            "filesystem.create_directory" => {
                let request: FilesystemCreateDirectoryCommand = decode(command.payload)?;
                let target = self.resolve_path(&request.path, true)?;
                apply_directory_conflict_policy(&target, request.conflict_mode)?;
                if request.recursive {
                    fs::create_dir_all(target).map_err(io_error)?;
                } else {
                    fs::create_dir(target).map_err(io_error)?;
                }
                serde_json::json!({"status":"success","path_hash":stable_hash(&request.path.relative_path)})
            }
            other => return Err(ServiceError::UnsupportedCommand(other.into())),
        };
        Ok(ServiceCallResult {
            output,
            trace,
            status: "ok".into(),
            metadata: BTreeMap::from([
                (
                    "replay.provider_class".into(),
                    "local_scoped_workspace".into(),
                ),
                ("replay.filesystem_command".into(), command.name.to_string()),
                (
                    "filesystem.redaction".into(),
                    "host_paths_and_content_redacted".into(),
                ),
            ]),
            cleanup_hint: Some(CleanupPolicy::OnStop),
        })
    }
    fn health(&self) -> ServiceHealth {
        ServiceHealth::Healthy
    }
    fn snapshot(&self) -> FilesystemProviderSnapshot {
        FilesystemProviderSnapshot {
            descriptor_hash: "foundation-filesystem-local-scoped-v1".into(),
            provider_class: "local_scoped_workspace".into(),
            open_handle_count: 0,
            active_watch_count: 0,
            root_hashes: self
                .roots
                .keys()
                .map(|id| (stable_hash(id), "local-root-v1".into()))
                .collect(),
        }
    }
    fn provider_capabilities(&self) -> FilesystemProviderCapability {
        FilesystemProviderCapability {
            provider_class: "local_scoped_workspace".into(),
            supported_commands: LOCAL_COMMANDS.iter().map(|item| (*item).into()).collect(),
            supported_root_kinds: ["app_workspace", "session_workspace", "temporary"]
                .into_iter()
                .map(String::from)
                .collect(),
            supports_recursive_operations: false,
            supports_watch: false,
            supports_snapshot: false,
            supports_atomic_write: true,
            max_file_bytes: MAX_FILE_BYTES,
            max_directory_entries: MAX_DIRECTORY_ENTRIES,
            availability: DomainPackProviderCapabilityState::Available,
            unavailable_reason: None,
        }
    }
    async fn shutdown(&self) -> ServiceResult<()> {
        tracing::info!(
            service_id = FOUNDATION_FILESYSTEM_SERVICE_ID,
            "local scoped filesystem provider shut down"
        );
        Ok(())
    }
}

fn descriptor() -> ServiceDescriptor {
    let mut value = ServiceDescriptor::new(
        KernelServiceId::new(FOUNDATION_FILESYSTEM_SERVICE_ID),
        ServiceType::new("foundation.filesystem"),
        TraceSchemaRef::new("macaca.trace.foundation.filesystem.v1"),
    );
    value.health = ServiceHealth::Healthy;
    value.cleanup_policy = CleanupPolicy::OnStop;
    value.capabilities = LOCAL_COMMANDS
        .iter()
        .map(|name| {
            ServiceCapability::new(CapabilityId::new(*name), "local scoped filesystem command")
        })
        .collect();
    value
}
fn decode<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> ServiceResult<T> {
    serde_json::from_value(value)
        .map_err(|_| ServiceError::InvalidArgument("invalid filesystem command payload".into()))
}
fn reject_unsafe_relative_path(path: &str) -> ServiceResult<()> {
    if path.is_empty() || path.len() > 512 || path.contains('\\') {
        return Err(ServiceError::DisabledByPolicy(
            "filesystem path must be bounded slash-separated relative path".into(),
        ));
    }
    for component in Path::new(path).components() {
        if !matches!(component, Component::Normal(_) | Component::CurDir) {
            return Err(ServiceError::DisabledByPolicy(
                "absolute paths and traversal are denied".into(),
            ));
        }
    }
    Ok(())
}
fn reject_symlinks(root: &Path, relative: &Path) -> ServiceResult<()> {
    let mut current = root.to_path_buf();
    for component in relative.components() {
        if let Component::Normal(part) = component {
            current.push(part);
            if fs::symlink_metadata(&current)
                .map(|metadata| metadata.file_type().is_symlink())
                .unwrap_or(false)
            {
                return Err(ServiceError::DisabledByPolicy(
                    "filesystem symlink traversal is denied".into(),
                ));
            }
        }
    }
    Ok(())
}
fn atomic_write(target: &Path, bytes: &[u8]) -> ServiceResult<()> {
    let parent = target
        .parent()
        .ok_or_else(|| ServiceError::InvalidArgument("filesystem path has no parent".into()))?;
    let temp = parent.join(format!(
        ".macaca-write-{}",
        stable_hash(&format!("{}:{}", target.display(), bytes.len()))
    ));
    fs::write(&temp, bytes).map_err(io_error)?;
    fs::rename(temp, target).map_err(io_error)
}

/// Return the nearest existing ancestor so recursive creation cannot bypass root checks.
fn canonical_existing_ancestor(candidate: &Path) -> ServiceResult<PathBuf> {
    let mut ancestor = candidate;
    while !ancestor.exists() {
        ancestor = ancestor.parent().ok_or_else(|| {
            ServiceError::InvalidArgument("filesystem path has no existing ancestor".into())
        })?;
    }
    ancestor.canonicalize().map_err(io_error)
}

/// Enforce provider-neutral conflict choices before a local file mutation occurs.
fn apply_file_conflict_policy(
    target: &Path,
    mode: macaca_proto::FilesystemConflictMode,
) -> ServiceResult<()> {
    use macaca_proto::FilesystemConflictMode::{CreateNew, Fail, Overwrite};

    if target.exists() && matches!(mode, Fail | CreateNew) {
        return Err(ServiceError::InvalidArgument(
            "filesystem destination already exists".into(),
        ));
    }
    if !matches!(mode, Fail | CreateNew | Overwrite) {
        return Err(ServiceError::UnsupportedCommand(
            "filesystem file conflict mode".into(),
        ));
    }
    Ok(())
}

/// Enforce directory-specific conflict choices before local directory creation.
fn apply_directory_conflict_policy(
    target: &Path,
    mode: macaca_proto::FilesystemConflictMode,
) -> ServiceResult<()> {
    use macaca_proto::FilesystemConflictMode::{CreateNew, Fail, MergeDirectory};

    if target.exists() && !matches!(mode, MergeDirectory) {
        return Err(ServiceError::InvalidArgument(
            "filesystem directory already exists".into(),
        ));
    }
    if !matches!(mode, Fail | CreateNew | MergeDirectory) {
        return Err(ServiceError::UnsupportedCommand(
            "filesystem directory conflict mode".into(),
        ));
    }
    Ok(())
}
fn stable_hash(value: &str) -> String {
    stable_hash_bytes(value.as_bytes())
}
fn stable_hash_bytes(value: &[u8]) -> String {
    format!(
        "{:016x}",
        value.iter().fold(0_u64, |state, byte| state
            .wrapping_mul(1099511628211)
            .wrapping_add(u64::from(*byte)))
    )
}
fn safe_reference(value: &str) -> bool {
    !value.is_empty() && value.len() <= 128 && value.is_ascii() && !value.contains(['/', '\\'])
}
fn io_error(error: std::io::Error) -> ServiceError {
    ServiceError::AdapterFailure(format!(
        "local scoped filesystem operation failed: {}",
        error.kind()
    ))
}
