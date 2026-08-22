//! Runtime-host provider for provider-neutral issue-tracker commands.
use crate::developer_issue_tracker_strategy::{
    ConfiguredDeveloperIssueTrackerStrategy, DeveloperIssueTrackerProviderStrategy,
};
use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::domain_pack_contract::developer_issue_tracker::{
    DEVELOPER_ISSUE_TRACKER_COMMANDS, DEVELOPER_ISSUE_TRACKER_PACK_ID,
    DEVELOPER_ISSUE_TRACKER_SERVICE_ID, DEVELOPER_ISSUE_TRACKER_TRACE_EVENTS,
};
use macaca_proto::{
    domain_pack_command_trace, domain_pack_service_result, KernelServiceId, ServiceCallResult,
    ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, ServiceType,
    TraceSchemaRef,
};
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::RwLock;

/// Retains bounded opaque issue references and never stores comments, identities, or attachments.
pub struct DeveloperIssueTrackerSystemServiceProvider {
    descriptor: ServiceDescriptor,
    references: RwLock<BTreeMap<String, String>>,
    unavailable_reason: Option<String>,
    strategy: Arc<dyn DeveloperIssueTrackerProviderStrategy>,
}
impl DeveloperIssueTrackerSystemServiceProvider {
    pub fn mock() -> Self {
        Self::new(
            None,
            Arc::new(ConfiguredDeveloperIssueTrackerStrategy::mock()),
        )
    }
    pub fn mock_with_commands<I, S>(commands: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::new(
            None,
            Arc::new(ConfiguredDeveloperIssueTrackerStrategy::with_commands(
                commands,
            )),
        )
    }
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(
            Some(reason.into()),
            Arc::new(ConfiguredDeveloperIssueTrackerStrategy::unavailable()),
        )
    }
    fn new(
        reason: Option<String>,
        strategy: Arc<dyn DeveloperIssueTrackerProviderStrategy>,
    ) -> Self {
        Self {
            descriptor: developer_issue_tracker_service_descriptor(),
            references: RwLock::new(BTreeMap::new()),
            unavailable_reason: reason,
            strategy,
        }
    }
    pub async fn snapshot(&self) -> BTreeMap<String, String> {
        BTreeMap::from([
            ("pack_id".into(), DEVELOPER_ISSUE_TRACKER_PACK_ID.into()),
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
                "issue_comment_identity_attachment_timeline_references_only".into(),
            ),
        ])
    }
    async fn shutdown(&self) {
        self.references.write().await.clear();
    }
}
#[async_trait]
impl SystemService for DeveloperIssueTrackerSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }
    async fn start(&self) -> ServiceResult<()> {
        Ok(())
    }
    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = domain_pack_command_trace(&command)?;
        if let Some(reason) = &self.unavailable_reason {
            return Err(ServiceError::ServiceUnavailable(sanitize(reason)));
        }
        if !DEVELOPER_ISSUE_TRACKER_COMMANDS.contains(&command.name.as_str()) {
            return Err(ServiceError::UnsupportedCommand(
                "issue_tracker_command_unsupported".into(),
            ));
        }
        self.strategy.validate_command(command.name.as_str())?;
        if let Some(reason) = denied(&command.payload, command.name.as_str()) {
            return Err(ServiceError::DisabledByPolicy(reason.into()));
        }
        if self.references.read().await.len() >= 256 {
            return Err(ServiceError::DisabledByPolicy("quota_exceeded".into()));
        }
        let reference = format!("issue-tracker:reference:{}", trace.trace_id);
        self.references
            .write()
            .await
            .insert(trace.trace_id.clone(), reference.clone());
        Ok(domain_pack_service_result(
            serde_json::json!({"status":"ok","issue_ref":reference,"provider_class":self.strategy.provider_class(),"content":"redacted","replay_ref":format!("replay:{}", trace.trace_id)}),
            trace,
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
            .map_or(Ok(ServiceHealth::Healthy), |reason| {
                Ok(ServiceHealth::Unavailable {
                    reason: sanitize(reason),
                })
            })
    }
}
fn denied(payload: &serde_json::Value, command: &str) -> Option<&'static str> {
    [
        "policy_denied",
        "project_denied",
        "security_issue",
        "customer_data",
        "stale_version",
        "schema_mismatch",
        "transition_denied",
        "attachment_denied",
        "network_denied",
        "quota_exceeded",
        "timeout",
        "cancelled",
        "approval_required",
    ]
    .into_iter()
    .find(|key| payload.get(*key).and_then(serde_json::Value::as_bool) == Some(true))
    .or_else(|| {
        ((command.ends_with("_request")
            || command == "issue_tracker.manage_labels"
            || command == "issue_tracker.manage_assignees"
            || command == "issue_tracker.manage_relations")
            && payload.get("approved").and_then(serde_json::Value::as_bool) != Some(true))
        .then_some("approval_required")
    })
}
fn sanitize(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .take(64)
        .collect()
}
pub fn developer_issue_tracker_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(DEVELOPER_ISSUE_TRACKER_SERVICE_ID),
        ServiceType::new("developer.issue_tracker"),
        TraceSchemaRef::new("developer.issue_tracker.replay.v1"),
    );
    descriptor
        .metadata
        .insert("pack_id".into(), DEVELOPER_ISSUE_TRACKER_PACK_ID.into());
    descriptor.metadata.insert(
        "command_count".into(),
        DEVELOPER_ISSUE_TRACKER_COMMANDS.len().to_string(),
    );
    descriptor.metadata.insert(
        "trace_event_count".into(),
        DEVELOPER_ISSUE_TRACKER_TRACE_EVENTS.len().to_string(),
    );
    descriptor
}
