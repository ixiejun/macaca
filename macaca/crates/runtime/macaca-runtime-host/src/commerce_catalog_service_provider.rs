//! Runtime-host service provider for the provider-neutral commerce catalog pack.
use crate::commerce_catalog_strategy::{
    CommerceCatalogProviderStrategy, ConfiguredCommerceCatalogStrategy,
};
use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::domain_pack_contract::commerce_catalog::{
    CatalogProviderCapability, COMMERCE_CATALOG_COMMANDS, COMMERCE_CATALOG_PACK_ID,
    COMMERCE_CATALOG_SERVICE_ID, COMMERCE_CATALOG_TRACE_EVENTS,
};
use macaca_proto::{
    domain_pack_command_trace, domain_pack_service_result, KernelServiceId, ServiceCallResult,
    ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, ServiceType,
    TraceSchemaRef,
};
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommerceCatalogRuntimeEventKind {
    PackDeclared,
    AdmissionValidated,
    PolicyDecision,
    EntitlementChecked,
    ApprovalChecked,
    ResourceReserved,
    ProviderInspected,
    ServiceCall,
    MutationPlanned,
    PublicationRequested,
    ExportPlanned,
    ProviderCallSucceeded,
    ProviderCallFailed,
    Unavailable,
    HealthReported,
    SnapshotRecorded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommerceCatalogRuntimeEvent {
    pub command: String,
    pub trace_id: String,
    pub kind: CommerceCatalogRuntimeEventKind,
    pub replay_ref: String,
}

pub struct CommerceCatalogSystemServiceProvider {
    descriptor: ServiceDescriptor,
    events: broadcast::Sender<CommerceCatalogRuntimeEvent>,
    references: RwLock<BTreeMap<String, String>>,
    unavailable_reason: Option<String>,
    strategy: Arc<dyn CommerceCatalogProviderStrategy>,
}

impl CommerceCatalogSystemServiceProvider {
    pub fn mock() -> Self {
        Self::new(None)
    }
    pub fn mock_with_commands<I, S>(commands: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut provider = Self::new(None);
        provider.strategy = Arc::new(ConfiguredCommerceCatalogStrategy::with_commands(commands));
        provider
    }
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(Some(reason.into()))
    }
    fn new(unavailable_reason: Option<String>) -> Self {
        let (events, _) = broadcast::channel(256);
        let strategy: Arc<dyn CommerceCatalogProviderStrategy> =
            Arc::new(if unavailable_reason.is_some() {
                ConfiguredCommerceCatalogStrategy::unavailable()
            } else {
                ConfiguredCommerceCatalogStrategy::mock()
            });
        Self {
            descriptor: commerce_catalog_service_descriptor(),
            events,
            references: RwLock::new(BTreeMap::new()),
            unavailable_reason,
            strategy,
        }
    }
    pub fn capability(&self) -> CatalogProviderCapability {
        self.strategy.capability()
    }
    pub fn subscribe(&self) -> broadcast::Receiver<CommerceCatalogRuntimeEvent> {
        self.events.subscribe()
    }
    pub async fn snapshot(&self) -> BTreeMap<String, String> {
        let count = self.references.read().await.len().min(256);
        let _ = self.events.send(event(
            "catalog.snapshot",
            "snapshot:commerce-catalog",
            CommerceCatalogRuntimeEventKind::SnapshotRecorded,
        ));
        BTreeMap::from([
            ("pack_id".into(), COMMERCE_CATALOG_PACK_ID.into()),
            ("provider_class".into(), self.capability().provider_class),
            ("active_reference_count".into(), count.to_string()),
            (
                "redaction_profile".into(),
                "references_hashes_and_dataset_metadata_only".into(),
            ),
        ])
    }
    pub async fn shutdown(&self) -> ServiceResult<()> {
        self.references.write().await.clear();
        Ok(())
    }
}

