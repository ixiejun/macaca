use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    ApplicationExecutionCommandStatus, ApplicationExecutionControlCommand,
    ApplicationExecutionControlKind, ApplicationExecutionControlResult,
    ApplicationExecutionEventType, ApplicationExecutionHeartbeatPolicy,
    ApplicationExecutionLifecycleState, ApplicationExecutionPayload,
    ApplicationExecutionProviderDescriptor, ApplicationExecutionProviderHealth,
    ApplicationExecutionProviderKind, ApplicationExecutionProviderPreference,
    ApplicationExecutionScope, ApplicationExecutionSnapshot, ApplicationId, CapabilityId,
    ServiceError, StartApplicationExecutionCommand, StartApplicationExecutionResult, TraceContext,
};

use crate::{
    ApplicationExecutionProvider, ApplicationExecutionProviderRegistry,
    ApplicationExecutionProviderSelectionCriteria, ApplicationExecutionProviderSelectionPolicy,
};

#[tokio::test]
async fn provider_trait_defaults_report_health_resume_and_shutdown_truthfully() {
    let provider = TestProvider::new(
        "provider-defaults",
        ApplicationExecutionProviderKind::RemoteAgent,
        vec![CapabilityId::new("capability.application_execution")],
    );
    let snapshot = ApplicationExecutionSnapshot {
        scope: ApplicationExecutionScope::new(
            ApplicationId::from_name("neutral-application"),
            "session-defaults",
            "run-defaults",
            "test",
        )
        .unwrap(),
        lifecycle_state: ApplicationExecutionLifecycleState::Running,
        provider_id: Some("provider-defaults".into()),
        provider_kind: ApplicationExecutionProviderKind::RemoteAgent,
        latest_event_cursor: Some("event/7".into()),
        latest_checkpoint_ref: None,
        metadata: Default::default(),
    };

    assert_eq!(
        provider.health().await.unwrap(),
        ApplicationExecutionProviderHealth::Healthy
    );
    assert_eq!(
        provider.resume(snapshot).await.unwrap().status,
        ApplicationExecutionCommandStatus::Unsupported
    );
    provider.shutdown().await.unwrap();
}

#[tokio::test]
async fn provider_selection_uses_declared_capabilities_and_preference_only() {
    let requested = CapabilityId::new("capability.application_execution");
    let registry = ApplicationExecutionProviderRegistry::new()
        .register(Arc::new(TestProvider::new(
            "domain-looking-provider-id-is-still-just-data",
            ApplicationExecutionProviderKind::ExternalAppBackend,
            vec![CapabilityId::new("capability.unrelated")],
        )))
        .unwrap()
        .register(Arc::new(TestProvider::new(
            "generic-remote-provider",
            ApplicationExecutionProviderKind::RemoteAgent,
            vec![requested.clone()],
        )))
        .unwrap();

    let selection = registry
        .select(
            Some(&ApplicationExecutionProviderPreference {
                provider_kind: ApplicationExecutionProviderKind::RemoteAgent,
                provider_id: None,
            }),
            &[requested],
        )
        .await
        .unwrap();

    assert_eq!(selection.descriptor.provider_id, "generic-remote-provider");
    assert_eq!(selection.considered.len(), 2);
    assert_eq!(
        selection.considered[0].reason,
        "provider kind preference mismatch"
    );
}

#[tokio::test]
async fn provider_selection_applies_profile_policy_tenant_and_dynamic_health() {
    let requested = CapabilityId::new("capability.application_execution");
    let registry = ApplicationExecutionProviderRegistry::new()
        .register(Arc::new(
            TestProvider::new(
                "provider-policy-denied",
                ApplicationExecutionProviderKind::MacacaHosted,
                vec![requested.clone()],
            )
            .with_resource_profile("application_execution.profile", "interactive")
            .with_resource_profile("application_execution.tenant", "tenant-a"),
        ))
        .unwrap()
        .register(Arc::new(
            TestProvider::new(
                "provider-dynamic-unhealthy",
                ApplicationExecutionProviderKind::RemoteAgent,
                vec![requested.clone()],
            )
            .with_resource_profile("application_execution.profile", "interactive")
            .with_resource_profile("application_execution.tenant", "tenant-a")
            .with_runtime_health(ApplicationExecutionProviderHealth::Unavailable {
                reason: "worker lease capacity exhausted".into(),
            }),
        ))
        .unwrap()
        .register(Arc::new(
            TestProvider::new(
                "provider-tenant-mismatch",
                ApplicationExecutionProviderKind::ExternalAppBackend,
                vec![requested.clone()],
            )
            .with_resource_profile("application_execution.profile", "interactive")
            .with_resource_profile("application_execution.tenant", "tenant-b"),
        ))
        .unwrap()
        .register(Arc::new(
            TestProvider::new(
                "provider-selected",
                ApplicationExecutionProviderKind::ExternalAppBackend,
                vec![requested.clone()],
            )
            .with_resource_profile("application_execution.profile", "interactive")
            .with_resource_profile("application_execution.tenant", "tenant-a"),
        ))
        .unwrap();
    let criteria = ApplicationExecutionProviderSelectionCriteria {
        provider_preference: None,
        required_capabilities: vec![requested],
        execution_profile: Some("interactive".into()),
        tenant_id: Some("tenant-a".into()),
        policy: ApplicationExecutionProviderSelectionPolicy {
            denied_provider_ids: BTreeSet::from([String::from("provider-policy-denied")]),
            ..ApplicationExecutionProviderSelectionPolicy::default()
        },
    };

    let selection = registry.select_with_criteria(criteria).await.unwrap();

    assert_eq!(selection.descriptor.provider_id, "provider-selected");
    assert_eq!(selection.considered.len(), 4);
    assert_eq!(selection.considered[0].reason, "provider policy denied");
    assert_eq!(
        selection.considered[1].reason,
        "provider health unavailable"
    );
    assert_eq!(
        selection.considered[2].reason,
        "provider tenant constraint mismatch"
    );
}

