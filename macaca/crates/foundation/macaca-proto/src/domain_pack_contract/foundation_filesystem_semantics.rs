//! Provider-neutral admission, approval, resource, and audit rules for filesystem commands.
//!
//! The Specification pattern in this module evaluates only bounded command facts
//! and policy evidence before a provider is selected. It never receives a host
//! path, file bytes, handle token, or provider-native error payload. A runtime
//! Decorator can use these pure functions to prove that rejected requests never
//! reach a filesystem provider.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::foundation_filesystem::FilesystemResultStatus;

const MAX_AUDIT_REFERENCE: usize = 160;

/// Resource units reserved before a filesystem request crosses the provider boundary.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemResourceReservation {
    pub byte_units: u64,
    pub entry_units: u32,
    pub recursive_operations: u32,
    pub watch_slots: u32,
    pub snapshot_units: u64,
    pub mutation_operations: u32,
    pub request_units: u32,
}

/// Policy-owned ceilings applied per caller scope by the service resource decorator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemResourceLimits {
    pub max_byte_units: u64,
    pub max_entry_units: u32,
    pub max_recursive_operations: u32,
    pub max_watch_slots: u32,
    pub max_snapshot_units: u64,
    pub max_mutation_operations: u32,
    pub max_request_units: u32,
}

/// Sanitized policy facts supplied by application admission and service decorators.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemPolicyContext {
    pub declared_scopes: BTreeSet<String>,
    pub policy_allowed: bool,
    pub provider_available: bool,
    pub supports_watch: bool,
    pub supports_snapshot: bool,
    pub supports_atomic_write: bool,
    pub approval_granted: bool,
    pub limits: FilesystemResourceLimits,
    pub current: FilesystemResourceReservation,
}

/// Bounded command facts evaluated without interpreting file paths or content bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemAdmissionRequest {
    pub byte_count: u64,
    pub entry_count: u32,
    pub has_safe_path: bool,
    pub has_safe_content_reference: bool,
    pub recursive: bool,
    pub overwrite: bool,
    pub non_temporary_root_mutation: bool,
    pub requires_atomic_write: bool,
}

/// Stable pre-provider rejection reasons that can be exposed without host details.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilesystemAdmissionFailure {
    PermissionNotDeclared,
    PolicyDenied,
    ApprovalRequired,
    ProviderUnavailable,
    InvalidPath,
    Unsupported,
    QuotaExceeded,
}

impl FilesystemAdmissionFailure {
    /// Convert a private admission decision into the public result status.
    pub fn status(self) -> FilesystemResultStatus {
        match self {
            Self::PermissionNotDeclared | Self::PolicyDenied | Self::ApprovalRequired => {
                FilesystemResultStatus::Denied
            }
            Self::ProviderUnavailable => FilesystemResultStatus::Unavailable,
            Self::InvalidPath => FilesystemResultStatus::InvalidPath,
            Self::Unsupported => FilesystemResultStatus::Unsupported,
            Self::QuotaExceeded => FilesystemResultStatus::QuotaExceeded,
        }
    }
}

/// Validate a descriptor-declared command before selecting or invoking a provider.
pub fn preflight_command(
    command: &str,
    request: FilesystemAdmissionRequest,
    context: &FilesystemPolicyContext,
) -> Result<FilesystemResourceReservation, FilesystemAdmissionFailure> {
    let scope = required_scope(command).ok_or(FilesystemAdmissionFailure::Unsupported)?;
    require_scope(context, scope)?;
    if !context.provider_available {
        return Err(FilesystemAdmissionFailure::ProviderUnavailable);
    }
    if command_requires_path(command) && !request.has_safe_path {
        return Err(FilesystemAdmissionFailure::InvalidPath);
    }
    if command_requires_content(command) && !request.has_safe_content_reference {
        return Err(FilesystemAdmissionFailure::InvalidPath);
    }
    if (command == "filesystem.watch_path" && !context.supports_watch)
        || (matches!(
            command,
            "filesystem.snapshot_tree" | "filesystem.restore_snapshot"
        ) && !context.supports_snapshot)
        || (request.requires_atomic_write && !context.supports_atomic_write)
    {
        return Err(FilesystemAdmissionFailure::Unsupported);
    }
    if requires_approval(command, request) && !context.approval_granted {
        return Err(FilesystemAdmissionFailure::ApprovalRequired);
    }
    reserve(
        context,
        FilesystemResourceReservation {
            byte_units: request.byte_count,
            entry_units: request.entry_count,
            recursive_operations: u32::from(request.recursive),
            watch_slots: u32::from(command == "filesystem.watch_path"),
            snapshot_units: if command == "filesystem.snapshot_tree" {
                request.byte_count
            } else {
                0
            },
            mutation_operations: u32::from(is_mutation(command)),
            request_units: 1,
        },
    )
}

/// Invoke a provider closure only when preflight admitted the command.
pub fn dispatch_after_preflight<T>(
    decision: Result<FilesystemResourceReservation, FilesystemAdmissionFailure>,
    provider: impl FnOnce() -> T,
) -> Result<T, FilesystemAdmissionFailure> {
    decision.map(|_| provider())
}

