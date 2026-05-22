//! SDK Application client facade for Route C S7.
//!
//! The SDK exposes a focused Application Service client for Web, CLI, Gateway,
//! and future shells.  It never constructs `AppRuntime`, `AppRegistry`,
//! `Kernel`, or runtime-host providers; it only translates typed commands into
//! the generic service-client boundary or returns explicit unavailable results.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    ApplicationDiscoverCommand, ApplicationDiscoverResult, ApplicationGenUiSurfaceCommand,
    ApplicationHeartbeatAgentsQueryCommand, ApplicationHeartbeatAgentsResult,
    ApplicationHostDispatchResult, ApplicationHostDispatchServiceCommand,
    ApplicationMetadataQueryCommand, ApplicationMetadataResult, ApplicationRemoveCommand,
    ApplicationServiceSnapshot, ApplicationSessionResult, ApplicationSessionResumeCommand,
    ApplicationSessionStartCommand, ApplicationSessionStopCommand, ApplicationSnapshotCommand,
    ApplicationStartCommand, ApplicationStartResult, ApplicationStatusCommand,
    ApplicationStatusResult, ApplicationStopCommand, MacacaError, MacacaResult, UiIntent,
    APPLICATION_DISCOVER_COMMAND, APPLICATION_GENUI_SURFACE_COMMAND,
    APPLICATION_HEARTBEAT_AGENTS_QUERY_COMMAND, APPLICATION_HOST_DISPATCH_COMMAND,
    APPLICATION_METADATA_QUERY_COMMAND, APPLICATION_REMOVE_COMMAND, APPLICATION_SERVICE_ID,
    APPLICATION_SESSION_RESUME_COMMAND, APPLICATION_SESSION_START_COMMAND,
    APPLICATION_SESSION_STOP_COMMAND, APPLICATION_SNAPSHOT_COMMAND, APPLICATION_START_COMMAND,
    APPLICATION_STATUS_COMMAND, APPLICATION_STOP_COMMAND,
};
use tracing::{info, warn};

use crate::service_client::{ServiceCallCommand, SystemServiceClient};

/// Focused Application client consumed by shells and adapters.
#[async_trait]
pub trait SystemApplicationClient: Send + Sync {
    async fn discover(
        &self,
        command: ApplicationDiscoverCommand,
    ) -> MacacaResult<ApplicationDiscoverResult>;
    async fn start(&self, command: ApplicationStartCommand)
        -> MacacaResult<ApplicationStartResult>;
    async fn stop(&self, command: ApplicationStopCommand) -> MacacaResult<serde_json::Value>;
    async fn remove(&self, command: ApplicationRemoveCommand) -> MacacaResult<serde_json::Value>;
    async fn status(
        &self,
        command: ApplicationStatusCommand,
    ) -> MacacaResult<ApplicationStatusResult>;
    async fn snapshot(
        &self,
        command: ApplicationSnapshotCommand,
    ) -> MacacaResult<ApplicationServiceSnapshot>;
    async fn session_start(
        &self,
        command: ApplicationSessionStartCommand,
    ) -> MacacaResult<ApplicationSessionResult>;
    async fn session_resume(
        &self,
        command: ApplicationSessionResumeCommand,
    ) -> MacacaResult<ApplicationSessionResult>;
    async fn session_stop(
        &self,
        command: ApplicationSessionStopCommand,
    ) -> MacacaResult<ApplicationSessionResult>;
    async fn host_dispatch(
        &self,
        command: ApplicationHostDispatchServiceCommand,
    ) -> MacacaResult<ApplicationHostDispatchResult>;
    /// Query the latest schema-defined GenUI surface for an app/session.
    ///
    /// The SDK returns an optional protocol intent rather than a shell-specific
    /// fallback object so every shell can make the same generic rendering
    /// decision: `Some(intent)` means the Application Service has stored a
    /// schema surface, while `None` means the session simply has no current
    /// surface.  No application names, workflow names, or renderer-specific
    /// assumptions are encoded at this boundary.
    async fn genui_surface(
        &self,
        command: ApplicationGenUiSurfaceCommand,
    ) -> MacacaResult<Option<UiIntent>>;
    async fn metadata(
        &self,
        command: ApplicationMetadataQueryCommand,
    ) -> MacacaResult<ApplicationMetadataResult>;
    /// Query sanitized manifest-declared heartbeat agents.
    ///
    /// This keeps Web and CLI on the Application Service facade when they need
    /// declaration views for Heartbeat Operations. Shells receive only bounded
    /// metadata and diagnostics; raw manifests and HEARTBEAT.md content remain
    /// behind the Application Service provider.
    async fn heartbeat_agents(
        &self,
        command: ApplicationHeartbeatAgentsQueryCommand,
    ) -> MacacaResult<ApplicationHeartbeatAgentsResult>;
}

