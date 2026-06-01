use macaca_proto::{
    ApplicationExecutionControlKind, ApplicationExecutionHeartbeatPolicy,
    ApplicationExecutionProviderKind, CapabilityId, ExternalApplicationBackendExecutionProfile,
};

use crate::{ApplicationExecutionProvider, ExternalApplicationBackendProvider};

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