/// Calculate a reservation without persisting counters in the protocol layer.
pub fn reserve(
    context: &FilesystemPolicyContext,
    requested: FilesystemResourceReservation,
) -> Result<FilesystemResourceReservation, FilesystemAdmissionFailure> {
    let next = FilesystemResourceReservation {
        byte_units: context
            .current
            .byte_units
            .saturating_add(requested.byte_units),
        entry_units: context
            .current
            .entry_units
            .saturating_add(requested.entry_units),
        recursive_operations: context
            .current
            .recursive_operations
            .saturating_add(requested.recursive_operations),
        watch_slots: context
            .current
            .watch_slots
            .saturating_add(requested.watch_slots),
        snapshot_units: context
            .current
            .snapshot_units
            .saturating_add(requested.snapshot_units),
        mutation_operations: context
            .current
            .mutation_operations
            .saturating_add(requested.mutation_operations),
        request_units: context
            .current
            .request_units
            .saturating_add(requested.request_units),
    };
    if next.byte_units > context.limits.max_byte_units
        || next.entry_units > context.limits.max_entry_units
        || next.recursive_operations > context.limits.max_recursive_operations
        || next.watch_slots > context.limits.max_watch_slots
        || next.snapshot_units > context.limits.max_snapshot_units
        || next.mutation_operations > context.limits.max_mutation_operations
        || next.request_units > context.limits.max_request_units
    {
        return Err(FilesystemAdmissionFailure::QuotaExceeded);
    }
    Ok(next)
}

/// Create a bounded audit projection that omits roots, paths, and content.
pub fn redacted_filesystem_audit_fields(
    command_name: &str,
    trace_id: &str,
    byte_count: u64,
    entry_count: u32,
) -> Option<FilesystemAuditFields> {
    (safe(command_name) && safe(trace_id)).then(|| FilesystemAuditFields {
        command_name: command_name.into(),
        trace_id: trace_id.into(),
        byte_count,
        entry_count,
        paths_redacted: true,
        content_redacted: true,
    })
}

/// Sanitized observer payload without host paths, handles, or file content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemAuditFields {
    pub command_name: String,
    pub trace_id: String,
    pub byte_count: u64,
    pub entry_count: u32,
    pub paths_redacted: bool,
    pub content_redacted: bool,
}

fn required_scope(command: &str) -> Option<&'static str> {
    match command {
        "filesystem.open_handle" | "filesystem.close_handle" | "filesystem.read_file" => {
            Some("filesystem.read")
        }
        "filesystem.write_file" | "filesystem.create_directory" => Some("filesystem.write"),
        "filesystem.append_file" => Some("filesystem.append"),
        "filesystem.list_directory" => Some("filesystem.list"),
        "filesystem.stat_path" => Some("filesystem.metadata"),
        "filesystem.copy_path" => Some("filesystem.copy"),
        "filesystem.move_path" => Some("filesystem.move"),
        "filesystem.delete_path" => Some("filesystem.delete"),
        "filesystem.watch_path" => Some("filesystem.watch"),
        "filesystem.create_temp" => Some("filesystem.temp"),
        "filesystem.snapshot_tree" => Some("filesystem.snapshot"),
        "filesystem.restore_snapshot" => Some("filesystem.restore"),
        _ => None,
    }
}

fn command_requires_path(command: &str) -> bool {
    !matches!(
        command,
        "filesystem.create_temp" | "filesystem.snapshot_tree"
    )
}

fn command_requires_content(command: &str) -> bool {
    matches!(command, "filesystem.write_file" | "filesystem.append_file")
}

fn is_mutation(command: &str) -> bool {
    matches!(
        command,
        "filesystem.write_file"
            | "filesystem.append_file"
            | "filesystem.create_directory"
            | "filesystem.copy_path"
            | "filesystem.move_path"
            | "filesystem.delete_path"
            | "filesystem.restore_snapshot"
    )
}

fn requires_approval(command: &str, request: FilesystemAdmissionRequest) -> bool {
    command == "filesystem.delete_path"
        || command == "filesystem.restore_snapshot"
        || request.overwrite
        || request.recursive
        || (is_mutation(command) && request.non_temporary_root_mutation)
}

fn require_scope(
    context: &FilesystemPolicyContext,
    scope: &str,
) -> Result<(), FilesystemAdmissionFailure> {
    if !context.declared_scopes.contains(scope) {
        return Err(FilesystemAdmissionFailure::PermissionNotDeclared);
    }
    if !context.policy_allowed {
        return Err(FilesystemAdmissionFailure::PolicyDenied);
    }
    Ok(())
}

fn safe(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_AUDIT_REFERENCE
        && !value.chars().any(char::is_control)
        && !["path", "content", "secret", "credential", "payload"]
            .iter()
            .any(|term| value.to_ascii_lowercase().contains(term))
}