#[async_trait]
impl SystemService for CommerceCatalogSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        let mut descriptor = self.descriptor.clone();
        descriptor
            .metadata
            .insert("provider_class".into(), self.capability().provider_class);
        descriptor
    }
    async fn start(&self) -> ServiceResult<()> {
        let _ = self.events.send(event(
            "catalog.declaration",
            "declaration:commerce-catalog",
            CommerceCatalogRuntimeEventKind::PackDeclared,
        ));
        Ok(())
    }
    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = domain_pack_command_trace(&command)?;
        if let Some(reason) = &self.unavailable_reason {
            let _ = self.events.send(event(
                &command.name.to_string(),
                &trace.trace_id,
                CommerceCatalogRuntimeEventKind::Unavailable,
            ));
            warn!(service_id = COMMERCE_CATALOG_SERVICE_ID, command = %command.name, reason_code = %reason, "commerce catalog provider unavailable");
            return Err(ServiceError::ServiceUnavailable(sanitize(reason)));
        }
        if !COMMERCE_CATALOG_COMMANDS.contains(&command.name.as_str()) {
            return Err(ServiceError::UnsupportedCommand(
                "catalog_command_unsupported".into(),
            ));
        }
        self.strategy.validate_command(command.name.as_str())?;
        if let Some(reason) = admission_denial(&command.payload) {
            let _ = self.events.send(event(
                "catalog.policy_decision",
                &trace.trace_id,
                CommerceCatalogRuntimeEventKind::PolicyDecision,
            ));
            return Err(ServiceError::DisabledByPolicy(reason.into()));
        }
        if self.references.read().await.len() >= 256 {
            return Err(ServiceError::DisabledByPolicy("quota_exceeded".into()));
        }
        let reference = format!("catalog:reference:{}", trace.trace_id);
        self.references
            .write()
            .await
            .insert(trace.trace_id.clone(), reference.clone());
        for kind in [
            CommerceCatalogRuntimeEventKind::AdmissionValidated,
            CommerceCatalogRuntimeEventKind::EntitlementChecked,
            CommerceCatalogRuntimeEventKind::ResourceReserved,
            event_kind(command.name.as_str()),
            CommerceCatalogRuntimeEventKind::ProviderCallSucceeded,
        ] {
            let _ = self
                .events
                .send(event(&command.name.to_string(), &trace.trace_id, kind));
        }
        Ok(domain_pack_service_result(
            serde_json::json!({"status":"ok","catalog_ref":reference,"provider_class":"mock","freshness":"current","dataset_version":"synthetic-catalog-v1","redaction":"references_only"}),
            trace,
            "mock",
        ))
    }
    async fn stop(&self) -> ServiceResult<()> {
        self.shutdown().await
    }
    async fn cleanup(&self) -> ServiceResult<()> {
        self.shutdown().await
    }
    async fn health(&self) -> ServiceResult<ServiceHealth> {
        let result = self
            .unavailable_reason
            .as_ref()
            .map_or(Ok(ServiceHealth::Healthy), |r| {
                Ok(ServiceHealth::Unavailable {
                    reason: sanitize(r),
                })
            });
        let _ = self.events.send(event(
            "catalog.health",
            "health:commerce-catalog",
            CommerceCatalogRuntimeEventKind::HealthReported,
        ));
        result
    }
}

fn admission_denial(payload: &serde_json::Value) -> Option<&'static str> {
    [
        "policy_denied",
        "entitlement_missing",
        "approval_required",
        "unsupported_filter",
        "conflict",
        "quota_exceeded",
        "stale_data",
        "timeout",
        "cancelled",
        "export_denied",
    ]
    .into_iter()
    .into_iter()
    .find(|key| payload.get(*key).and_then(serde_json::Value::as_bool) == Some(true))
}
fn sanitize(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .take(64)
        .collect()
}
fn event_kind(command: &str) -> CommerceCatalogRuntimeEventKind {
    match command {
        "catalog.inspect_provider" | "catalog.describe_schema" => {
            CommerceCatalogRuntimeEventKind::ProviderInspected
        }
        c if c.contains("plan_") => CommerceCatalogRuntimeEventKind::MutationPlanned,
        "catalog.product_request" | "catalog.variant_request" | "catalog.media_request" => {
            CommerceCatalogRuntimeEventKind::PublicationRequested
        }
        "catalog.export_catalog" | "catalog.get_artifact_handle" | "catalog.plan_export" => {
            CommerceCatalogRuntimeEventKind::ExportPlanned
        }
        _ => CommerceCatalogRuntimeEventKind::ServiceCall,
    }
}
fn event(
    command: &str,
    trace_id: &str,
    kind: CommerceCatalogRuntimeEventKind,
) -> CommerceCatalogRuntimeEvent {
    CommerceCatalogRuntimeEvent {
        command: command.into(),
        trace_id: trace_id.into(),
        kind,
        replay_ref: format!("replay:{trace_id}"),
    }
}
pub fn commerce_catalog_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(COMMERCE_CATALOG_SERVICE_ID),
        ServiceType::new("commerce.catalog"),
        TraceSchemaRef::new("commerce.catalog.replay.v1"),
    );
    descriptor
        .metadata
        .insert("pack_id".into(), COMMERCE_CATALOG_PACK_ID.into());
    descriptor
        .metadata
        .insert("provider_class".into(), "mock".into());
    descriptor.metadata.insert(
        "command_count".into(),
        COMMERCE_CATALOG_COMMANDS.len().to_string(),
    );
    descriptor.metadata.insert(
        "trace_event_count".into(),
        COMMERCE_CATALOG_TRACE_EVENTS.len().to_string(),
    );
    descriptor
}
