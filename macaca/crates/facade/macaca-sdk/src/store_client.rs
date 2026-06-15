//! SDK Store Service client.
//!
//! The client is a shell-facing Facade.  Web, CLI, and future Gateway adapters
//! use this trait instead of constructing Store providers or reading package
//! internals.  The service-backed implementation delegates to the generic
//! `SystemServiceClient`, while the unavailable implementation preserves safe
//! Null Object behavior when Store Service is not installed.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    MacacaError, MacacaResult, StoreInstallResult, StorePackageInspectCommand,
    StorePackageInspectResult, StorePackageInstallCommand, StorePackageResolveCommand,
    StorePackageStatusCommand, StorePackageStatusResult, StorePackageView, StoreServiceSnapshot,
    StoreSnapshotCommand, STORE_PACKAGE_INSPECT_COMMAND, STORE_PACKAGE_INSTALL_COMMAND,
    STORE_PACKAGE_RESOLVE_COMMAND, STORE_PACKAGE_STATUS_COMMAND, STORE_SERVICE_ID,
    STORE_SNAPSHOT_COMMAND,
};
use tracing::{info, warn};

use crate::service_client::{ServiceCallCommand, SystemServiceClient};

/// Focused Store client consumed by presentation shells and SDK users.
#[async_trait]
pub trait SystemStoreClient: Send + Sync {
    async fn inspect(
        &self,
        command: StorePackageInspectCommand,
    ) -> MacacaResult<StorePackageInspectResult>;
    async fn resolve(&self, command: StorePackageResolveCommand) -> MacacaResult<StorePackageView>;
    async fn install(
        &self,
        command: StorePackageInstallCommand,
    ) -> MacacaResult<StoreInstallResult>;
    async fn status(
        &self,
        command: StorePackageStatusCommand,
    ) -> MacacaResult<StorePackageStatusResult>;
    async fn snapshot(&self, command: StoreSnapshotCommand) -> MacacaResult<StoreServiceSnapshot>;
}

/// Null-object Store client used when Store Service is not configured.
#[derive(Debug, Clone, Default)]
pub struct UnavailableSystemStoreClient;

#[async_trait]
impl SystemStoreClient for UnavailableSystemStoreClient {
    async fn inspect(
        &self,
        command: StorePackageInspectCommand,
    ) -> MacacaResult<StorePackageInspectResult> {
        info!(
            trace_id = %command.trace.trace_id,
            package = %command.scope.label(),
            "sdk store client returning empty unavailable inspection"
        );
        Ok(Vec::new())
    }

    async fn resolve(&self, command: StorePackageResolveCommand) -> MacacaResult<StorePackageView> {
        warn!(
            trace_id = %command.trace.trace_id,
            package = %command.scope.label(),
            "sdk store client unavailable for resolve"
        );
        Ok(StorePackageView::unavailable(
            &command.scope,
            "Store service is unavailable",
        ))
    }

    async fn install(
        &self,
        command: StorePackageInstallCommand,
    ) -> MacacaResult<StoreInstallResult> {
        warn!(
            trace_id = %command.trace.trace_id,
            package = %command.scope.label(),
            "sdk store client unavailable for install"
        );
        Err(MacacaError::Config("Store service is unavailable".into()))
    }

    async fn status(
        &self,
        command: StorePackageStatusCommand,
    ) -> MacacaResult<StorePackageStatusResult> {
        info!(
            trace_id = %command.trace.trace_id,
            package = %command.scope.label(),
            "sdk store client returning empty unavailable status"
        );
        Ok(Vec::new())
    }

    async fn snapshot(&self, command: StoreSnapshotCommand) -> MacacaResult<StoreServiceSnapshot> {
        Ok(StoreServiceSnapshot::unavailable(format!(
            "Store service is unavailable for trace {}",
            command.trace.trace_id
        )))
    }
}

/// Runtime-backed Store client implemented over generic service calls.
#[derive(Clone)]
pub struct ServiceBackedStoreClient {
    service: Arc<dyn SystemServiceClient>,
}

