//! Safety tests for the root-scoped local filesystem Strategy.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    FilesystemConflictMode, FilesystemContentRef, FilesystemPathRef, FilesystemRootRef,
    FilesystemStatPathCommand, FilesystemWriteFileCommand, ServiceCommand, ServiceCommandName,
    ServiceError, TraceContext,
};

use crate::{FilesystemContentResolver, FilesystemService, LocalScopedWorkspaceFilesystemProvider};

#[derive(Default)]
struct StaticContentResolver {
    content: BTreeMap<String, Vec<u8>>,
}

#[async_trait]
impl FilesystemContentResolver for StaticContentResolver {
    async fn resolve(
        &self,
        content: &FilesystemContentRef,
        max_bytes: u64,
    ) -> macaca_proto::ServiceResult<Vec<u8>> {
        let bytes = self
            .content
            .get(&content.content_ref)
            .cloned()
            .ok_or_else(|| {
                ServiceError::ServiceUnavailable("test content is unavailable".into())
            })?;
        if bytes.len() as u64 > max_bytes {
            return Err(ServiceError::InvalidArgument(
                "test content exceeds limit".into(),
            ));
        }
        Ok(bytes)
    }
}

fn root() -> FilesystemRootRef {
    FilesystemRootRef {
        root_id: "workspace".into(),
        root_kind: "app_workspace".into(),
    }
}

fn path(relative_path: &str) -> FilesystemPathRef {
    FilesystemPathRef {
        root: root(),
        relative_path: relative_path.into(),
    }
}

fn command(name: &str, payload: impl serde::Serialize) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        serde_json::to_value(payload).unwrap(),
        TraceContext::new(format!("trace-{name}")),
    )
}

#[tokio::test]
async fn local_provider_resolves_opaque_content_and_redacts_host_paths_and_bytes() {
    let workspace = tempfile::tempdir().unwrap();
    let resolver = StaticContentResolver {
        content: [(
            "artifact:document".into(),
            b"private file contents".to_vec(),
        )]
        .into_iter()
        .collect(),
    };
    let provider = LocalScopedWorkspaceFilesystemProvider::new(
        [("workspace".into(), PathBuf::from(workspace.path()))],
        Arc::new(resolver),
    )
    .unwrap();
    let write = FilesystemWriteFileCommand {
        path: path("document.txt"),
        content: FilesystemContentRef {
            content_ref: "artifact:document".into(),
            encoding: None,
            expected_hash: None,
        },
        conflict_mode: FilesystemConflictMode::Overwrite,
        atomic: true,
    };
    provider
        .call(command("filesystem.write_file", write))
        .await
        .unwrap();
    assert_eq!(
        std::fs::read(workspace.path().join("document.txt")).unwrap(),
        b"private file contents"
    );

    let read = provider
        .call(command(
            "filesystem.read_file",
            macaca_proto::FilesystemReadFileCommand {
                path: Some(path("document.txt")),
                handle: None,
                range_start: 0,
                max_bytes: 1024,
            },
        ))
        .await
        .unwrap();
    let observation = format!("{:?}", read);
    assert!(!observation.contains("private file contents"));
    assert!(!observation.contains(&workspace.path().display().to_string()));
}

#[tokio::test]
async fn local_provider_rejects_absolute_traversal_and_backslash_paths_before_io() {
    let workspace = tempfile::tempdir().unwrap();
    let provider = LocalScopedWorkspaceFilesystemProvider::with_unavailable_content_resolver([(
        "workspace".into(),
        PathBuf::from(workspace.path()),
    )])
    .unwrap();
    for unsafe_path in ["/private/host", "../outside", "nested\\outside"] {
        let result = provider
            .call(command(
                "filesystem.stat_path",
                FilesystemStatPathCommand {
                    path: path(unsafe_path),
                    follow_symlinks: false,
                },
            ))
            .await;
        assert!(matches!(result, Err(ServiceError::DisabledByPolicy(_))));
    }
}

#[cfg(unix)]
#[tokio::test]
async fn local_provider_rejects_symlink_escapes_before_reading() {
    use std::os::unix::fs::symlink;

    let workspace = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("private.txt"), "outside").unwrap();
    symlink(outside.path(), workspace.path().join("escape")).unwrap();
    let provider = LocalScopedWorkspaceFilesystemProvider::with_unavailable_content_resolver([(
        "workspace".into(),
        PathBuf::from(workspace.path()),
    )])
    .unwrap();
    let result = provider
        .call(command(
            "filesystem.stat_path",
            FilesystemStatPathCommand {
                path: path("escape/private.txt"),
                follow_symlinks: false,
            },
        ))
        .await;
    assert!(matches!(result, Err(ServiceError::DisabledByPolicy(_))));
}
