//! Runtime-host CI service provider that exposes only redacted references.
use crate::developer_ci_strategy::{ConfiguredDeveloperCiStrategy, DeveloperCiProviderStrategy};
use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::domain_pack_contract::developer_ci::{
    CiProviderCapability, DEVELOPER_CI_COMMANDS, DEVELOPER_CI_PACK_ID, DEVELOPER_CI_SERVICE_ID,
    DEVELOPER_CI_TRACE_EVENTS,
};
use macaca_proto::{
    domain_pack_command_trace, domain_pack_service_result, KernelServiceId, ServiceCallResult,
    ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, ServiceType,
    TraceSchemaRef,
};
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::{broadcast, RwLock};
use tracing::warn;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeveloperCiRuntimeEventKind {
    PackDeclared,
    AdmissionValidated,
    PolicyDecision,
    EntitlementChecked,
    ResourceReserved,
    ProviderInspected,
    RunRequested,
    LogRead,
    ArtifactHandle,
    Unavailable,
    ProviderCallSucceeded,
    ProviderCallFailed,
    HealthReported,
    SnapshotRecorded,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeveloperCiRuntimeEvent {
    pub command: String,
    pub trace_id: String,
    pub kind: DeveloperCiRuntimeEventKind,
    pub replay_ref: String,
}
pub struct DeveloperCiSystemServiceProvider {
    descriptor: ServiceDescriptor,
    events: broadcast::Sender<DeveloperCiRuntimeEvent>,
    references: RwLock<BTreeMap<String, String>>,
    unavailable_reason: Option<String>,
    strategy: Arc<dyn DeveloperCiProviderStrategy>,
}
impl DeveloperCiSystemServiceProvider {
    pub fn mock() -> Self {
        Self::new(None)
    }
    pub fn mock_with_commands<I, S>(commands: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut p = Self::new(None);
        p.strategy = Arc::new(ConfiguredDeveloperCiStrategy::with_commands(commands));
        p
    }
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(Some(reason.into()))
    }
    fn new(unavailable_reason: Option<String>) -> Self {
        let (events, _) = broadcast::channel(256);
        let strategy: Arc<dyn DeveloperCiProviderStrategy> =
            Arc::new(if unavailable_reason.is_some() {
                ConfiguredDeveloperCiStrategy::unavailable()
            } else {
                ConfiguredDeveloperCiStrategy::mock()
            });
        Self {
            descriptor: developer_ci_service_descriptor(),
            events,
            references: RwLock::new(BTreeMap::new()),
            unavailable_reason,
            strategy,
        }
    }
    pub fn capability(&self) -> CiProviderCapability {
        self.strategy.capability()
    }
    pub fn subscribe(&self) -> broadcast::Receiver<DeveloperCiRuntimeEvent> {
        self.events.subscribe()
    }
    pub async fn snapshot(&self) -> BTreeMap<String, String> {
        let count = self.references.read().await.len().min(256);
        let _ = self.events.send(event(
            "ci.snapshot",
            "snapshot:ci",
            DeveloperCiRuntimeEventKind::SnapshotRecorded,
        ));
        BTreeMap::from([
            ("pack_id".into(), DEVELOPER_CI_PACK_ID.into()),
            ("provider_class".into(), self.capability().provider_class),
            ("active_reference_count".into(), count.to_string()),
            (
                "redaction_profile".into(),
                "run_log_and_artifact_references_only".into(),
            ),
        ])
    }
    pub async fn shutdown(&self) -> ServiceResult<()> {
        self.references.write().await.clear();
        Ok(())
    }
}
#[async_trait]
impl SystemService for DeveloperCiSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        let mut d = self.descriptor.clone();
        d.metadata
            .insert("provider_class".into(), self.capability().provider_class);
        d
    }
    async fn start(&self) -> ServiceResult<()> {
        let _ = self.events.send(event(
            "ci.declaration",
            "declaration:ci",
            DeveloperCiRuntimeEventKind::PackDeclared,
        ));
        Ok(())
    }
    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = domain_pack_command_trace(&command)?;
        if let Some(reason) = &self.unavailable_reason {
            let _ = self.events.send(event(
                &command.name.to_string(),
                &trace.trace_id,
                DeveloperCiRuntimeEventKind::Unavailable,
            ));
            warn!(service_id=DEVELOPER_CI_SERVICE_ID,command=%command.name,"CI provider unavailable");
            return Err(ServiceError::ServiceUnavailable(sanitize(reason)));
        }
        if !DEVELOPER_CI_COMMANDS.contains(&command.name.as_str()) {
            return Err(ServiceError::UnsupportedCommand(
                "ci_command_unsupported".into(),
            ));
        }
        self.strategy.validate_command(command.name.as_str())?;
        if let Some(reason) = denial(&command.payload) {
            return Err(ServiceError::DisabledByPolicy(reason.into()));
        }
        if self.references.read().await.len() >= 256 {
            return Err(ServiceError::DisabledByPolicy("quota_exceeded".into()));
        }
        let reference = format!("ci:reference:{}", trace.trace_id);
        self.references
            .write()
            .await
            .insert(trace.trace_id.clone(), reference.clone());
        for kind in [
            DeveloperCiRuntimeEventKind::AdmissionValidated,
            DeveloperCiRuntimeEventKind::EntitlementChecked,
            DeveloperCiRuntimeEventKind::ResourceReserved,
            command_event(command.name.as_str()),
            DeveloperCiRuntimeEventKind::ProviderCallSucceeded,
        ] {
            let _ = self
                .events
                .send(event(&command.name.to_string(), &trace.trace_id, kind));
        }
        Ok(domain_pack_service_result(
            serde_json::json!({"status":"ok","ci_ref":reference,"provider_class":"mock","freshness":"current","log":"redacted","artifact_bytes":false}),
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
        self.unavailable_reason
            .as_ref()
            .map_or(Ok(ServiceHealth::Healthy), |r| {
                Ok(ServiceHealth::Unavailable {
                    reason: sanitize(r),
                })
            })
    }
}
fn denial(p: &serde_json::Value) -> Option<&'static str> {
    [
        "policy_denied",
        "project_denied",
        "approval_required",
        "conflict",
        "stale_status",
        "quota_exceeded",
        "network_denied",
        "timeout",
        "cancelled",
        "artifact_denied",
    ]
    .into_iter()
    .find(|k| p.get(*k).and_then(serde_json::Value::as_bool) == Some(true))
}
fn command_event(c: &str) -> DeveloperCiRuntimeEventKind {
    match c {
        "ci.inspect_provider" => DeveloperCiRuntimeEventKind::ProviderInspected,
        c if c.contains("trigger") || c.contains("cancel") || c.contains("rerun") => {
            DeveloperCiRuntimeEventKind::RunRequested
        }
        c if c.contains("log") => DeveloperCiRuntimeEventKind::LogRead,
        c if c.contains("artifact") => DeveloperCiRuntimeEventKind::ArtifactHandle,
        _ => DeveloperCiRuntimeEventKind::ProviderCallSucceeded,
    }
}
fn sanitize(v: &str) -> String {
    v.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .take(64)
        .collect()
}
fn event(c: &str, t: &str, k: DeveloperCiRuntimeEventKind) -> DeveloperCiRuntimeEvent {
    DeveloperCiRuntimeEvent {
        command: c.into(),
        trace_id: t.into(),
        kind: k,
        replay_ref: format!("replay:{t}"),
    }
}
pub fn developer_ci_service_descriptor() -> ServiceDescriptor {
    let mut d = ServiceDescriptor::new(
        KernelServiceId::new(DEVELOPER_CI_SERVICE_ID),
        ServiceType::new("developer.ci"),
        TraceSchemaRef::new("developer.ci.replay.v1"),
    );
    d.metadata
        .insert("pack_id".into(), DEVELOPER_CI_PACK_ID.into());
    d.metadata.insert(
        "command_count".into(),
        DEVELOPER_CI_COMMANDS.len().to_string(),
    );
    d.metadata.insert(
        "trace_event_count".into(),
        DEVELOPER_CI_TRACE_EVENTS.len().to_string(),
    );
    d
}
