//! Local filesystem Strategy used by `service.file`.
//!
//! The strategy owns host filesystem mechanics only.  The service provider
//! remains responsible for service lifecycle, trace, audit refs, and command
//! envelopes, while this module enforces workspace path specifications and
//! returns provider-neutral DTOs.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Component, Path, PathBuf};

use async_trait::async_trait;
use macaca_proto::workbench::file::*;
use macaca_proto::{
    ArtifactRef, BoundedSummary, ServiceError, ServiceResult, WorkbenchCommandStatus,
};
use uuid::Uuid;

const DIRECTORY_ENTRY_LIMIT: usize = 512;

/// Replaceable filesystem provider Strategy.
#[async_trait]
pub trait FileProvider: Send + Sync {
    async fn read(&self, request: FileReadRequest) -> ServiceResult<FileProviderRead>;
    async fn write(&self, request: FileWriteRequest) -> ServiceResult<FileServiceResponse>;
    async fn patch(&self, request: FilePatchRequest) -> ServiceResult<FileServiceResponse>;
    async fn copy(&self, request: FileCopyRequest) -> ServiceResult<FileServiceResponse>;
    async fn remove(&self, request: FilePathRequest) -> ServiceResult<FileServiceResponse>;
    async fn metadata(&self, request: FilePathRequest) -> ServiceResult<FileServiceResponse>;
    async fn list(&self, request: FilePathRequest) -> ServiceResult<FileServiceResponse>;
    async fn diff(&self, request: FileDiffRequest) -> ServiceResult<FileServiceResponse>;
}

/// Read result keeps binary/body classification separate from command status.
pub struct FileProviderRead {
    pub response: FileServiceResponse,
    pub summary: Option<BoundedSummary>,
    pub artifact_status: Option<WorkbenchCommandStatus>,
}

/// Local provider that applies workspace, traversal, and symlink policy.
#[derive(Debug, Default)]
pub struct LocalFileProvider;

#[async_trait]
impl FileProvider for LocalFileProvider {
    async fn read(&self, request: FileReadRequest) -> ServiceResult<FileProviderRead> {
        let budget = request.inline_budget_bytes.max(1);
        let resolved = resolve_path(&request.target, false)?;
        let bytes = tokio::fs::read(&resolved.absolute)
            .await
            .map_err(io_error)?;
        let metadata = metadata_for(&resolved.public_path, &resolved.absolute)?;
        let binary = std::str::from_utf8(&bytes).is_err();
        let over_budget = bytes.len() as u64 > budget;
        let content = if binary || over_budget {
            None
        } else {
            Some(String::from_utf8(bytes.clone()).map_err(|error| {
                ServiceError::AdapterFailure(format!("invalid utf-8 after binary check: {error}"))
            })?)
        };
        let summary = (binary || over_budget).then(|| artifact_summary(&bytes, "file.read"));
        Ok(FileProviderRead {
            response: FileServiceResponse::Read {
                path: resolved.public_path,
                content,
                binary,
                metadata,
            },
            artifact_status: summary
                .as_ref()
                .and_then(|value| value.artifact_ref.clone())
                .map(|artifact_ref| WorkbenchCommandStatus::ArtifactBacked { artifact_ref }),
            summary,
        })
    }

    async fn write(&self, request: FileWriteRequest) -> ServiceResult<FileServiceResponse> {
        if request.require_approval && request.approval_ref.is_none() {
            return Err(ServiceError::DisabledByPolicy(
                "file.write requires approval_ref before side effect".into(),
            ));
        }
        let resolved = resolve_path(&request.target, true)?;
        if request.create_parent_directories {
            if let Some(parent) = resolved.absolute.parent() {
                tokio::fs::create_dir_all(parent).await.map_err(io_error)?;
            }
        }
        deny_readonly_existing_file(&resolved.absolute)?;
        let memento = existing_memento(&resolved.public_path, &resolved.absolute).await?;
        tokio::fs::write(&resolved.absolute, request.content.as_bytes())
            .await
            .map_err(io_error)?;
        Ok(FileServiceResponse::Written {
            path: resolved.public_path.clone(),
            metadata: metadata_for(&resolved.public_path, &resolved.absolute)?,
            memento,
        })
    }

