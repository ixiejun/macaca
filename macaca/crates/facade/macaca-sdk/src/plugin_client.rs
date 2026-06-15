//! SDK Plugin Control client for the Plugin Fabric.
//!
//! This client is the shell-facing Facade for plugin management.  Web, CLI, and
//! future gateways call this trait instead of reading plugin repositories,
//! loading manifests, or invoking runtime-host internals.  The service-backed
//! implementation uses generic protocol service calls, so the actual Plugin
//! Control Service can be local, remote, or replaced by another host strategy.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    MacacaError, MacacaResult, PluginControlRecord, PluginDiagnostics, PluginHealthSnapshot,
    PluginInstallRequest, PluginInstallResult, PluginListCommand, PluginTargetCommand,
    PLUGIN_CONTROL_DIAGNOSTICS_COMMAND, PLUGIN_CONTROL_DISABLE_COMMAND,
    PLUGIN_CONTROL_ENABLE_COMMAND, PLUGIN_CONTROL_HEALTH_COMMAND, PLUGIN_CONTROL_INSPECT_COMMAND,
    PLUGIN_CONTROL_INSTALL_COMMAND, PLUGIN_CONTROL_LIST_COMMAND, PLUGIN_CONTROL_SERVICE_ID,
    PLUGIN_CONTROL_START_COMMAND, PLUGIN_CONTROL_STOP_COMMAND, PLUGIN_CONTROL_UNINSTALL_COMMAND,
};
use tracing::{info, warn};

use crate::service_client::{ServiceCallCommand, SystemServiceClient};

/// Focused Plugin Control client consumed by shells and SDK users.
#[async_trait]
pub trait SystemPluginControlClient: Send + Sync {
    async fn list(&self, command: PluginListCommand) -> MacacaResult<Vec<PluginControlRecord>>;
    async fn inspect(
        &self,
        command: PluginTargetCommand,
    ) -> MacacaResult<Option<PluginControlRecord>>;
    async fn install(&self, command: PluginInstallRequest) -> MacacaResult<PluginInstallResult>;
    async fn enable(&self, command: PluginTargetCommand) -> MacacaResult<PluginControlRecord>;
    async fn disable(&self, command: PluginTargetCommand) -> MacacaResult<PluginControlRecord>;
    async fn start(&self, command: PluginTargetCommand) -> MacacaResult<PluginControlRecord>;
    async fn stop(&self, command: PluginTargetCommand) -> MacacaResult<PluginControlRecord>;
    async fn uninstall(&self, command: PluginTargetCommand) -> MacacaResult<()>;
    async fn health(&self, command: PluginTargetCommand) -> MacacaResult<PluginHealthSnapshot>;
    async fn diagnostics(&self, command: PluginTargetCommand) -> MacacaResult<PluginDiagnostics>;
}

/// Null-object client used when Plugin Control Service is not configured.
#[derive(Debug, Clone, Default)]
pub struct UnavailableSystemPluginControlClient;

#[async_trait]
impl SystemPluginControlClient for UnavailableSystemPluginControlClient {
    async fn list(&self, command: PluginListCommand) -> MacacaResult<Vec<PluginControlRecord>> {
        info!(
            trace_id = %command.trace.trace_id,
            "sdk plugin control client returning empty unavailable list"
        );
        Ok(Vec::new())
    }

    async fn inspect(
        &self,
        command: PluginTargetCommand,
    ) -> MacacaResult<Option<PluginControlRecord>> {
        info!(
            trace_id = %command.trace.trace_id,
            plugin_id = %command.plugin_id,
            "sdk plugin control client unavailable for inspect"
        );
        Ok(None)
    }

    async fn install(&self, command: PluginInstallRequest) -> MacacaResult<PluginInstallResult> {
        warn!(
            trace_id = %command.trace.trace_id,
            "sdk plugin control client unavailable for install"
        );
        Err(MacacaError::Config(
            "Plugin Control service is unavailable".into(),
        ))
    }

    async fn enable(&self, command: PluginTargetCommand) -> MacacaResult<PluginControlRecord> {
        unavailable_state("enable", command)
    }

    async fn disable(&self, command: PluginTargetCommand) -> MacacaResult<PluginControlRecord> {
        unavailable_state("disable", command)
    }