impl ServiceBackedStoreClient {
    /// Create a service-backed Store client from a generic service client.
    pub fn new(service: Arc<dyn SystemServiceClient>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl SystemStoreClient for ServiceBackedStoreClient {
    async fn inspect(
        &self,
        command: StorePackageInspectCommand,
    ) -> MacacaResult<StorePackageInspectResult> {
        call(
            &self.service,
            STORE_PACKAGE_INSPECT_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn resolve(&self, command: StorePackageResolveCommand) -> MacacaResult<StorePackageView> {
        call(
            &self.service,
            STORE_PACKAGE_RESOLVE_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn install(
        &self,
        command: StorePackageInstallCommand,
    ) -> MacacaResult<StoreInstallResult> {
        call(
            &self.service,
            STORE_PACKAGE_INSTALL_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn status(
        &self,
        command: StorePackageStatusCommand,
    ) -> MacacaResult<StorePackageStatusResult> {
        call(
            &self.service,
            STORE_PACKAGE_STATUS_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn snapshot(&self, command: StoreSnapshotCommand) -> MacacaResult<StoreServiceSnapshot> {
        call(
            &self.service,
            STORE_SNAPSHOT_COMMAND,
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
    let command = ServiceCallCommand::new(
        STORE_SERVICE_ID,
        command_name,
        serde_json::to_value(payload).map_err(|error| MacacaError::Config(error.to_string()))?,
    )?
    .with_trace(trace);
    let result = service.call_service(&command).await?;
    serde_json::from_value(result.output).map_err(|error| MacacaError::Config(error.to_string()))
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use macaca_proto::{CommerceMetadata, DeveloperId, PackageId, StorePackageScope, TraceContext};

    use super::*;
    use crate::service_client::{
        ServiceCallResult, ServiceInspectionCommand, ServiceInspectionResult,
    };

    struct EchoStoreServiceClient;

    #[async_trait]
    impl SystemServiceClient for EchoStoreServiceClient {
        async fn inspect_services(
            &self,
            command: &ServiceInspectionCommand,
        ) -> MacacaResult<ServiceInspectionResult> {
            Ok(ServiceInspectionResult {
                scope: command.scope.clone(),
                services: vec![STORE_SERVICE_ID.into()],
            })
        }

        async fn call_service(
            &self,
            command: &ServiceCallCommand,
        ) -> MacacaResult<ServiceCallResult> {
            assert_eq!(command.service_id, STORE_SERVICE_ID);
            assert_eq!(command.command_name, STORE_PACKAGE_INSTALL_COMMAND);
            let typed: StorePackageInstallCommand =
                serde_json::from_value(command.payload.clone()).unwrap();
            Ok(ServiceCallResult {
                service_id: command.service_id.clone(),
                output: serde_json::to_value(StoreInstallResult {
                    package: StorePackageView {
                        package_id: typed.scope.package_id.clone(),
                        developer_id: typed.scope.developer_id.clone(),
                        package_ref: typed.scope.package_ref.clone(),
                        version: Some("1.0.0".into()),
                        runtime_kind: typed.runtime_kind.clone(),
                        commerce: typed.commerce.clone(),
                        install_status: "installed".into(),
                        source_status: "metadata_resolved".into(),
                        diagnostics: Vec::new(),
                        metadata: Default::default(),
                    },
                    allowed: true,
                    status: "installed".into(),
                    reason: "test install".into(),
                    trace: typed.trace,
                })
                .unwrap(),
            })
        }
    }

    #[tokio::test]
    async fn service_backed_store_client_dispatches_install_command() {
        let client = ServiceBackedStoreClient::new(Arc::new(EchoStoreServiceClient));
        let result = client
            .install(StorePackageInstallCommand {
                trace: TraceContext::new("trace-sdk-store-install"),
                scope: StorePackageScope {
                    package_id: Some(PackageId::new("pkg.sdk")),
                    developer_id: Some(DeveloperId::new("developer.sdk")),
                    package_ref: None,
                    source_id: None,
                },
                commerce: CommerceMetadata::default(),
                runtime_kind: None,
            })
            .await
            .unwrap();

        assert!(result.allowed);
        assert_eq!(result.package.package_id.unwrap().as_str(), "pkg.sdk");
    }
}
