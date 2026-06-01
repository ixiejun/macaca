use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use macaca_proto::{
    ApplicationExecutionCommandStatus, ApplicationExecutionControlCommand,
    ApplicationExecutionControlKind, ApplicationExecutionHeartbeatPolicy,
    ApplicationExecutionPayload, ApplicationExecutionProviderKind, ApplicationExecutionScope,
    ApplicationId, CapabilityId, ExternalApplicationBackendExecutionProfile, ServiceError,
    StartApplicationExecutionCommand, TraceContext,
};

use crate::{
    ApplicationExecutionProvider, ExternalApplicationBackendProvider,
    ExternalApplicationBackendStartRequest, ExternalApplicationBackendTransport,
};

fn external_profile() -> ExternalApplicationBackendExecutionProfile {
    ExternalApplicationBackendExecutionProfile {
        provider_id: "provider.external.fixture".into(),
        start_endpoint: "https://backend.example.test/start".into(),
        control_endpoint: Some("https://backend.example.test/control".into()),
        protocol_version: "application-execution.v1".into(),
        callback_gateway_ref: Some("gateway/application-execution".into()),
        callback_identity_ref: "identity.external.fixture".into(),
        supported_controls: vec![
            ApplicationExecutionControlKind::Cancel,
            ApplicationExecutionControlKind::Approve,
        ],
        heartbeat_policy: ApplicationExecutionHeartbeatPolicy {
            interval_ms: 1_000,
            timeout_ms: 5_000,
            required: true,
        },
        request_timeout_ms: 3_000,
        event_schema_version: "application-execution.v1".into(),
        capability_declarations: vec![CapabilityId::new("capability.application_execution")],
        resource_profile: Default::default(),
    }
}

#[test]
fn external_backend_provider_admits_valid_profile_as_descriptor() {
    let provider = ExternalApplicationBackendProvider::from_profile(external_profile()).unwrap();
    let descriptor = provider.describe();

    assert_eq!(descriptor.provider_id, "provider.external.fixture");
    assert_eq!(
        descriptor.provider_kind,
        ApplicationExecutionProviderKind::ExternalAppBackend
    );
    assert_eq!(descriptor.transport_kind, "external_app_backend");
    assert!(descriptor.heartbeat_policy.required);
    assert_eq!(
        provider.callback_identity_ref(),
        "identity.external.fixture"
    );
    assert_eq!(provider.event_schema_version(), "application-execution.v1");
}

#[test]
fn external_backend_provider_rejects_incomplete_profile_before_registration() {
    let mut profile = external_profile();
    profile.start_endpoint.clear();

    let error = ExternalApplicationBackendProvider::from_profile(profile).unwrap_err();

    assert!(error
        .to_string()
        .contains("external backend start_endpoint"));
}

#[tokio::test]
async fn external_backend_start_calls_transport_with_scoped_lease_envelope() {
    let transport = Arc::new(FakeExternalBackendTransport::default());
    let provider =
        ExternalApplicationBackendProvider::with_transport(external_profile(), transport.clone())
            .expect("profile should admit");

    let result = provider.start(start_command()).await.unwrap();
    let request = transport.last_start();

    assert_eq!(result.status, ApplicationExecutionCommandStatus::Accepted);
    assert_eq!(
        result.provider_kind,
        ApplicationExecutionProviderKind::ExternalAppBackend
    );
    assert_eq!(
        result.provider_id.as_deref(),
        Some("provider.external.fixture")
    );
    assert!(result
        .control_ref
        .unwrap()
        .contains("provider.external.fixture"));
    assert_eq!(request.endpoint, "https://backend.example.test/start");
    assert_eq!(
        request.application_id,
        ApplicationId::from_name("application.external-start")
    );
    assert_eq!(
        request.session_id.as_deref(),
        Some("session-external-start")
    );
    assert_eq!(request.run_id.as_deref(), Some("run-external-start"));
    assert_eq!(
        request.callback_identity_ref, "identity.external.fixture",
        "the provider must send only a scoped callback identity reference, not raw credentials"
    );
    assert_eq!(request.allowed_controls.len(), 2);
    assert_eq!(request.event_schema_version, "application-execution.v1");
}