    async fn start(&self, command: PluginTargetCommand) -> MacacaResult<PluginControlRecord> {
        unavailable_state("start", command)
    }

    async fn stop(&self, command: PluginTargetCommand) -> MacacaResult<PluginControlRecord> {
        unavailable_state("stop", command)
    }

    async fn uninstall(&self, command: PluginTargetCommand) -> MacacaResult<()> {
        warn!(
            trace_id = %command.trace.trace_id,
            plugin_id = %command.plugin_id,
            "sdk plugin control client unavailable for uninstall"
        );
        Err(MacacaError::Config(
            "Plugin Control service is unavailable".into(),
        ))
    }

    async fn health(&self, command: PluginTargetCommand) -> MacacaResult<PluginHealthSnapshot> {
        warn!(
            trace_id = %command.trace.trace_id,
            plugin_id = %command.plugin_id,
            "sdk plugin control client unavailable for health"
        );
        Err(MacacaError::Config(
            "Plugin Control service is unavailable".into(),
        ))
    }

    async fn diagnostics(&self, command: PluginTargetCommand) -> MacacaResult<PluginDiagnostics> {
        warn!(
            trace_id = %command.trace.trace_id,
            plugin_id = %command.plugin_id,
            "sdk plugin control client unavailable for diagnostics"
        );
        Err(MacacaError::Config(
            "Plugin Control service is unavailable".into(),
        ))
    }
}

/// Runtime-backed client implemented over generic service dispatch.
#[derive(Clone)]
pub struct ServiceBackedPluginControlClient {
    service: Arc<dyn SystemServiceClient>,
}

