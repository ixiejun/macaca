//! Runtime-host browser automation service with opaque handles and fail-closed policy gates.
use crate::developer_browser_automation_strategy::{
    ConfiguredDeveloperBrowserAutomationStrategy, DeveloperBrowserAutomationProviderStrategy,
};
use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::domain_pack_contract::developer_browser_automation::{
    BrowserProviderCapability, DEVELOPER_BROWSER_AUTOMATION_COMMANDS,
    DEVELOPER_BROWSER_AUTOMATION_PACK_ID, DEVELOPER_BROWSER_AUTOMATION_SERVICE_ID,
    DEVELOPER_BROWSER_AUTOMATION_TRACE_EVENTS,
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
pub enum DeveloperBrowserAutomationRuntimeEventKind {
    PackDeclared,
    AdmissionValidated,
    PolicyDecision,
    EntitlementChecked,
    ResourceReserved,
    ServiceCall,
    ProviderInspected,
    ContextOpened,
    PageOpened,
    Navigation,
    ActionRequested,
    EvaluationRequested,
    ArtifactRecorded,
    Unavailable,
    ProviderCallSucceeded,
    ProviderCallFailed,
    HealthReported,
    SnapshotRecorded,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeveloperBrowserAutomationRuntimeEvent {
    pub command: String,
    pub trace_id: String,
    pub kind: DeveloperBrowserAutomationRuntimeEventKind,
    pub replay_ref: String,
}

pub struct DeveloperBrowserAutomationSystemServiceProvider {
    descriptor: ServiceDescriptor,
    events: broadcast::Sender<DeveloperBrowserAutomationRuntimeEvent>,
    references: RwLock<BTreeMap<String, String>>,
    unavailable_reason: Option<String>,
    strategy: Arc<dyn DeveloperBrowserAutomationProviderStrategy>,
}

impl DeveloperBrowserAutomationSystemServiceProvider {
    pub fn mock() -> Self {
        Self::new(None)
    }
    pub fn mock_with_commands<I, S>(commands: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut provider = Self::new(None);
        provider.strategy = Arc::new(ConfiguredDeveloperBrowserAutomationStrategy::with_commands(
            commands,
        ));
        provider
    }
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(Some(reason.into()))
    }
    fn new(unavailable_reason: Option<String>) -> Self {
        let (events, _) = broadcast::channel(256);
        let strategy: Arc<dyn DeveloperBrowserAutomationProviderStrategy> =
            Arc::new(if unavailable_reason.is_some() {
                ConfiguredDeveloperBrowserAutomationStrategy::unavailable()
            } else {
                ConfiguredDeveloperBrowserAutomationStrategy::mock()
            });
        Self {
            descriptor: developer_browser_automation_service_descriptor(),
            events,
            references: RwLock::new(BTreeMap::new()),
            unavailable_reason,
            strategy,
        }
    }
    pub fn capability(&self) -> BrowserProviderCapability {
        self.strategy.capability()
    }
    pub fn subscribe(&self) -> broadcast::Receiver<DeveloperBrowserAutomationRuntimeEvent> {
        self.events.subscribe()
    }
    pub async fn snapshot(&self) -> BTreeMap<String, String> {
        let count = self.references.read().await.len().min(256);
        let _ = self.events.send(event(
            "browser.snapshot",
            "snapshot:browser",
            DeveloperBrowserAutomationRuntimeEventKind::SnapshotRecorded,
        ));
        BTreeMap::from([
            (
                "pack_id".into(),
                DEVELOPER_BROWSER_AUTOMATION_PACK_ID.into(),
            ),
            ("provider_class".into(), self.capability().provider_class),
            ("active_reference_count".into(), count.to_string()),
            (
                "redaction_profile".into(),
                "handles_hashes_and_policy_metadata_only".into(),
            ),
        ])
    }
    pub async fn shutdown(&self) -> ServiceResult<()> {
        self.references.write().await.clear();
        Ok(())
    }
}

#[async_trait]
impl SystemService for DeveloperBrowserAutomationSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        let mut d = self.descriptor.clone();
        d.metadata
            .insert("provider_class".into(), self.capability().provider_class);
        d
    }
    async fn start(&self) -> ServiceResult<()> {
        let _ = self.events.send(event(
            "browser.declaration",
            "declaration:browser",
            DeveloperBrowserAutomationRuntimeEventKind::PackDeclared,
        ));
        Ok(())
    }
    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = domain_pack_command_trace(&command)?;
        if let Some(reason) = &self.unavailable_reason {
            let _ = self.events.send(event(
                &command.name.to_string(),
                &trace.trace_id,
                DeveloperBrowserAutomationRuntimeEventKind::Unavailable,
            ));
            warn!(service_id=DEVELOPER_BROWSER_AUTOMATION_SERVICE_ID,command=%command.name,"browser automation provider unavailable");
            return Err(ServiceError::ServiceUnavailable(sanitize(reason)));
        }
        if !DEVELOPER_BROWSER_AUTOMATION_COMMANDS.contains(&command.name.as_str()) {
            return Err(ServiceError::UnsupportedCommand(
                "browser_command_unsupported".into(),
            ));
        }
        self.strategy.validate_command(command.name.as_str())?;
        if let Some(reason) = policy_denial(&command.payload) {
            let _ = self.events.send(event(
                "browser.policy_decision",
                &trace.trace_id,
                DeveloperBrowserAutomationRuntimeEventKind::PolicyDecision,
            ));
            return Err(ServiceError::DisabledByPolicy(reason.into()));
        }
        if command
            .payload
            .get("stale_handle")
            .and_then(serde_json::Value::as_bool)
            == Some(true)
        {
            return Err(ServiceError::InvalidArgument("stale_handle".into()));
        }
        if self.references.read().await.len() >= 256 {
            return Err(ServiceError::DisabledByPolicy("quota_exceeded".into()));
        }
        let handle = format!("browser:handle:{}", trace.trace_id);
        self.references
            .write()
            .await
            .insert(trace.trace_id.clone(), handle.clone());
        for kind in [
            DeveloperBrowserAutomationRuntimeEventKind::AdmissionValidated,
            DeveloperBrowserAutomationRuntimeEventKind::EntitlementChecked,
            DeveloperBrowserAutomationRuntimeEventKind::ResourceReserved,
            command_event(command.name.as_str()),
            DeveloperBrowserAutomationRuntimeEventKind::ProviderCallSucceeded,
        ] {
            let _ = self
                .events
                .send(event(&command.name.to_string(), &trace.trace_id, kind));
        }
        Ok(domain_pack_service_result(
            serde_json::json!({"status":"ok","handle":handle,"provider_class":"mock","freshness":"current","redaction":"handles_only","raw_dom":false,"raw_network":false,"raw_cookies":false}),
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
        let health = self
            .unavailable_reason
            .as_ref()
            .map_or(Ok(ServiceHealth::Healthy), |r| {
                Ok(ServiceHealth::Unavailable {
                    reason: sanitize(r),
                })
            });
        let _ = self.events.send(event(
            "browser.health",
            "health:browser",
            DeveloperBrowserAutomationRuntimeEventKind::HealthReported,
        ));
        health
    }
}

