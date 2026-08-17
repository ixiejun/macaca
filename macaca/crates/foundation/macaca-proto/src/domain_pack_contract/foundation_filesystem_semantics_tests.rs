//! Tests for filesystem admission, approval, resource, and audit specifications.

use std::collections::BTreeSet;

use super::foundation_filesystem::FilesystemResultStatus;
use super::foundation_filesystem_semantics::*;

fn context() -> FilesystemPolicyContext {
    FilesystemPolicyContext {
        declared_scopes: BTreeSet::from([
            "filesystem.read".into(),
            "filesystem.write".into(),
            "filesystem.append".into(),
            "filesystem.list".into(),
            "filesystem.metadata".into(),
            "filesystem.copy".into(),
            "filesystem.move".into(),
            "filesystem.delete".into(),
            "filesystem.watch".into(),
            "filesystem.temp".into(),
            "filesystem.snapshot".into(),
            "filesystem.restore".into(),
        ]),
        policy_allowed: true,
        provider_available: true,
        supports_watch: true,
        supports_snapshot: true,
        supports_atomic_write: true,
        approval_granted: true,
        limits: FilesystemResourceLimits {
            max_byte_units: 256,
            max_entry_units: 32,
            max_recursive_operations: 2,
            max_watch_slots: 1,
            max_snapshot_units: 256,
            max_mutation_operations: 4,
            max_request_units: 4,
        },
        current: FilesystemResourceReservation::default(),
    }
}

fn request() -> FilesystemAdmissionRequest {
    FilesystemAdmissionRequest {
        byte_count: 16,
        entry_count: 1,
        has_safe_path: true,
        has_safe_content_reference: true,
        recursive: false,
        overwrite: false,
        non_temporary_root_mutation: false,
        requires_atomic_write: false,
    }
}

#[test]
fn denied_unavailable_and_quota_paths_never_call_a_provider() {
    let mut denied = context();
    denied.declared_scopes.clear();
    let mut called = false;
    assert_eq!(
        dispatch_after_preflight(
            preflight_command("filesystem.read_file", request(), &denied),
            || called = true,
        ),
        Err(FilesystemAdmissionFailure::PermissionNotDeclared)
    );
    assert!(!called);

    let mut unavailable = context();
    unavailable.provider_available = false;
    assert_eq!(
        preflight_command("filesystem.read_file", request(), &unavailable),
        Err(FilesystemAdmissionFailure::ProviderUnavailable)
    );

    let mut quota = context();
    quota.current.request_units = quota.limits.max_request_units;
    assert_eq!(
        preflight_command("filesystem.read_file", request(), &quota),
        Err(FilesystemAdmissionFailure::QuotaExceeded)
    );
}

#[test]
fn destructive_and_recursive_operations_require_approval_before_dispatch() {
    let mut denied = context();
    denied.approval_granted = false;
    assert_eq!(
        preflight_command("filesystem.delete_path", request(), &denied),
        Err(FilesystemAdmissionFailure::ApprovalRequired)
    );
    let mut recursive = request();
    recursive.recursive = true;
    assert_eq!(
        preflight_command("filesystem.copy_path", recursive, &denied),
        Err(FilesystemAdmissionFailure::ApprovalRequired)
    );
    assert_eq!(
        FilesystemAdmissionFailure::ApprovalRequired.status(),
        FilesystemResultStatus::Denied
    );
}

#[test]
fn audit_projection_contains_only_bounded_non_sensitive_command_facts() {
    let event =
        redacted_filesystem_audit_fields("filesystem.read_file", "trace:filesystem:1", 16, 1)
            .unwrap();
    let serialized = serde_json::to_string(&event).unwrap();
    assert!(event.paths_redacted && event.content_redacted);
    assert!(!serialized.contains("workspace"));
    assert!(
        redacted_filesystem_audit_fields("filesystem.read_file", "trace-path", 16, 1).is_none()
    );
}