impl ServiceBackedPluginControlClient {
    /// Create a service-backed Plugin Control client.
    pub fn new(service: Arc<dyn SystemServiceClient>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl SystemPluginControlClient for ServiceBackedPluginControlClient {
    async fn list(&self, command: PluginListCommand) -> MacacaResult<Vec<PluginControlRecord>> {
        call(&*self.service, PLUGIN_CONTROL_LIST_COMMAND, command).await
    }

    async fn inspect(
        &self,
        command: PluginTargetCommand,
    ) -> MacacaResult<Option<PluginControlRecord>> {
        call(&*self.service, PLUGIN_CONTROL_INSPECT_COMMAND, command).await
    }

    async fn install(&self, command: PluginInstallRequest) -> MacacaResult<PluginInstallResult> {
        call(&*self.service, PLUGIN_CONTROL_INSTALL_COMMAND, command).await
    }

    async fn enable(&self, command: PluginTargetCommand) -> MacacaResult<PluginControlRecord> {
        call(&*self.service, PLUGIN_CONTROL_ENABLE_COMMAND, command).await
    }

    async fn disable(&self, command: PluginTargetCommand) -> MacacaResult<PluginControlRecord> {
        call(&*self.service, PLUGIN_CONTROL_DISABLE_COMMAND, command).await
    }

    async fn start(&self, command: PluginTargetCommand) -> MacacaResult<PluginControlRecord> {
        call(&*self.service, PLUGIN_CONTROL_START_COMMAND, command).await
    }

    async fn stop(&self, command: PluginTargetCommand) -> MacacaResult<PluginControlRecord> {
        call(&*self.service, PLUGIN_CONTROL_STOP_COMMAND, command).await
    }

    async fn uninstall(&self, command: PluginTargetCommand) -> MacacaResult<()> {
        let _: serde_json::Value =
            call(&*self.service, PLUGIN_CONTROL_UNINSTALL_COMMAND, command).await?;
        Ok(())
    }

    async fn health(&self, command: PluginTargetCommand) -> MacacaResult<PluginHealthSnapshot> {
        call(&*self.service, PLUGIN_CONTROL_HEALTH_COMMAND, command).await
    }

    async fn diagnostics(&self, command: PluginTargetCommand) -> MacacaResult<PluginDiagnostics> {
        call(&*self.service, PLUGIN_CONTROL_DIAGNOSTICS_COMMAND, command).await
    }
}

fn unavailable_state<T>(operation: &str, command: PluginTargetCommand) -> MacacaResult<T> {
    warn!(
        trace_id = %command.trace.trace_id,
        plugin_id = %command.plugin_id,
        operation,
        "sdk plugin control client unavailable for state-changing operation"
    );
    Err(MacacaError::Config(
        "Plugin Control service is unavailable".into(),
    ))
}

async fn call<T, R>(
    service: &dyn SystemServiceClient,
    command_name: &str,
    command: T,
) -> MacacaResult<R>
where
    T: serde::Serialize,
    R: serde::de::DeserializeOwned,
{
    let result = service
        .call_service(&ServiceCallCommand::new(
            PLUGIN_CONTROL_SERVICE_ID,
            command_name,
            serde_json::to_value(command)?,
        )?)
        .await?;
    serde_json::from_value(result.output).map_err(MacacaError::from)
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use macaca_proto::{
        DeveloperId, MacacaResult, PluginId, PluginInstallSourceKind, PluginPackageLocation,
        PluginRuntimeDeclaration, PluginRuntimeKind, PluginVersion, TraceContext,
    };

    use super::*;
    use crate::service_client::{
        ServiceCallResult, ServiceInspectionCommand, ServiceInspectionResult,
    };

    struct EchoPluginServiceClient;

    #[async_trait]
    impl SystemServiceClient for EchoPluginServiceClient {
        async fn inspect_services(
            &self,
            command: &ServiceInspectionCommand,
        ) -> MacacaResult<ServiceInspectionResult> {
            Ok(ServiceInspectionResult {
                scope: command.scope.clone(),
                services: vec![PLUGIN_CONTROL_SERVICE_ID.into()],
            })
        }

        async fn call_service(
            &self,
            command: &ServiceCallCommand,
        ) -> MacacaResult<ServiceCallResult> {
            assert_eq!(command.service_id, PLUGIN_CONTROL_SERVICE_ID);
            let output = match command.command_name.as_str() {
                PLUGIN_CONTROL_LIST_COMMAND => serde_json::json!([]),
                PLUGIN_CONTROL_INSTALL_COMMAND => {
                    let request: PluginInstallRequest =
                        serde_json::from_value(command.payload.clone()).unwrap();
                    serde_json::json!({
                        "record": {
                            "plugin_id": request.manifest.as_ref().unwrap().plugin_id,
                            "version": request.manifest.as_ref().unwrap().version,
                            "repository_id": "default",
                            "source_kind": request.location.source_kind,
                            "activation_state": "disabled",
                            "lifecycle_state": "installed",
                            "health": "Unknown",
                            "enabled": false,
                            "installed_at": chrono::Utc::now(),
                            "updated_at": chrono::Utc::now(),
                            "manifest": request.manifest.unwrap(),
                            "config_requirements": [],
                            "secret_requirements": [],
                            "metadata": {}
                        },
                        "events": []
                    })
                }
                other => panic!("unexpected command {other}"),
            };
            Ok(ServiceCallResult {
                service_id: command.service_id.clone(),
                output,
            })
        }
    }

    fn install_request() -> PluginInstallRequest {
        PluginInstallRequest {
            location: PluginPackageLocation::new(PluginInstallSourceKind::UserLocal, "/tmp/plugin"),
            manifest: Some(macaca_proto::PluginManifest::new(
                PluginId::new("plugin.sdk.fixture"),
                PluginVersion::new("1.0.0"),
                DeveloperId::new("developer.fixture"),
                PluginRuntimeDeclaration::new(PluginRuntimeKind::DescriptorOnly, "1"),
            )),
            enable_after_install: false,
            trace: TraceContext::new("trace-sdk-plugin"),
            metadata: Default::default(),
        }
    }

    #[tokio::test]
    async fn service_backed_client_dispatches_plugin_control_commands() {
        let client = ServiceBackedPluginControlClient::new(Arc::new(EchoPluginServiceClient));
        let listed = client
            .list(PluginListCommand {
                repository_id: None,
                include_disabled: true,
                trace: TraceContext::new("trace-list"),
            })
            .await
            .unwrap();
        assert!(listed.is_empty());

        let installed = client.install(install_request()).await.unwrap();
        assert_eq!(installed.record.plugin_id.as_str(), "plugin.sdk.fixture");
    }

    #[tokio::test]
    async fn unavailable_client_is_safe_for_read_only_list() {
        let client = UnavailableSystemPluginControlClient;
        let listed = client
            .list(PluginListCommand {
                repository_id: None,
                include_disabled: true,
                trace: TraceContext::new("trace-list"),
            })
            .await
            .unwrap();
        assert!(listed.is_empty());
    }
}