    async fn patch(&self, request: FilePatchRequest) -> ServiceResult<FileServiceResponse> {
        if request.require_approval && request.approval_ref.is_none() {
            return Err(ServiceError::DisabledByPolicy(
                "file.patch requires approval_ref before side effect".into(),
            ));
        }
        let resolved = resolve_path(&request.target, false)?;
        deny_readonly_existing_file(&resolved.absolute)?;
        let before = tokio::fs::read_to_string(&resolved.absolute)
            .await
            .map_err(io_error)?;
        let memento = memento_from_bytes(&resolved.public_path, before.as_bytes());
        let replaced_count = before.matches(&request.search).count() as u64;
        if replaced_count == 0 {
            return Err(ServiceError::InvalidArgument(
                "file.patch search text did not match target".into(),
            ));
        }
        let after = before.replace(&request.search, &request.replace);
        tokio::fs::write(&resolved.absolute, after.as_bytes())
            .await
            .map_err(io_error)?;
        Ok(FileServiceResponse::Patched {
            path: resolved.public_path.clone(),
            metadata: metadata_for(&resolved.public_path, &resolved.absolute)?,
            memento,
            replaced_count,
        })
    }

    async fn copy(&self, request: FileCopyRequest) -> ServiceResult<FileServiceResponse> {
        let source = resolve_path(&request.source, false)?;
        let destination = resolve_sibling_path(
            &request.source.workspace_root,
            &request.destination_path,
            request.source.follow_symlinks,
            true,
        )?;
        if destination.absolute.exists() && !request.overwrite {
            return Err(ServiceError::InvalidArgument(
                "file.copy destination exists and overwrite is false".into(),
            ));
        }
        tokio::fs::copy(&source.absolute, &destination.absolute)
            .await
            .map_err(io_error)?;
        Ok(FileServiceResponse::Copied {
            source_path: source.public_path,
            destination_path: destination.public_path.clone(),
            metadata: metadata_for(&destination.public_path, &destination.absolute)?,
        })
    }

    async fn remove(&self, request: FilePathRequest) -> ServiceResult<FileServiceResponse> {
        let resolved = resolve_path(&request, false)?;
        let memento = existing_memento(&resolved.public_path, &resolved.absolute).await?;
        if resolved.absolute.is_dir() {
            tokio::fs::remove_dir_all(&resolved.absolute)
                .await
                .map_err(io_error)?;
        } else {
            tokio::fs::remove_file(&resolved.absolute)
                .await
                .map_err(io_error)?;
        }
        Ok(FileServiceResponse::Removed {
            path: resolved.public_path,
            memento,
        })
    }

    async fn metadata(&self, request: FilePathRequest) -> ServiceResult<FileServiceResponse> {
        let resolved = resolve_path(&request, false)?;
        Ok(FileServiceResponse::Metadata(metadata_for(
            &resolved.public_path,
            &resolved.absolute,
        )?))
    }

    async fn list(&self, request: FilePathRequest) -> ServiceResult<FileServiceResponse> {
        let resolved = resolve_path(&request, false)?;
        let mut reader = tokio::fs::read_dir(&resolved.absolute)
            .await
            .map_err(io_error)?;
        let mut entries = Vec::new();
        while let Some(entry) = reader.next_entry().await.map_err(io_error)? {
            if entries.len() >= DIRECTORY_ENTRY_LIMIT {
                return Ok(FileServiceResponse::DirectoryList {
                    path: resolved.public_path,
                    entries,
                    truncated: true,
                });
            }
            let child = entry.path();
            let rel = child
                .strip_prefix(workspace_root(&request.workspace_root)?)
                .unwrap_or(&child)
                .to_string_lossy()
                .to_string();
            entries.push(FileEntry {
                path: rel.clone(),
                metadata: metadata_for(&rel, &child)?,
            });
        }
        Ok(FileServiceResponse::DirectoryList {
            path: resolved.public_path,
            entries,
            truncated: false,
        })
    }

    async fn diff(&self, request: FileDiffRequest) -> ServiceResult<FileServiceResponse> {
        let resolved = resolve_path(&request.target, false)?;
        let before = tokio::fs::read(&resolved.absolute)
            .await
            .map_err(io_error)?;
        let after = request.proposed_content.as_bytes();
        let before_text = String::from_utf8_lossy(&before);
        let after_text = request.proposed_content.as_str();
        Ok(FileServiceResponse::Diff(FileDiffSummary {
            changed: before != after,
            before_hash: Some(stable_hash(&before)),
            after_hash: stable_hash(after),
            line_delta: after_text.lines().count() as i64 - before_text.lines().count() as i64,
        }))
    }
}

#[derive(Debug)]
pub(crate) struct ResolvedPath {
    pub absolute: PathBuf,
    pub public_path: String,
}

pub(crate) fn resolve_path(
    request: &FilePathRequest,
    allow_missing_leaf: bool,
) -> ServiceResult<ResolvedPath> {
    resolve_sibling_path(
        &request.workspace_root,
        &request.path,
        request.follow_symlinks,
        allow_missing_leaf,
    )
}

