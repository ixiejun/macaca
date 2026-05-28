//! Local sandbox Strategy used by `service.sandbox`.
//!
//! The built-in provider records permission-profile resolution and runtime
//! environment lifecycle metadata.  It deliberately does not claim Docker, SSH,
//! browser, OS-specific, or WASM isolation when those providers are absent; it
//! returns explicit unavailable records so higher layers can degrade honestly.

use std::collections::HashMap;
use std::path::{Component, Path};
use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::workbench::sandbox::*;
use macaca_proto::{ServiceError, ServiceResult, WorkbenchCommandStatus};
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

/// Replaceable sandbox provider Strategy.
#[async_trait]
pub trait SandboxProvider: Send + Sync {
    async fn list_profiles(
        &self,
        request: SandboxProfileListRequest,
    ) -> ServiceResult<Vec<SandboxPermissionProfile>>;
    async fn resolve_profile(
        &self,
        request: SandboxProfileResolveRequest,
    ) -> ServiceResult<SandboxPermissionProfile>;
    async fn prepare_environment(
        &self,
        request: SandboxEnvironmentPrepareRequest,
        trace_id: String,
    ) -> ServiceResult<(SandboxEnvironmentRecord, WorkbenchCommandStatus)>;
    async fn environment_health(
        &self,
        request: SandboxEnvironmentHealthRequest,
    ) -> ServiceResult<Vec<SandboxEnvironmentRecord>>;
    async fn cleanup_environment(
        &self,
        request: SandboxEnvironmentCleanupRequest,
        trace_id: String,
    ) -> ServiceResult<(SandboxEnvironmentRecord, Vec<String>)>;
    async fn explain_policy(
        &self,
        request: SandboxPolicyExplainRequest,
    ) -> ServiceResult<Vec<SandboxPolicyDecision>>;
}

/// In-memory local provider for profile resolution and environment records.
#[derive(Clone, Default)]
pub struct LocalSandboxProvider {
    records: Arc<RwLock<HashMap<String, SandboxEnvironmentRecord>>>,
}

/// Test and fixture provider with the same deterministic behavior as the local
/// provider.
///
/// The alias keeps mock composition explicit without introducing a second
/// behavior path.  Tests can inject it through [`SandboxSystemServiceProvider`]
/// while production composition continues to use the local provider factory.
pub type MockSandboxProvider = LocalSandboxProvider;

#[async_trait]
impl SandboxProvider for LocalSandboxProvider {
    async fn list_profiles(
        &self,
        _request: SandboxProfileListRequest,
    ) -> ServiceResult<Vec<SandboxPermissionProfile>> {
        info!("sandbox local provider listed built-in permission profiles");
        Ok(SandboxPermissionProfile::catalog())
    }

    async fn resolve_profile(
        &self,
        request: SandboxProfileResolveRequest,
    ) -> ServiceResult<SandboxPermissionProfile> {
        let profile = resolve_profile_ref(&request.requested_profile_ref)?;
        ensure_profile_matches_runtime(&profile, &request.runtime_kind)?;
        info!(
            profile_ref = %profile.profile_ref,
            runtime_kind = ?request.runtime_kind,
            "sandbox local provider resolved permission profile"
        );
        Ok(profile)
    }

    async fn prepare_environment(
        &self,
        request: SandboxEnvironmentPrepareRequest,
        trace_id: String,
    ) -> ServiceResult<(SandboxEnvironmentRecord, WorkbenchCommandStatus)> {
        let profile = resolve_profile_ref(&request.requested_profile_ref)?;
        if let Some(root) = &request.workspace_root {
            validate_workspace_root(root)?;
        }
        if let Some(cwd) = &request.cwd {
            reject_unsafe_relative_path(cwd)?;
        }
        let unsupported = unavailable_optional_runtime(&request.runtime_kind);
        let now = chrono::Utc::now();
        let environment_id = format!("sandbox-env-{}", Uuid::new_v4());
        let state = if unsupported.is_some() {
            SandboxEnvironmentState::Unavailable
        } else {
            SandboxEnvironmentState::Ready
        };
        let leases = request
            .resource_request_refs
            .iter()
            .enumerate()
            .map(|(idx, requested)| SandboxResourceLease {
                lease_ref: format!("sandbox-lease-{idx}-{requested}"),
                resource_class: requested.clone(),
                released: false,
            })
            .collect::<Vec<_>>();
        let mut record = SandboxEnvironmentRecord {
            environment_id: environment_id.clone(),
            runtime_kind: request.runtime_kind.clone(),
            state,
            permission_profile: profile,
            scope: request.scope,
            workspace_root_ref: request.workspace_root,
            cwd_ref: request.cwd,
            resource_leases: leases,
            trace_refs: vec![trace_id.clone()],
            audit_refs: vec![format!("sandbox:prepare:{trace_id}")],
            created_at: now,
            updated_at: now,
            metadata: Default::default(),
        };
        if let Some(reason) = unsupported {
            record
                .metadata
                .insert("unavailable_reason".into(), reason.clone());
            warn!(
                environment_id = %environment_id,
                runtime_kind = ?record.runtime_kind,
                reason_code = "sandbox_provider_unavailable",
                "sandbox optional runtime provider is unavailable"
            );
            return Ok((record, WorkbenchCommandStatus::Unavailable { reason }));
        }
        self.records
            .write()
            .await
            .insert(environment_id.clone(), record.clone());
        info!(
            environment_id = %environment_id,
            profile_ref = %record.permission_profile.profile_ref,
            lease_count = record.resource_leases.len(),
            "sandbox local provider prepared runtime environment"
        );
        Ok((record, WorkbenchCommandStatus::Completed))
    }

