//! Lifecycle command handlers for Application Service.
//!
//! **Pattern:** Command handlers — each method decodes one typed proto command,
//! delegates to `macaca-app` registry/runtime primitives, and shapes a replayable
//! service result with trace-preserving audit metadata.

use std::path::Path;

use macaca_app::{
    app_manifest_to_heartbeat_agent_views, app_manifest_to_metadata_view_with_catalog, AppLoader,
};
use macaca_proto::{
    ApplicationDiscoverCommand, ApplicationDiscoverResult, ApplicationHeartbeatAgentsQueryCommand,
    ApplicationHeartbeatAgentsResult, ApplicationLoadCommand, ApplicationMetadataQueryCommand,
    ApplicationMetadataResult, ApplicationRemoveCommand, ApplicationServiceSnapshot,
    ApplicationServiceUnavailable, ApplicationSnapshotCommand, ApplicationStartCommand,
    ApplicationStatusCommand, ApplicationStatusResult, ApplicationStopCommand, ServiceCommand,
    ServiceError, ServiceResult, APPLICATION_LOAD_COMMAND,
};

use super::support::{decode, discovered_view, manifest_view, running_status_for, service_adapter_error, to_value};
use super::ApplicationSystemServiceProvider;

impl ApplicationSystemServiceProvider {
    pub(super) async fn handle_discover(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<macaca_proto::ServiceCallResult> {
        let typed: ApplicationDiscoverCommand = decode(command.payload)?;
        let registry = self.registry()?;
        let mut guard = registry.write().await;
        let discovered = guard.discover_apps().map_err(service_adapter_error)?;
        tracing::info!(
            trace_id = %typed.trace.trace_id,
            count = discovered.len(),
            "application service discovery completed"
        );
        let views: ApplicationDiscoverResult = discovered
            .iter()
            .map(|app| discovered_view(app, self.domain_pack_catalog.as_ref()))
            .collect();
        Ok(Self::service_result(to_value(views)?, typed.trace))
    }

    pub(super) async fn handle_start(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<macaca_proto::ServiceCallResult> {
        let typed: ApplicationStartCommand = decode(command.payload)?;
        let runtime = self.runtime()?;
        let kernel = self.kernel()?;
        let manifest_path = typed.manifest_path.as_deref().ok_or_else(|| {
            ServiceError::AdapterFailure("application.start requires manifest_path in S7".into())
        })?;
        let path = Path::new(manifest_path);
        // Provider-internal bootstrap — external callers must use Application Service.
        let app_id = runtime
            .bootstrap_manifest_from_path(path, &kernel)
            .await
            .map_err(service_adapter_error)?;
        let app_dir = path.parent().unwrap_or_else(|| Path::new("."));
        let manifest = AppLoader::load_manifest(path).map_err(service_adapter_error)?;
        let view = manifest_view(
            &manifest,
            Some(app_dir),
            true,
            self.domain_pack_catalog.as_ref(),
        );
        tracing::info!(
            service_id = "application",
            command = "start",
            trace_id = %typed.trace.trace_id,
            application_id = %app_id,
            "application service start completed"
        );
        Ok(Self::service_result(to_value(view)?, typed.trace))
    }

    pub(super) async fn handle_status(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<macaca_proto::ServiceCallResult> {
        let typed: ApplicationStatusCommand = decode(command.payload)?;
        let runtime = self.runtime()?;
        let result: ApplicationStatusResult = Self::running_views(
            &runtime,
            self.registry.as_ref(),
            &self.domain_pack_catalog,
        )
        .await?;
        Ok(Self::service_result(to_value(result)?, typed.trace))
    }

    pub(super) async fn handle_metadata_query(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<macaca_proto::ServiceCallResult> {
        let typed: ApplicationMetadataQueryCommand = decode(command.payload)?;
        let registry = self.registry()?;
        let app_id = typed.scope.application_id.ok_or_else(|| {
            ServiceError::AdapterFailure(
                "application.metadata.query requires application_id".into(),
            )
        })?;
        let discovered = {
            let guard = registry.read().await;
            guard.get_app(&app_id).cloned()
        }
        .ok_or_else(|| ServiceError::AdapterFailure(format!("application {app_id} not found")))?;
        let status = running_status_for(self.runtime.as_ref(), &app_id).await;
        let view: ApplicationMetadataResult = app_manifest_to_metadata_view_with_catalog(
            &discovered.manifest,
            Some(&discovered.path),
            status,
            typed.include_abilities,
            typed.include_policy,
            typed.include_overlay,
            typed.include_digest,
            self.domain_pack_catalog.as_ref(),
        );
        tracing::info!(
            trace_id = %typed.trace.trace_id,
            app_id = %app_id,
            ability_count = view.abilities.len(),
            "application service metadata query completed"
        );
        Ok(Self::service_result(to_value(view)?, typed.trace))
    }

    pub(super) async fn handle_heartbeat_agents_query(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<macaca_proto::ServiceCallResult> {
        let typed: ApplicationHeartbeatAgentsQueryCommand = decode(command.payload)?;
        let registry = self.registry()?;
        let app_id = typed.scope.application_id.ok_or_else(|| {
            ServiceError::AdapterFailure(
                "application.heartbeat.agents.query requires application_id".into(),
            )
        })?;
        let discovered = {
            let guard = registry.read().await;
            guard.get_app(&app_id).cloned()
        }
        .ok_or_else(|| ServiceError::AdapterFailure(format!("application {app_id} not found")))?;
        let views: ApplicationHeartbeatAgentsResult =
            app_manifest_to_heartbeat_agent_views(&discovered.manifest);
        tracing::info!(
            trace_id = %typed.trace.trace_id,
            app_id = %app_id,
            declaration_count = views.len(),
            enabled_count = views.iter().filter(|view| view.enabled).count(),
            invalid_count = views.iter().filter(|view| !view.diagnostics.is_empty()).count(),
            "application service heartbeat agent query completed"
        );
        Ok(Self::service_result(to_value(views)?, typed.trace))
    }

    pub(super) async fn handle_snapshot(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<macaca_proto::ServiceCallResult> {
        let typed: ApplicationSnapshotCommand = decode(command.payload)?;
        let discovered = if typed.include_discovered {
            if let Some(registry) = &self.registry {
                Self::discovered_views(registry, &self.domain_pack_catalog).await?
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };
        let running = if typed.include_running {
            if let Some(runtime) = &self.runtime {
                Self::running_views(runtime, self.registry.as_ref(), &self.domain_pack_catalog)
                    .await?
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };
        let sessions = self.sessions.read().await.values().cloned().collect();
        let snapshot = ApplicationServiceSnapshot::healthy(discovered, running, sessions);
        tracing::info!(
            trace_id = %typed.trace.trace_id,
            "application service snapshot emitted"
        );
        Ok(Self::service_result(to_value(snapshot)?, typed.trace))
    }

    pub(super) async fn handle_load(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<macaca_proto::ServiceCallResult> {
        let typed: ApplicationLoadCommand = decode(command.payload)?;
        let unavailable = ApplicationServiceUnavailable::new(
            APPLICATION_LOAD_COMMAND,
            "application load/admission is metadata-only in this S7 provider path",
            Some(&typed.trace),
        );
        Ok(Self::service_result(to_value(unavailable)?, typed.trace))
    }

    pub(super) async fn handle_stop(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<macaca_proto::ServiceCallResult> {
        let typed: ApplicationStopCommand = decode(command.payload)?;
        let runtime = self.runtime()?;
        let kernel = self.kernel()?;
        let app_id = typed.scope.application_id.ok_or_else(|| {
            ServiceError::AdapterFailure("application.stop requires application_id".into())
        })?;
        runtime
            .stop_app(&app_id, &kernel)
            .await
            .map_err(service_adapter_error)?;
        Ok(Self::service_result(
            serde_json::json!({"application_id": app_id, "status": "stopped"}),
            typed.trace,
        ))
    }

    pub(super) async fn handle_remove(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<macaca_proto::ServiceCallResult> {
        let typed: ApplicationRemoveCommand = decode(command.payload)?;
        let runtime = self.runtime()?;
        let app_id = typed.scope.application_id.ok_or_else(|| {
            ServiceError::AdapterFailure("application.remove requires application_id".into())
        })?;
        runtime
            .remove_app(&app_id)
            .await
            .map_err(service_adapter_error)?;
        Ok(Self::service_result(
            serde_json::json!({"application_id": app_id, "status": "removed"}),
            typed.trace,
        ))
    }
}
