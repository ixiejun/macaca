//! SDK Entitlement Service client for Route C S9.
//!
//! The client keeps entitlement query, authorization, audit, and metering calls
//! behind `SystemFacade`.  SDK users do not construct runtime-host facades,
//! repositories, app guards, or skill loaders directly.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    EntitlementAuditPage, EntitlementAuditQueryCommand, EntitlementAuthorizeCallCommand,
    EntitlementAuthorizeInstallCommand, EntitlementAuthorizeResult,
    EntitlementAuthorizeStartCommand, EntitlementMeteringRecordCommand, EntitlementQueryCommand,
    EntitlementQueryResult, EntitlementRecord, EntitlementRevokeCommand,
    EntitlementServiceSnapshot, EntitlementSnapshotCommand, EntitlementUpsertCommand, MacacaError,
    MacacaResult, ENTITLEMENT_AUDIT_QUERY_COMMAND, ENTITLEMENT_AUTHORIZE_CALL_COMMAND,
    ENTITLEMENT_AUTHORIZE_INSTALL_COMMAND, ENTITLEMENT_AUTHORIZE_START_COMMAND,
    ENTITLEMENT_METERING_RECORD_COMMAND, ENTITLEMENT_QUERY_COMMAND, ENTITLEMENT_REVOKE_COMMAND,
    ENTITLEMENT_SERVICE_ID, ENTITLEMENT_SNAPSHOT_COMMAND, ENTITLEMENT_UPSERT_COMMAND,
};
use tracing::{info, warn};

use crate::service_client::{ServiceCallCommand, SystemServiceClient};

/// Focused Entitlement client consumed by Store, Web, CLI, app, and skill adapters.
#[async_trait]
pub trait SystemEntitlementClient: Send + Sync {
    async fn query(&self, command: EntitlementQueryCommand)
        -> MacacaResult<EntitlementQueryResult>;
    async fn upsert(&self, command: EntitlementUpsertCommand) -> MacacaResult<EntitlementRecord>;
    async fn revoke(&self, command: EntitlementRevokeCommand) -> MacacaResult<EntitlementRecord>;
    async fn authorize_install(
        &self,
        command: EntitlementAuthorizeInstallCommand,
    ) -> MacacaResult<EntitlementAuthorizeResult>;
    async fn authorize_start(
        &self,
        command: EntitlementAuthorizeStartCommand,
    ) -> MacacaResult<EntitlementAuthorizeResult>;
    async fn authorize_call(
        &self,
        command: EntitlementAuthorizeCallCommand,
    ) -> MacacaResult<EntitlementAuthorizeResult>;
    async fn audit(
        &self,
        command: EntitlementAuditQueryCommand,
    ) -> MacacaResult<EntitlementAuditPage>;
    async fn record_metering(
        &self,
        command: EntitlementMeteringRecordCommand,
    ) -> MacacaResult<macaca_proto::MeteringEvent>;
    async fn snapshot(
        &self,
        command: EntitlementSnapshotCommand,
    ) -> MacacaResult<EntitlementServiceSnapshot>;
}

/// Null-object Entitlement client used when the service is not installed.
#[derive(Debug, Clone, Default)]
pub struct UnavailableSystemEntitlementClient;

#[async_trait]
impl SystemEntitlementClient for UnavailableSystemEntitlementClient {
    async fn query(
        &self,
        command: EntitlementQueryCommand,
    ) -> MacacaResult<EntitlementQueryResult> {
        info!(trace_id = %command.trace.trace_id, "sdk entitlement client returning empty query");
        Ok(None)
    }

    async fn upsert(&self, _command: EntitlementUpsertCommand) -> MacacaResult<EntitlementRecord> {
        Err(MacacaError::Config(
            "Entitlement service is unavailable".into(),
        ))
    }

    async fn revoke(&self, _command: EntitlementRevokeCommand) -> MacacaResult<EntitlementRecord> {
        Err(MacacaError::Config(
            "Entitlement service is unavailable".into(),
        ))
    }

    async fn authorize_install(
        &self,
        command: EntitlementAuthorizeInstallCommand,
    ) -> MacacaResult<EntitlementAuthorizeResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk entitlement client unavailable for install authorization");
        Err(MacacaError::Config(
            "Entitlement service is unavailable".into(),
        ))
    }

    async fn authorize_start(
        &self,
        command: EntitlementAuthorizeStartCommand,
    ) -> MacacaResult<EntitlementAuthorizeResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk entitlement client unavailable for start authorization");
        Err(MacacaError::Config(
            "Entitlement service is unavailable".into(),
        ))
    }

    async fn authorize_call(
        &self,
        command: EntitlementAuthorizeCallCommand,
    ) -> MacacaResult<EntitlementAuthorizeResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk entitlement client unavailable for call authorization");
        Err(MacacaError::Config(
            "Entitlement service is unavailable".into(),
        ))
    }

    async fn audit(
        &self,
        command: EntitlementAuditQueryCommand,
    ) -> MacacaResult<EntitlementAuditPage> {
        Ok(EntitlementAuditPage {
            package_id: command.scope.package_id,
            decisions: Vec::new(),
            next_offset: None,
        })
    }

    async fn record_metering(
        &self,
        _command: EntitlementMeteringRecordCommand,
    ) -> MacacaResult<macaca_proto::MeteringEvent> {
        Err(MacacaError::Config(
            "Entitlement service is unavailable".into(),
        ))
    }

    async fn snapshot(
        &self,
        _command: EntitlementSnapshotCommand,
    ) -> MacacaResult<EntitlementServiceSnapshot> {
        Ok(EntitlementServiceSnapshot::unavailable(
            "Entitlement service is unavailable",
        ))
    }
}

