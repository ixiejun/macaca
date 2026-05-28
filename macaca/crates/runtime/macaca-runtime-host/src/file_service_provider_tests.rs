//! Focused tests for `service.file`.
//!
//! The tests exercise provider-owned path policy, approval-before-write,
//! artifact fallback, watch notification, and tool-service routing without
//! relying on any application-specific workflow.

use macaca_kernel::SystemService;
use macaca_proto::workbench::file::*;
use macaca_proto::{
    ServiceCommand, ServiceCommandName, ServiceError, TraceContext, WorkbenchCommandResult,
    WorkbenchCommandStatus,
};

use crate::FileSystemServiceProvider;

fn path(root: &tempfile::TempDir, path: &str) -> FilePathRequest {
    FilePathRequest {
        workspace_root: root.path().to_string_lossy().to_string(),
        path: path.into(),
        follow_symlinks: false,
    }
}

fn command<T: serde::Serialize>(name: &str, payload: T) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        serde_json::to_value(payload).unwrap(),
        TraceContext::new(format!("trace-{name}")),
    )
}

fn decode(value: serde_json::Value) -> WorkbenchCommandResult<FileServiceResponse> {
    serde_json::from_value(value).unwrap()
}

#[tokio::test]
async fn read_denies_path_traversal_before_filesystem_access() {
    let root = tempfile::tempdir().unwrap();
    let provider = FileSystemServiceProvider::local();

    let err = provider
        .call(command(
            READ_COMMAND,
            FileReadRequest {
                target: path(&root, "../outside.txt"),
                inline_budget_bytes: 1024,
            },
        ))
        .await
        .unwrap_err();

    assert!(matches!(err, ServiceError::DisabledByPolicy(_)));
}

#[tokio::test]
async fn read_denies_symlink_when_policy_disallows_following() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("target.txt"), "secret").unwrap();
    std::os::unix::fs::symlink(root.path().join("target.txt"), root.path().join("link.txt"))
        .unwrap();
    let provider = FileSystemServiceProvider::local();

    let err = provider
        .call(command(
            READ_COMMAND,
            FileReadRequest {
                target: path(&root, "link.txt"),
                inline_budget_bytes: 1024,
            },
        ))
        .await
        .unwrap_err();

    assert!(matches!(err, ServiceError::DisabledByPolicy(_)));
}

#[tokio::test]
async fn write_requires_approval_before_side_effect_when_requested() {
    let root = tempfile::tempdir().unwrap();
    let provider = FileSystemServiceProvider::local();

    let err = provider
        .call(command(
            WRITE_COMMAND,
            FileWriteRequest {
                target: path(&root, "new.txt"),
                content: "blocked".into(),
                create_parent_directories: false,
                require_approval: true,
                approval_ref: None,
            },
        ))
        .await
        .unwrap_err();

    assert!(matches!(err, ServiceError::DisabledByPolicy(_)));
    assert!(!root.path().join("new.txt").exists());
}

#[tokio::test]
async fn write_denies_readonly_existing_file_before_mutation() {
    let root = tempfile::tempdir().unwrap();
    let file_path = root.path().join("readonly.txt");
    std::fs::write(&file_path, "locked").unwrap();
    let mut permissions = std::fs::metadata(&file_path).unwrap().permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&file_path, permissions).unwrap();
    let provider = FileSystemServiceProvider::local();

    let err = provider
        .call(command(
            WRITE_COMMAND,
            FileWriteRequest {
                target: path(&root, "readonly.txt"),
                content: "changed".into(),
                create_parent_directories: false,
                require_approval: false,
                approval_ref: None,
            },
        ))
        .await
        .unwrap_err();

    let mut permissions = std::fs::metadata(&file_path).unwrap().permissions();
    permissions.set_readonly(false);
    std::fs::set_permissions(&file_path, permissions).unwrap();
    assert!(matches!(err, ServiceError::DisabledByPolicy(_)));
    assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "locked");
}