/// Null-object Application client used when the service is not installed.
#[derive(Debug, Clone, Default)]
pub struct UnavailableSystemApplicationClient;

#[async_trait]
impl SystemApplicationClient for UnavailableSystemApplicationClient {
    async fn discover(
        &self,
        command: ApplicationDiscoverCommand,
    ) -> MacacaResult<ApplicationDiscoverResult> {
        info!(trace_id = %command.trace.trace_id, "sdk application client returning empty discovery");
        Ok(Vec::new())
    }

    async fn start(
        &self,
        command: ApplicationStartCommand,
    ) -> MacacaResult<ApplicationStartResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk application client unavailable for start");
        Err(MacacaError::Config(
            "Application service is unavailable".into(),
        ))
    }

    async fn stop(&self, command: ApplicationStopCommand) -> MacacaResult<serde_json::Value> {
        warn!(trace_id = %command.trace.trace_id, "sdk application client unavailable for stop");
        Err(MacacaError::Config(
            "Application service is unavailable".into(),
        ))
    }

    async fn remove(&self, command: ApplicationRemoveCommand) -> MacacaResult<serde_json::Value> {
        warn!(trace_id = %command.trace.trace_id, "sdk application client unavailable for remove");
        Err(MacacaError::Config(
            "Application service is unavailable".into(),
        ))
    }

    async fn status(
        &self,
        command: ApplicationStatusCommand,
    ) -> MacacaResult<ApplicationStatusResult> {
        info!(trace_id = %command.trace.trace_id, "sdk application client returning empty status");
        Ok(Vec::new())
    }

    async fn snapshot(
        &self,
        command: ApplicationSnapshotCommand,
    ) -> MacacaResult<ApplicationServiceSnapshot> {
        info!(trace_id = %command.trace.trace_id, "sdk application client returning unavailable snapshot");
        Ok(ApplicationServiceSnapshot::unavailable(
            "runtime-backed Application service is not installed",
        ))
    }

    async fn session_start(
        &self,
        command: ApplicationSessionStartCommand,
    ) -> MacacaResult<ApplicationSessionResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk application client unavailable for session start");
        Err(MacacaError::Config(
            "Application service is unavailable".into(),
        ))
    }

    async fn session_resume(
        &self,
        command: ApplicationSessionResumeCommand,
    ) -> MacacaResult<ApplicationSessionResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk application client unavailable for session resume");
        Err(MacacaError::Config(
            "Application service is unavailable".into(),
        ))
    }

    async fn session_stop(
        &self,
        command: ApplicationSessionStopCommand,
    ) -> MacacaResult<ApplicationSessionResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk application client unavailable for session stop");
        Err(MacacaError::Config(
            "Application service is unavailable".into(),
        ))
    }

    async fn host_dispatch(
        &self,
        command: ApplicationHostDispatchServiceCommand,
    ) -> MacacaResult<ApplicationHostDispatchResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk application client unavailable for host dispatch");
        Err(MacacaError::Config(
            "Application service is unavailable".into(),
        ))
    }

    async fn genui_surface(
        &self,
        command: ApplicationGenUiSurfaceCommand,
    ) -> MacacaResult<Option<UiIntent>> {
        info!(
            trace_id = %command.trace.trace_id,
            "sdk application client returning empty GenUI surface because Application service is unavailable"
        );
        Ok(None)
    }

    async fn metadata(
        &self,
        command: ApplicationMetadataQueryCommand,
    ) -> MacacaResult<ApplicationMetadataResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk application client unavailable for metadata query");
        Err(MacacaError::Config(
            "Application service is unavailable".into(),
        ))
    }

    async fn heartbeat_agents(
        &self,
        command: ApplicationHeartbeatAgentsQueryCommand,
    ) -> MacacaResult<ApplicationHeartbeatAgentsResult> {
        info!(
            trace_id = %command.trace.trace_id,
            "sdk application client returning empty heartbeat declaration view"
        );
        Ok(Vec::new())
    }
}