    async fn environment_health(
        &self,
        request: SandboxEnvironmentHealthRequest,
    ) -> ServiceResult<Vec<SandboxEnvironmentRecord>> {
        let records = self.records.read().await;
        let selected = records
            .values()
            .filter(|record| {
                request
                    .environment_id
                    .as_ref()
                    .map_or(true, |id| &record.environment_id == id)
            })
            .cloned()
            .collect::<Vec<_>>();
        info!(
            requested_environment = ?request.environment_id,
            record_count = selected.len(),
            "sandbox local provider captured environment health"
        );
        Ok(selected)
    }

    async fn cleanup_environment(
        &self,
        request: SandboxEnvironmentCleanupRequest,
        trace_id: String,
    ) -> ServiceResult<(SandboxEnvironmentRecord, Vec<String>)> {
        let mut records = self.records.write().await;
        let record = records
            .get_mut(&request.environment_id)
            .ok_or_else(|| ServiceError::InvalidArgument("unknown sandbox environment".into()))?;
        record.state = SandboxEnvironmentState::Cleaned;
        record.updated_at = chrono::Utc::now();
        record.trace_refs.push(trace_id.clone());
        record.audit_refs.push(format!(
            "sandbox:cleanup:{trace_id}:{}",
            request.reason_code
        ));
        let released = record
            .resource_leases
            .iter_mut()
            .map(|lease| {
                lease.released = true;
                lease.lease_ref.clone()
            })
            .collect::<Vec<_>>();
        info!(
            environment_id = %request.environment_id,
            reason_code = %request.reason_code,
            released_lease_count = released.len(),
            "sandbox local provider cleaned runtime environment"
        );
        Ok((record.clone(), released))
    }

    async fn explain_policy(
        &self,
        request: SandboxPolicyExplainRequest,
    ) -> ServiceResult<Vec<SandboxPolicyDecision>> {
        let profile = resolve_profile_ref(&request.requested_profile_ref)?;
        let mut decisions = Vec::new();
        decisions.push(SandboxPolicyDecision {
            rule: "runtime_kind".into(),
            allowed: unavailable_optional_runtime(&request.runtime_kind).is_none(),
            reason: format!("{:?} provider availability checked", request.runtime_kind),
        });
        decisions.push(SandboxPolicyDecision {
            rule: "network".into(),
            allowed: !matches!(profile.network_mode, SandboxNetworkMode::Denied)
                || request.network_host_ref.is_none(),
            reason: format!("network mode is {:?}", profile.network_mode),
        });
        decisions.push(SandboxPolicyDecision {
            rule: "write_scope".into(),
            allowed: !request.write_requested
                || matches!(
                    profile.write_scope,
                    SandboxWriteScope::Workspace
                        | SandboxWriteScope::FullAccess
                        | SandboxWriteScope::RemoteEnvironment
                ),
            reason: format!("write scope is {:?}", profile.write_scope),
        });
        decisions.push(SandboxPolicyDecision {
            rule: "workspace_root".into(),
            allowed: request
                .workspace_root
                .as_deref()
                .map_or(true, |root| validate_workspace_root(root).is_ok()),
            reason: "workspace root must be absolute and canonicalizable".into(),
        });
        decisions.push(SandboxPolicyDecision {
            rule: "environment_variable".into(),
            allowed: request
                .env_var_key
                .as_deref()
                .map_or(true, |key| !key.to_ascii_uppercase().contains("SECRET")),
            reason: "secret-looking environment variables require secret service injection".into(),
        });
        info!(
            profile_ref = %profile.profile_ref,
            decision_count = decisions.len(),
            "sandbox local provider explained policy"
        );
        Ok(decisions)
    }
}

fn resolve_profile_ref(profile_ref: &str) -> ServiceResult<SandboxPermissionProfile> {
    resolve_builtin_permission_profile(profile_ref).ok_or_else(|| {
        ServiceError::DisabledByPolicy(format!("unknown permission profile: {profile_ref}"))
    })
}

fn ensure_profile_matches_runtime(
    profile: &SandboxPermissionProfile,
    runtime_kind: &SandboxRuntimeKind,
) -> ServiceResult<()> {
    if matches!(runtime_kind, SandboxRuntimeKind::RemoteEnvironment)
        && !profile.remote_environment_allowed
    {
        return Err(ServiceError::DisabledByPolicy(
            "permission profile does not allow remote environments".into(),
        ));
    }
    Ok(())
}

fn unavailable_optional_runtime(runtime_kind: &SandboxRuntimeKind) -> Option<String> {
    match runtime_kind {
        SandboxRuntimeKind::LocalWorkspace | SandboxRuntimeKind::LocalSandbox => None,
        SandboxRuntimeKind::Docker => Some("Docker sandbox provider is not installed".into()),
        SandboxRuntimeKind::SshRemote => Some("SSH sandbox provider is not installed".into()),
        SandboxRuntimeKind::OsSpecific => {
            Some("OS-specific sandbox provider is not installed".into())
        }
        SandboxRuntimeKind::Browser => Some("browser sandbox provider is not installed".into()),
        SandboxRuntimeKind::Wasm => Some("WASM sandbox provider is not installed".into()),
        SandboxRuntimeKind::RemoteEnvironment => {
            Some("remote environment sandbox provider is not installed".into())
        }
    }
}

fn validate_workspace_root(root: &str) -> ServiceResult<()> {
    Path::new(root)
        .canonicalize()
        .map(|_| ())
        .map_err(|error| ServiceError::InvalidArgument(error.to_string()))
}

fn reject_unsafe_relative_path(path: &str) -> ServiceResult<()> {
    for component in Path::new(path).components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(ServiceError::DisabledByPolicy(
                    "absolute cwd and traversal are denied by sandbox policy".into(),
                ));
            }
        }
    }
    Ok(())
}
