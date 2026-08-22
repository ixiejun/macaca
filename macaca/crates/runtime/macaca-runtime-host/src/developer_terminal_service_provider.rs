//! Runtime-host terminal provider that returns bounded handles and never spawns a process itself.
use crate::developer_terminal_strategy::{
    ConfiguredDeveloperTerminalStrategy, DeveloperTerminalProviderStrategy,
};
use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::domain_pack_contract::developer_terminal::{
    DEVELOPER_TERMINAL_COMMANDS, DEVELOPER_TERMINAL_PACK_ID, DEVELOPER_TERMINAL_SERVICE_ID,
    DEVELOPER_TERMINAL_TRACE_EVENTS,
};
use macaca_proto::{
    domain_pack_command_trace, domain_pack_service_result, KernelServiceId, ServiceCallResult,
    ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, ServiceType,
    TraceSchemaRef,
};
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::RwLock;
/// Stores opaque session/process references and bounded counters, never raw command output or env values.
pub struct DeveloperTerminalSystemServiceProvider {
    descriptor: ServiceDescriptor,
    references: RwLock<BTreeMap<String, String>>,
    unavailable_reason: Option<String>,
    strategy: Arc<dyn DeveloperTerminalProviderStrategy>,
}
impl DeveloperTerminalSystemServiceProvider {
    pub fn mock() -> Self {
        Self::new(None, Arc::new(ConfiguredDeveloperTerminalStrategy::mock()))
    }
    pub fn mock_with_commands<I, S>(c: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::new(
            None,
            Arc::new(ConfiguredDeveloperTerminalStrategy::with_commands(c)),
        )
    }
    pub fn unavailable(r: impl Into<String>) -> Self {
        Self::new(
            Some(r.into()),
            Arc::new(ConfiguredDeveloperTerminalStrategy::unavailable()),
        )
    }
    fn new(r: Option<String>, s: Arc<dyn DeveloperTerminalProviderStrategy>) -> Self {
        Self {
            descriptor: developer_terminal_service_descriptor(),
            references: RwLock::new(BTreeMap::new()),
            unavailable_reason: r,
            strategy: s,
        }
    }
    pub async fn snapshot(&self) -> BTreeMap<String, String> {
        BTreeMap::from([
            ("pack_id".into(), DEVELOPER_TERMINAL_PACK_ID.into()),
            (
                "provider_class".into(),
                self.strategy.provider_class().into(),
            ),
            (
                "reference_count".into(),
                self.references.read().await.len().min(256).to_string(),
            ),
            (
                "redaction_profile".into(),
                "process_stream_snapshot_handles_only".into(),
            ),
        ])
    }
    async fn shutdown(&self) {
        self.references.write().await.clear();
    }
}
#[async_trait]
impl SystemService for DeveloperTerminalSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }
    async fn start(&self) -> ServiceResult<()> {
        Ok(())
    }
    async fn call(&self, c: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let t = domain_pack_command_trace(&c)?;
        if let Some(r) = &self.unavailable_reason {
            return Err(ServiceError::ServiceUnavailable(sanitize(r)));
        }
        if !DEVELOPER_TERMINAL_COMMANDS.contains(&c.name.as_str()) {
            return Err(ServiceError::UnsupportedCommand(
                "terminal_command_unsupported".into(),
            ));
        }
        self.strategy.validate_command(c.name.as_str())?;
        if let Some(r) = denied(&c.payload, c.name.as_str()) {
            return Err(ServiceError::DisabledByPolicy(r.into()));
        }
        if self.references.read().await.len() >= 128 {
            return Err(ServiceError::DisabledByPolicy("quota_exceeded".into()));
        }
        let reference = format!("terminal:reference:{}", t.trace_id);
        self.references
            .write()
            .await
            .insert(t.trace_id.clone(), reference.clone());
        Ok(domain_pack_service_result(
            serde_json::json!({"status":"ok","terminal_ref":reference,"provider_class":self.strategy.provider_class(),"output":"redacted","replay_ref":format!("replay:{}",t.trace_id)}),
            t,
            self.strategy.provider_class(),
        ))
    }
    async fn stop(&self) -> ServiceResult<()> {
        self.shutdown().await;
        Ok(())
    }
    async fn cleanup(&self) -> ServiceResult<()> {
        self.shutdown().await;
        Ok(())
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
fn denied(p: &serde_json::Value, c: &str) -> Option<&'static str> {
    [
        "policy_denied",
        "invalid_command",
        "invalid_workdir",
        "invalid_env",
        "network_denied",
        "stdin_denied",
        "resize_unsupported",
        "stale_handle",
        "timeout",
        "cancelled",
        "approval_required",
        "quota_exceeded",
    ]
    .into_iter()
    .find(|k| p.get(*k).and_then(serde_json::Value::as_bool) == Some(true))
    .or_else(|| {
        ((c.ends_with("_request")
            || c.contains("stdin")
            || c.contains("resize")
            || c.contains("cancel"))
            && p.get("approved").and_then(serde_json::Value::as_bool) != Some(true))
        .then_some("approval_required")
    })
}
fn sanitize(v: &str) -> String {
    v.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .take(64)
        .collect()
}
pub fn developer_terminal_service_descriptor() -> ServiceDescriptor {
    let mut d = ServiceDescriptor::new(
        KernelServiceId::new(DEVELOPER_TERMINAL_SERVICE_ID),
        ServiceType::new("developer.terminal"),
        TraceSchemaRef::new("developer.terminal.replay.v1"),
    );
    d.metadata
        .insert("pack_id".into(), DEVELOPER_TERMINAL_PACK_ID.into());
    d.metadata.insert(
        "command_count".into(),
        DEVELOPER_TERMINAL_COMMANDS.len().to_string(),
    );
    d.metadata.insert(
        "trace_event_count".into(),
        DEVELOPER_TERMINAL_TRACE_EVENTS.len().to_string(),
    );
    d
}
