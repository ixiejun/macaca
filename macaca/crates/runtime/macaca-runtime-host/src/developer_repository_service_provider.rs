//! Runtime-host provider for provider-neutral repository operations.
use crate::developer_repository_strategy::{
    ConfiguredDeveloperRepositoryStrategy, DeveloperRepositoryProviderStrategy,
};
use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::domain_pack_contract::developer_repository::{
    DEVELOPER_REPOSITORY_COMMANDS, DEVELOPER_REPOSITORY_PACK_ID, DEVELOPER_REPOSITORY_SERVICE_ID,
    DEVELOPER_REPOSITORY_TRACE_EVENTS,
};
use macaca_proto::{
    domain_pack_command_trace, domain_pack_service_result, KernelServiceId, ServiceCallResult,
    ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, ServiceType,
    TraceSchemaRef,
};
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::RwLock;
/// Retains bounded opaque repository references and never stores source, diffs, URLs, or credentials.
pub struct DeveloperRepositorySystemServiceProvider {
    descriptor: ServiceDescriptor,
    references: RwLock<BTreeMap<String, String>>,
    unavailable_reason: Option<String>,
    strategy: Arc<dyn DeveloperRepositoryProviderStrategy>,
}
impl DeveloperRepositorySystemServiceProvider {
    pub fn mock() -> Self {
        Self::new(
            None,
            Arc::new(ConfiguredDeveloperRepositoryStrategy::mock()),
        )
    }
    pub fn mock_with_commands<I, S>(c: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::new(
            None,
            Arc::new(ConfiguredDeveloperRepositoryStrategy::with_commands(c)),
        )
    }
    pub fn unavailable(r: impl Into<String>) -> Self {
        Self::new(
            Some(r.into()),
            Arc::new(ConfiguredDeveloperRepositoryStrategy::unavailable()),
        )
    }
    fn new(r: Option<String>, s: Arc<dyn DeveloperRepositoryProviderStrategy>) -> Self {
        Self {
            descriptor: developer_repository_service_descriptor(),
            references: RwLock::new(BTreeMap::new()),
            unavailable_reason: r,
            strategy: s,
        }
    }
    pub async fn snapshot(&self) -> BTreeMap<String, String> {
        BTreeMap::from([
            ("pack_id".into(), DEVELOPER_REPOSITORY_PACK_ID.into()),
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
                "repository_ref_status_diff_mutation_remote_metadata_only".into(),
            ),
        ])
    }
    async fn shutdown(&self) {
        self.references.write().await.clear();
    }
}
#[async_trait]
impl SystemService for DeveloperRepositorySystemServiceProvider {
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
        if !DEVELOPER_REPOSITORY_COMMANDS.contains(&c.name.as_str()) {
            return Err(ServiceError::UnsupportedCommand(
                "repository_command_unsupported".into(),
            ));
        }
        self.strategy.validate_command(c.name.as_str())?;
        if let Some(r) = denied(&c.payload, c.name.as_str()) {
            return Err(ServiceError::DisabledByPolicy(r.into()));
        }
        if self.references.read().await.len() >= 256 {
            return Err(ServiceError::DisabledByPolicy("quota_exceeded".into()));
        }
        let reference = format!("repository:reference:{}", t.trace_id);
        self.references
            .write()
            .await
            .insert(t.trace_id.clone(), reference.clone());
        Ok(domain_pack_service_result(
            serde_json::json!({"status":"ok","repository_ref":reference,"provider_class":self.strategy.provider_class(),"content":"redacted","replay_ref":format!("replay:{}",t.trace_id)}),
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
        "dirty_worktree",
        "diverged_ref",
        "protected_ref",
        "network_denied",
        "missing_credential",
        "unsupported_vcs",
        "quota_exceeded",
        "timeout",
        "cancelled",
        "approval_required",
    ]
    .into_iter()
    .find(|k| p.get(*k).and_then(serde_json::Value::as_bool) == Some(true))
    .or_else(|| {
        ((c.ends_with("_request")
            || c.contains("stage")
            || c.contains("commit")
            || c.contains("push")
            || c.contains("merge"))
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
pub fn developer_repository_service_descriptor() -> ServiceDescriptor {
    let mut d = ServiceDescriptor::new(
        KernelServiceId::new(DEVELOPER_REPOSITORY_SERVICE_ID),
        ServiceType::new("developer.repository"),
        TraceSchemaRef::new("developer.repository.replay.v1"),
    );
    d.metadata
        .insert("pack_id".into(), DEVELOPER_REPOSITORY_PACK_ID.into());
    d.metadata.insert(
        "command_count".into(),
        DEVELOPER_REPOSITORY_COMMANDS.len().to_string(),
    );
    d.metadata.insert(
        "trace_event_count".into(),
        DEVELOPER_REPOSITORY_TRACE_EVENTS.len().to_string(),
    );
    d
}
