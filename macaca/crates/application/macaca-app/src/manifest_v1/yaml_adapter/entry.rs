//! Entry-point and permission extraction helpers for YAML manifest projection.
//!
//! These pure functions interpret legacy YAML entry declarations and aggregate
//! application-level permissions from resolved agents without embedding
//! application-specific routing logic.

use macaca_proto::ApplicationPermissionDeclaration;
use macaca_sdk::AgentConfig;

use crate::model::{AppManifest, EntrypointType};

/// Resolve the effective entry identifier from legacy YAML entry fields.
///
/// `entry_agent` takes precedence over `entrypoint.name` when both are present.
pub(super) fn entry_value(manifest: &AppManifest) -> Option<String> {
    if let Some(entry_agent) = &manifest.entry_agent {
        return Some(entry_agent.clone());
    }
    manifest
        .entrypoint
        .as_ref()
        .map(|entrypoint| match entrypoint.type_ {
            EntrypointType::Workflow | EntrypointType::Agent | EntrypointType::Custom => {
                entrypoint.name.clone()
            }
        })
}

/// Resolve the entry kind label stored as sanitized Manifest v1 metadata.
pub(super) fn entry_kind(manifest: &AppManifest) -> Option<String> {
    if manifest.entry_agent.is_some() {
        return Some("agent".into());
    }
    manifest
        .entrypoint
        .as_ref()
        .map(|entrypoint| match entrypoint.type_ {
            EntrypointType::Workflow => "workflow".into(),
            EntrypointType::Agent => "agent".into(),
            EntrypointType::Custom => "custom".into(),
        })
}

/// Normalize a filesystem path into a stable ability id fragment.
///
/// Non-alphanumeric characters become `.` so ability ids remain URL-safe and
/// deterministic across platforms.
pub(super) fn sanitize_path_for_id(path: &str) -> String {
    path.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '.'
            }
        })
        .collect::<String>()
        .trim_matches('.')
        .to_string()
}

/// Aggregate application-level permissions implied by resolved YAML agents.
pub(super) fn application_permissions(
    agents: &[AgentConfig],
) -> Vec<ApplicationPermissionDeclaration> {
    let mut permissions = Vec::new();
    if agents.iter().any(|agent| agent.network_access) {
        permissions.push(ApplicationPermissionDeclaration::required(
            "network.access",
            "At least one YAML agent declares network access",
        ));
    }
    if agents.iter().any(|agent| !agent.allowed_paths.is_empty()) {
        permissions.push(ApplicationPermissionDeclaration::required(
            "filesystem.scoped",
            "At least one YAML agent declares scoped filesystem paths",
        ));
    }
    permissions
}