#[test]
fn selection_criteria_from_start_command_preserves_generic_policy_context() {
    let criteria = ApplicationExecutionProviderSelectionCriteria::from_start_command(
        &StartApplicationExecutionCommand {
            application_id: ApplicationId::from_name("neutral-application"),
            session_id: Some("session-policy".into()),
            run_id: Some("run-policy".into()),
            task_input: ApplicationExecutionPayload::summary("generic task"),
            workspace_ref: None,
            requested_capabilities: vec![CapabilityId::new("capability.application_execution")],
            provider_preference: Some(ApplicationExecutionProviderPreference {
                provider_kind: ApplicationExecutionProviderKind::ExternalAppBackend,
                provider_id: None,
            }),
            trace: TraceContext::new("trace-selection-criteria"),
            policy_context: BTreeMap::from([
                (
                    String::from("application_execution.profile"),
                    String::from("interactive"),
                ),
                (
                    String::from("application_execution.policy.denied_provider_ids"),
                    String::from("provider-a, provider-b"),
                ),
            ]),
            tenant_id: Some("tenant-a".into()),
            actor: "runtime-test".into(),
            idempotency_key: "criteria-from-command".into(),
        },
    );

    assert_eq!(criteria.execution_profile.as_deref(), Some("interactive"));
    assert_eq!(criteria.tenant_id.as_deref(), Some("tenant-a"));
    assert!(criteria.policy.denied_provider_ids.contains("provider-a"));
    assert!(criteria.policy.denied_provider_ids.contains("provider-b"));
}

struct TestProvider {
    descriptor: ApplicationExecutionProviderDescriptor,
    runtime_health: Option<ApplicationExecutionProviderHealth>,
}

impl TestProvider {
    fn new(
        provider_id: &str,
        provider_kind: ApplicationExecutionProviderKind,
        capability_declarations: Vec<CapabilityId>,
    ) -> Self {
        Self {
            descriptor: ApplicationExecutionProviderDescriptor {
                provider_id: provider_id.into(),
                provider_kind,
                protocol_version: "application-execution.v1".into(),
                supported_commands: vec![ApplicationExecutionControlKind::Cancel],
                supported_events: vec![ApplicationExecutionEventType::ExecutionCompleted],
                checkpoint_support: false,
                heartbeat_policy: ApplicationExecutionHeartbeatPolicy {
                    interval_ms: 1000,
                    timeout_ms: 5000,
                    required: false,
                },
                control_delivery: "test".into(),
                capability_declarations,
                resource_profile: Default::default(),
                transport_kind: "test".into(),
                health_state: ApplicationExecutionProviderHealth::Healthy,
            },
            runtime_health: None,
        }
    }

    fn with_resource_profile(mut self, key: &str, value: &str) -> Self {
        self.descriptor
            .resource_profile
            .insert(key.into(), value.into());
        self
    }

    fn with_runtime_health(mut self, health: ApplicationExecutionProviderHealth) -> Self {
        self.runtime_health = Some(health);
        self
    }
}

#[async_trait]
impl ApplicationExecutionProvider for TestProvider {
    fn describe(&self) -> ApplicationExecutionProviderDescriptor {
        self.descriptor.clone()
    }

    async fn start(
        &self,
        command: StartApplicationExecutionCommand,
    ) -> Result<StartApplicationExecutionResult, ServiceError> {
        Ok(StartApplicationExecutionResult {
            status: ApplicationExecutionCommandStatus::Accepted,
            session_id: command.session_id,
            run_id: command.run_id,
            provider_id: Some(self.descriptor.provider_id.clone()),
            provider_kind: self.descriptor.provider_kind,
            event_cursor: None,
            control_ref: None,
            workspace_ref: command.workspace_ref,
            error: None,
        })
    }

    async fn control(
        &self,
        command: ApplicationExecutionControlCommand,
    ) -> Result<ApplicationExecutionControlResult, ServiceError> {
        Ok(ApplicationExecutionControlResult {
            status: ApplicationExecutionCommandStatus::Delivered,
            scope: command.scope,
            provider_id: Some(self.descriptor.provider_id.clone()),
            provider_kind: self.descriptor.provider_kind,
            event_cursor: None,
            error: None,
        })
    }

    async fn snapshot(&self) -> Result<Option<ApplicationExecutionSnapshot>, ServiceError> {
        Ok(None)
    }

    async fn health(&self) -> Result<ApplicationExecutionProviderHealth, ServiceError> {
        Ok(self
            .runtime_health
            .clone()
            .unwrap_or_else(|| self.descriptor.health_state.clone()))
    }
}