#[tokio::test]
async fn external_backend_control_forwards_to_control_endpoint_with_trace_scope() {
    let transport = Arc::new(FakeExternalBackendTransport::default());
    let provider =
        ExternalApplicationBackendProvider::with_transport(external_profile(), transport.clone())
            .expect("profile should admit");
    let _ = provider.start(start_command()).await.unwrap();

    let result = provider.control(control_command()).await.unwrap();
    let request = transport.last_control();

    assert_eq!(result.status, ApplicationExecutionCommandStatus::Delivered);
    assert_eq!(request.endpoint, "https://backend.example.test/control");
    assert_eq!(request.scope.session_id, "session-external-start");
    assert_eq!(request.scope.run_id, "run-external-start");
    assert_eq!(request.control_id, "control-external-1");
    assert_eq!(request.trace.trace_id, "trace-external-control");
}

fn start_command() -> StartApplicationExecutionCommand {
    StartApplicationExecutionCommand {
        application_id: ApplicationId::from_name("application.external-start"),
        session_id: Some("session-external-start".into()),
        run_id: Some("run-external-start".into()),
        task_input: ApplicationExecutionPayload::summary("start external backend execution"),
        workspace_ref: Some("workspace://session-external-start".into()),
        requested_capabilities: vec![CapabilityId::new("capability.application_execution")],
        provider_preference: None,
        trace: TraceContext::new("trace-external-start"),
        policy_context: Default::default(),
        tenant_id: Some("tenant-external".into()),
        actor: "runtime-test".into(),
        idempotency_key: "idem-external-start".into(),
    }
}

fn control_command() -> ApplicationExecutionControlCommand {
    ApplicationExecutionControlCommand {
        scope: ApplicationExecutionScope {
            application_id: ApplicationId::from_name("application.external-start"),
            session_id: "session-external-start".into(),
            run_id: "run-external-start".into(),
            tenant_id: Some("tenant-external".into()),
            actor: "runtime-test".into(),
        },
        command: ApplicationExecutionControlKind::Approve,
        control_id: "control-external-1".into(),
        reason_code: "operator_approval".into(),
        trace: TraceContext::new("trace-external-control"),
        policy_context: Default::default(),
        payload: Some(ApplicationExecutionPayload::summary("approval granted")),
        idempotency_key: "idem-external-control".into(),
    }
}

#[derive(Default)]
struct FakeExternalBackendTransport {
    starts: Mutex<Vec<ExternalApplicationBackendStartRequest>>,
    controls: Mutex<Vec<crate::ExternalApplicationBackendControlRequest>>,
}

impl FakeExternalBackendTransport {
    fn last_start(&self) -> ExternalApplicationBackendStartRequest {
        self.starts
            .lock()
            .expect("fake transport lock should not be poisoned")
            .last()
            .expect("start request should be recorded")
            .clone()
    }

    fn last_control(&self) -> crate::ExternalApplicationBackendControlRequest {
        self.controls
            .lock()
            .expect("fake transport lock should not be poisoned")
            .last()
            .expect("control request should be recorded")
            .clone()
    }
}

#[async_trait]
impl ExternalApplicationBackendTransport for FakeExternalBackendTransport {
    async fn start(
        &self,
        request: ExternalApplicationBackendStartRequest,
    ) -> Result<(), ServiceError> {
        self.starts
            .lock()
            .expect("fake transport lock should not be poisoned")
            .push(request);
        Ok(())
    }

    async fn control(
        &self,
        request: crate::ExternalApplicationBackendControlRequest,
    ) -> Result<(), ServiceError> {
        self.controls
            .lock()
            .expect("fake transport lock should not be poisoned")
            .push(request);
        Ok(())
    }
}
