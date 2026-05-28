//! Focused tests for `service.git`.
//!
//! The tests exercise path policy, patch conflict handling, and provenance
//! output through the service boundary rather than calling Git from shell code.

use std::process::Command;

use macaca_kernel::SystemService;
use macaca_proto::workbench::git::*;
use macaca_proto::{
    ServiceCommand, ServiceCommandName, TraceContext, WorkbenchCommandResult,
    WorkbenchCommandStatus,
};

use crate::GitSystemServiceProvider;

fn command<T: serde::Serialize>(name: &str, payload: T) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        serde_json::to_value(payload).unwrap(),
        TraceContext::new(format!("trace-{name}")),
    )
}

fn decode(value: serde_json::Value) -> WorkbenchCommandResult<GitServiceResponse> {
    serde_json::from_value(value).unwrap()
}

fn git(root: &std::path::Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success(), "git command failed: {args:?}");
}

fn repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init"]);
    git(
        dir.path(),
        &["config", "user.email", "test@example.invalid"],
    );
    git(dir.path(), &["config", "user.name", "Macaca Test"]);
    std::fs::write(dir.path().join("README.md"), "hello\n").unwrap();
    git(dir.path(), &["add", "README.md"]);
    git(dir.path(), &["commit", "-m", "init"]);
    dir
}

#[tokio::test]
async fn apply_patch_records_provenance_and_status() {
    let dir = repo();
    let provider = GitSystemServiceProvider::mock();
    let patch = "\
diff --git a/README.md b/README.md
--- a/README.md
+++ b/README.md
@@ -1 +1 @@
-hello
+hello world
";

    let result = decode(
        provider
            .call(command(
                APPLY_PATCH_COMMAND,
                GitApplyPatchRequest {
                    workspace_root: dir.path().to_string_lossy().to_string(),
                    patch_text: patch.into(),
                    require_approval: false,
                    approval_ref: None,
                    actor_ref: "actor://test".into(),
                },
            ))
            .await
            .unwrap()
            .output,
    );

    let Some(GitServiceResponse::PatchApplied(provenance)) = result.output else {
        panic!("expected patch provenance");
    };
    assert!(matches!(result.status, WorkbenchCommandStatus::Completed));
    assert_eq!(provenance.affected_paths, vec!["README.md"]);
    assert!(provenance.audit_ref.contains("trace-git.apply_patch"));
    assert_eq!(
        std::fs::read_to_string(dir.path().join("README.md")).unwrap(),
        "hello world\n"
    );
}

#[tokio::test]
async fn path_policy_and_patch_conflicts_are_denied_before_mutation() {
    let dir = repo();
    let provider = GitSystemServiceProvider::mock();
    let escaping_patch = "\
diff --git a/../escape.txt b/../escape.txt
--- a/../escape.txt
+++ b/../escape.txt
@@ -1 +1 @@
-a
+b
";
    let denied = provider
        .call(command(
            APPLY_PATCH_COMMAND,
            GitApplyPatchRequest {
                workspace_root: dir.path().to_string_lossy().to_string(),
                patch_text: escaping_patch.into(),
                require_approval: false,
                approval_ref: None,
                actor_ref: "actor://test".into(),
            },
        ))
        .await;
    assert!(denied.is_err());

    let conflict = "\
diff --git a/README.md b/README.md
--- a/README.md
+++ b/README.md
@@ -1 +1 @@
-missing
+replacement
";
    let failed = provider
        .call(command(
            APPLY_PATCH_COMMAND,
            GitApplyPatchRequest {
                workspace_root: dir.path().to_string_lossy().to_string(),
                patch_text: conflict.into(),
                require_approval: false,
                approval_ref: None,
                actor_ref: "actor://test".into(),
            },
        ))
        .await;
    assert!(failed.is_err());
    assert_eq!(
        std::fs::read_to_string(dir.path().join("README.md")).unwrap(),
        "hello\n"
    );
}

#[tokio::test]
async fn rollback_marker_replay_returns_auditable_structured_result() {
    let dir = repo();
    let provider = GitSystemServiceProvider::mock();
    let marker = decode(
        provider
            .call(command(
                ROLLBACK_MARKER_CREATE_COMMAND,
                GitRepositoryRequest {
                    workspace_root: dir.path().to_string_lossy().to_string(),
                },
            ))
            .await
            .unwrap()
            .output,
    );
    let Some(GitServiceResponse::RollbackMarker(marker)) = marker.output else {
        panic!("expected rollback marker");
    };

    let replay = decode(
        provider
            .call(command(
                ROLLBACK_MARKER_REPLAY_COMMAND,
                GitRollbackMarkerReplayRequest {
                    workspace_root: dir.path().to_string_lossy().to_string(),
                    marker_ref: marker.marker_ref,
                },
            ))
            .await
            .unwrap()
            .output,
    );

    assert!(matches!(
        replay.output,
        Some(GitServiceResponse::RollbackReplay {
            applied: false,
            audit_ref,
            ..
        }) if audit_ref.starts_with("audit://git/rollback/")
    ));
}