/// Runtime-backed Entitlement client implemented over generic service calls.
#[derive(Clone)]
pub struct ServiceBackedEntitlementClient {
    service: Arc<dyn SystemServiceClient>,
}

impl ServiceBackedEntitlementClient {
    /// Create a service-backed client from a generic service dispatcher.
    pub fn new(service: Arc<dyn SystemServiceClient>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl SystemEntitlementClient for ServiceBackedEntitlementClient {
    async fn query(
        &self,
        command: EntitlementQueryCommand,
    ) -> MacacaResult<EntitlementQueryResult> {
        call(
            &self.service,
            ENTITLEMENT_QUERY_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn upsert(&self, command: EntitlementUpsertCommand) -> MacacaResult<EntitlementRecord> {
        call(
            &self.service,
            ENTITLEMENT_UPSERT_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn revoke(&self, command: EntitlementRevokeCommand) -> MacacaResult<EntitlementRecord> {
        call(
            &self.service,
            ENTITLEMENT_REVOKE_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn authorize_install(
        &self,
        command: EntitlementAuthorizeInstallCommand,
    ) -> MacacaResult<EntitlementAuthorizeResult> {
        call(
            &self.service,
            ENTITLEMENT_AUTHORIZE_INSTALL_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn authorize_start(
        &self,
        command: EntitlementAuthorizeStartCommand,
    ) -> MacacaResult<EntitlementAuthorizeResult> {
        call(
            &self.service,
            ENTITLEMENT_AUTHORIZE_START_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn authorize_call(
        &self,
        command: EntitlementAuthorizeCallCommand,
    ) -> MacacaResult<EntitlementAuthorizeResult> {
        call(
            &self.service,
            ENTITLEMENT_AUTHORIZE_CALL_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn audit(
        &self,
        command: EntitlementAuditQueryCommand,
    ) -> MacacaResult<EntitlementAuditPage> {
        call(
            &self.service,
            ENTITLEMENT_AUDIT_QUERY_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn record_metering(
        &self,
        command: EntitlementMeteringRecordCommand,
    ) -> MacacaResult<macaca_proto::MeteringEvent> {
        call(
            &self.service,
            ENTITLEMENT_METERING_RECORD_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn snapshot(
        &self,
        command: EntitlementSnapshotCommand,
    ) -> MacacaResult<EntitlementServiceSnapshot> {
        call(
            &self.service,
            ENTITLEMENT_SNAPSHOT_COMMAND,
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
        ENTITLEMENT_SERVICE_ID,
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
    use chrono::Utc;
    use macaca_proto::{
        DeveloperId, EntitlementId, EntitlementRecord, EntitlementServiceScope, EntitlementState,
        PackageId, TraceContext,
    };

    use super::*;
    use crate::service_client::{
        ServiceCallResult, ServiceInspectionCommand, ServiceInspectionResult,
    };

    struct EchoEntitlementServiceClient;

    #[async_trait]
    impl SystemServiceClient for EchoEntitlementServiceClient {
        async fn inspect_services(
            &self,
            command: &ServiceInspectionCommand,
        ) -> MacacaResult<ServiceInspectionResult> {
            Ok(ServiceInspectionResult {
                scope: command.scope.clone(),
                services: vec![ENTITLEMENT_SERVICE_ID.into()],
            })
        }

        async fn call_service(
            &self,
            command: &ServiceCallCommand,
        ) -> MacacaResult<ServiceCallResult> {
            assert_eq!(command.service_id, ENTITLEMENT_SERVICE_ID);
            assert_eq!(command.command_name, ENTITLEMENT_REVOKE_COMMAND);
            let typed: EntitlementRevokeCommand =
                serde_json::from_value(command.payload.clone()).unwrap();
            let now = Utc::now();
            Ok(ServiceCallResult {
                service_id: command.service_id.clone(),
                output: serde_json::to_value(EntitlementRecord {
                    entitlement_id: typed
                        .scope
                        .entitlement_id
                        .unwrap_or_else(|| EntitlementId::new("entitlement.revoked")),
                    package_id: typed.scope.package_id.unwrap(),
                    developer_id: typed.scope.developer_id.unwrap(),
                    state: EntitlementState::revoked(),
                    granted_at: now,
                    updated_at: now,
                    expires_at: None,
                    metadata: Default::default(),
                })
                .unwrap(),
            })
        }
    }

    #[tokio::test]
    async fn service_backed_entitlement_client_dispatches_revoke_command() {
        let client = ServiceBackedEntitlementClient::new(Arc::new(EchoEntitlementServiceClient));
        let record = client
            .revoke(EntitlementRevokeCommand {
                trace: TraceContext::new("trace-sdk-entitlement-revoke"),
                scope: EntitlementServiceScope::package(
                    PackageId::new("pkg.sdk"),
                    DeveloperId::new("developer.sdk"),
                ),
                reason: "test revoke".into(),
            })
            .await
            .unwrap();

        assert_eq!(record.package_id.as_str(), "pkg.sdk");
        assert_eq!(record.state, EntitlementState::revoked());
    }
}