#[tokio::test]
async fn oversized_read_returns_artifact_backed_status() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("large.txt"), "abcdef").unwrap();
    let provider = FileSystemServiceProvider::local();

    let result = decode(
        provider
            .call(command(
                READ_COMMAND,
                FileReadRequest {
                    target: path(&root, "large.txt"),
                    inline_budget_bytes: 2,
                },
            ))
            .await
            .unwrap()
            .output,
    );

    assert!(matches!(
        result.status,
        WorkbenchCommandStatus::ArtifactBacked { .. }
    ));
    assert!(result.summary.unwrap().artifact_ref.is_some());
}

#[tokio::test]
async fn write_emits_bounded_watch_notification() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("watched.txt"), "old").unwrap();
    let provider = FileSystemServiceProvider::local();
    let mut rx = provider.subscribe();

    let watch = decode(
        provider
            .call(command(
                WATCH_COMMAND,
                FileWatchRequest {
                    target: path(&root, "watched.txt"),
                    recursive: false,
                },
            ))
            .await
            .unwrap()
            .output,
    );
    assert!(matches!(watch.output, Some(FileServiceResponse::Watch(_))));

    provider
        .call(command(
            WRITE_COMMAND,
            FileWriteRequest {
                target: path(&root, "watched.txt"),
                content: "new".into(),
                create_parent_directories: false,
                require_approval: false,
                approval_ref: None,
            },
        ))
        .await
        .unwrap();

    let event = rx.recv().await.unwrap();
    assert_eq!(event.path, "watched.txt");
    assert_eq!(event.change_kind, "written");
}

#[tokio::test]
async fn patch_copy_metadata_list_diff_and_remove_are_structured() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("source.txt"), "alpha\nbeta\n").unwrap();
    let provider = FileSystemServiceProvider::local();

    let patched = decode(
        provider
            .call(command(
                PATCH_COMMAND,
                FilePatchRequest {
                    target: path(&root, "source.txt"),
                    search: "beta".into(),
                    replace: "gamma".into(),
                    require_approval: false,
                    approval_ref: None,
                },
            ))
            .await
            .unwrap()
            .output,
    );
    assert!(matches!(
        patched.output,
        Some(FileServiceResponse::Patched {
            replaced_count: 1,
            ..
        })
    ));
    assert!(patched.audit_ref.unwrap().starts_with("file:"));

    let copied = decode(
        provider
            .call(command(
                COPY_COMMAND,
                FileCopyRequest {
                    source: path(&root, "source.txt"),
                    destination_path: "copy.txt".into(),
                    overwrite: false,
                },
            ))
            .await
            .unwrap()
            .output,
    );
    assert!(matches!(
        copied.output,
        Some(FileServiceResponse::Copied { .. })
    ));

    let listed = decode(
        provider
            .call(command(DIRECTORY_LIST_COMMAND, path(&root, ".")))
            .await
            .unwrap()
            .output,
    );
    assert!(matches!(
        listed.output,
        Some(FileServiceResponse::DirectoryList {
            truncated: false,
            ..
        })
    ));

    let diffed = decode(
        provider
            .call(command(
                DIFF_COMMAND,
                FileDiffRequest {
                    target: path(&root, "copy.txt"),
                    proposed_content: "changed\n".into(),
                },
            ))
            .await
            .unwrap()
            .output,
    );
    assert!(matches!(
        diffed.output,
        Some(FileServiceResponse::Diff(FileDiffSummary {
            changed: true,
            ..
        }))
    ));

    let removed = decode(
        provider
            .call(command(REMOVE_COMMAND, path(&root, "copy.txt")))
            .await
            .unwrap()
            .output,
    );
    assert!(matches!(
        removed.output,
        Some(FileServiceResponse::Removed {
            memento: Some(_),
            ..
        })
    ));
}