fn resolve_sibling_path(
    workspace: &str,
    path: &str,
    follow_symlinks: bool,
    allow_missing_leaf: bool,
) -> ServiceResult<ResolvedPath> {
    let root = workspace_root(workspace)?;
    reject_unsafe_relative_path(path)?;
    let candidate = root.join(path);
    if !follow_symlinks && symlink_in_path(&root, Path::new(path))? {
        return Err(ServiceError::DisabledByPolicy(
            "symlink traversal is denied by service.file path policy".into(),
        ));
    }
    let absolute =
        if candidate.exists() {
            candidate.canonicalize().map_err(io_error)?
        } else if allow_missing_leaf {
            let parent = candidate
                .parent()
                .ok_or_else(|| ServiceError::InvalidArgument("file path has no parent".into()))?;
            let canonical_parent = parent.canonicalize().map_err(io_error)?;
            canonical_parent.join(candidate.file_name().ok_or_else(|| {
                ServiceError::InvalidArgument("file path has no file name".into())
            })?)
        } else {
            return Err(ServiceError::InvalidArgument(
                "file path does not exist".into(),
            ));
        };
    if !absolute.starts_with(&root) {
        return Err(ServiceError::DisabledByPolicy(
            "path escapes workspace root".into(),
        ));
    }
    Ok(ResolvedPath {
        absolute,
        public_path: path.trim_matches('/').to_string(),
    })
}

fn workspace_root(workspace: &str) -> ServiceResult<PathBuf> {
    Path::new(workspace).canonicalize().map_err(io_error)
}

fn reject_unsafe_relative_path(path: &str) -> ServiceResult<()> {
    if path.trim().is_empty() {
        return Err(ServiceError::InvalidArgument("file path is empty".into()));
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

fn symlink_in_path(root: &Path, relative: &Path) -> ServiceResult<bool> {
    let mut current = root.to_path_buf();
    for component in relative.components() {
        if let Component::Normal(part) = component {
            current.push(part);
            if let Ok(metadata) = std::fs::symlink_metadata(&current) {
                if metadata.file_type().is_symlink() {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

async fn existing_memento(path: &str, absolute: &Path) -> ServiceResult<Option<FileMementoRef>> {
    if absolute.exists() && absolute.is_file() {
        let bytes = tokio::fs::read(absolute).await.map_err(io_error)?;
        Ok(Some(memento_from_bytes(path, &bytes)))
    } else {
        Ok(None)
    }
}

fn memento_from_bytes(path: &str, bytes: &[u8]) -> FileMementoRef {
    FileMementoRef {
        id: format!("file-memento-{}", Uuid::new_v4()),
        path: path.into(),
        content_hash: Some(stable_hash(bytes)),
        byte_len: bytes.len() as u64,
    }
}

fn metadata_for(public_path: &str, absolute: &Path) -> ServiceResult<FileMetadata> {
    let metadata = std::fs::metadata(absolute).map_err(io_error)?;
    Ok(FileMetadata {
        path: public_path.into(),
        byte_len: metadata.len(),
        is_file: metadata.is_file(),
        is_dir: metadata.is_dir(),
        readonly: metadata.permissions().readonly(),
        modified_at: metadata
            .modified()
            .ok()
            .map(chrono::DateTime::<chrono::Utc>::from),
        content_hash: if metadata.is_file() {
            std::fs::read(absolute)
                .ok()
                .map(|bytes| stable_hash(&bytes))
        } else {
            None
        },
    })
}

fn deny_readonly_existing_file(absolute: &Path) -> ServiceResult<()> {
    if absolute.exists()
        && std::fs::metadata(absolute)
            .map_err(io_error)?
            .permissions()
            .readonly()
    {
        return Err(ServiceError::DisabledByPolicy(
            "read-only file write is denied by service.file policy".into(),
        ));
    }
    Ok(())
}

fn artifact_summary(bytes: &[u8], profile: &str) -> BoundedSummary {
    BoundedSummary {
        text: "[artifact-backed file payload]".into(),
        truncated: true,
        original_byte_len: Some(bytes.len() as u64),
        artifact_ref: Some(ArtifactRef {
            id: format!("file-artifact-{}", Uuid::new_v4()),
            media_type: "application/octet-stream".into(),
            sha256: None,
            byte_len: bytes.len() as u64,
            redaction_profile: profile.into(),
        }),
    }
}

fn stable_hash(bytes: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn io_error(error: std::io::Error) -> ServiceError {
    ServiceError::AdapterFailure(error.to_string())
}