fn policy_denial(payload: &serde_json::Value) -> Option<&'static str> {
    [
        "policy_denied",
        "origin_denied",
        "credential_denied",
        "script_denied",
        "artifact_denied",
        "storage_denied",
        "network_denied",
        "approval_required",
        "timeout",
        "cancelled",
        "quota_exceeded",
    ]
    .into_iter()
    .find(|key| payload.get(*key).and_then(serde_json::Value::as_bool) == Some(true))
}
fn command_event(command: &str) -> DeveloperBrowserAutomationRuntimeEventKind {
    match command {
        "browser.inspect_provider" => DeveloperBrowserAutomationRuntimeEventKind::ProviderInspected,
        "browser.open_context_request" => DeveloperBrowserAutomationRuntimeEventKind::ContextOpened,
        "browser.open_page" => DeveloperBrowserAutomationRuntimeEventKind::PageOpened,
        "browser.navigate" => DeveloperBrowserAutomationRuntimeEventKind::Navigation,
        "browser.action_request" => DeveloperBrowserAutomationRuntimeEventKind::ActionRequested,
        "browser.evaluate_request" => {
            DeveloperBrowserAutomationRuntimeEventKind::EvaluationRequested
        }
        c if c.contains("screenshot") || c.contains("download") || c.contains("upload") => {
            DeveloperBrowserAutomationRuntimeEventKind::ArtifactRecorded
        }
        _ => DeveloperBrowserAutomationRuntimeEventKind::ServiceCall,
    }
}
fn sanitize(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .take(64)
        .collect()
}
fn event(
    command: &str,
    trace_id: &str,
    kind: DeveloperBrowserAutomationRuntimeEventKind,
) -> DeveloperBrowserAutomationRuntimeEvent {
    DeveloperBrowserAutomationRuntimeEvent {
        command: command.into(),
        trace_id: trace_id.into(),
        kind,
        replay_ref: format!("replay:{trace_id}"),
    }
}
pub fn developer_browser_automation_service_descriptor() -> ServiceDescriptor {
    let mut d = ServiceDescriptor::new(
        KernelServiceId::new(DEVELOPER_BROWSER_AUTOMATION_SERVICE_ID),
        ServiceType::new("developer.browser_automation"),
        TraceSchemaRef::new("developer.browser_automation.replay.v1"),
    );
    d.metadata.insert(
        "pack_id".into(),
        DEVELOPER_BROWSER_AUTOMATION_PACK_ID.into(),
    );
    d.metadata.insert("provider_class".into(), "mock".into());
    d.metadata.insert(
        "command_count".into(),
        DEVELOPER_BROWSER_AUTOMATION_COMMANDS.len().to_string(),
    );
    d.metadata.insert(
        "trace_event_count".into(),
        DEVELOPER_BROWSER_AUTOMATION_TRACE_EVENTS.len().to_string(),
    );
    d
}