/// Runtime-backed Application client implemented over generic service calls.
#[derive(Clone)]
pub struct ServiceBackedApplicationClient {
    service: Arc<dyn SystemServiceClient>,
}

impl ServiceBackedApplicationClient {
    /// Create a service-backed client from an existing generic service client.
    pub fn new(service: Arc<dyn SystemServiceClient>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl SystemApplicationClient for ServiceBackedApplicationClient {
    async fn discover(
        &self,
        command: ApplicationDiscoverCommand,
    ) -> MacacaResult<ApplicationDiscoverResult> {
        call(
            &self.service,
            APPLICATION_DISCOVER_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn start(
        &self,
        command: ApplicationStartCommand,
    ) -> MacacaResult<ApplicationStartResult> {
        call(
            &self.service,
            APPLICATION_START_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn stop(&self, command: ApplicationStopCommand) -> MacacaResult<serde_json::Value> {
        call(
            &self.service,
            APPLICATION_STOP_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn remove(&self, command: ApplicationRemoveCommand) -> MacacaResult<serde_json::Value> {
        call(
            &self.service,
            APPLICATION_REMOVE_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn status(
        &self,
        command: ApplicationStatusCommand,
    ) -> MacacaResult<ApplicationStatusResult> {
        call(
            &self.service,
            APPLICATION_STATUS_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn snapshot(
        &self,
        command: ApplicationSnapshotCommand,
    ) -> MacacaResult<ApplicationServiceSnapshot> {
        call(
            &self.service,
            APPLICATION_SNAPSHOT_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn session_start(
        &self,
        command: ApplicationSessionStartCommand,
    ) -> MacacaResult<ApplicationSessionResult> {
        call(
            &self.service,
            APPLICATION_SESSION_START_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn session_resume(
        &self,
        command: ApplicationSessionResumeCommand,
    ) -> MacacaResult<ApplicationSessionResult> {
        call(
            &self.service,
            APPLICATION_SESSION_RESUME_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn session_stop(
        &self,
        command: ApplicationSessionStopCommand,
    ) -> MacacaResult<ApplicationSessionResult> {
        call(
            &self.service,
            APPLICATION_SESSION_STOP_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn host_dispatch(
        &self,
        command: ApplicationHostDispatchServiceCommand,
    ) -> MacacaResult<ApplicationHostDispatchResult> {
        call(
            &self.service,
            APPLICATION_HOST_DISPATCH_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn genui_surface(
        &self,
        command: ApplicationGenUiSurfaceCommand,
    ) -> MacacaResult<Option<UiIntent>> {
        call(
            &self.service,
            APPLICATION_GENUI_SURFACE_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn metadata(
        &self,
        command: ApplicationMetadataQueryCommand,
    ) -> MacacaResult<ApplicationMetadataResult> {
        call(
            &self.service,
            APPLICATION_METADATA_QUERY_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn heartbeat_agents(
        &self,
        command: ApplicationHeartbeatAgentsQueryCommand,
    ) -> MacacaResult<ApplicationHeartbeatAgentsResult> {
        call(
            &self.service,
            APPLICATION_HEARTBEAT_AGENTS_QUERY_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }
}

async fn call<T, R>(
    service: &Arc<dyn SystemServiceClient>,
    command_name: &str,
    trace: macaca_proto::TraceContext,
    payload: T,
) -> MacacaResult<R>
where
    T: serde::Serialize,
    R: serde::de::DeserializeOwned,
{
    let service_command = ServiceCallCommand::new(
        APPLICATION_SERVICE_ID,
        command_name,
        serde_json::to_value(payload)?,
    )?
    .with_trace(trace.clone());
    info!(
        service_id = APPLICATION_SERVICE_ID,
        command = command_name,
        trace_id = %trace.trace_id,
        "sdk application client dispatching service command"
    );
    let result = service.call_service(&service_command).await?;
    serde_json::from_value(result.output).map_err(|error| {
        MacacaError::Config(format!("invalid Application service result: {error}"))
    })
}
