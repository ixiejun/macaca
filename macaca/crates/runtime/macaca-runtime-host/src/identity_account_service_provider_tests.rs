use std::sync::Arc;

use macaca_kernel::SystemService;
use macaca_proto::domain_pack_contract::identity_account::IDENTITY_ACCOUNT_COMMANDS;
use macaca_proto::{
    ServiceBusSource, ServiceCommand, ServiceCommandName, ServiceError, ServiceHealth, TraceContext,
};

use super::identity_account_service_provider::{
    IdentityAccountRuntimeEventKind, IdentityAccountSystemServiceProvider,
};
use crate::{
    InMemoryServiceRuntimeEventSink, ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig,
    StaticServiceProviderFactory,
};

#[tokio::test]
async fn identity_account_provider_dispatches_every_contract_command_without_payload_echo() {
    let provider = IdentityAccountSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    for command in IDENTITY_ACCOUNT_COMMANDS {
        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(*command),
                serde_json::json!({"password": "secret-marker", "provider_payload": "raw-marker"}),
                TraceContext::new(format!("trace-{command}")),
            ))
            .await
            .unwrap();
        assert!(!result.output.to_string().contains("marker"));
    }
    let observed = events.try_recv().unwrap();
    assert_eq!(
        observed.kind,
        IdentityAccountRuntimeEventKind::AdmissionValidated
    );
    assert!(!format!("{observed:?}").contains("marker"));
    assert_eq!(
        provider.capability().feature_flags.len(),
        IDENTITY_ACCOUNT_COMMANDS.len()
    );
}

#[tokio::test]
async fn identity_account_provider_is_explicitly_unavailable_or_unsupported() {
    let provider = IdentityAccountSystemServiceProvider::mock();
    assert!(matches!(
        provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new("account.unknown"),
                serde_json::json!({}),
                TraceContext::new("unknown")
            ))
            .await,
        Err(ServiceError::UnsupportedCommand(_))
    ));
    let unavailable = IdentityAccountSystemServiceProvider::unavailable("not_installed");
    assert!(matches!(
        unavailable.health().await.unwrap(),
        ServiceHealth::Unavailable { .. }
    ));
    assert!(matches!(
        unavailable
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new("account.read_account"),
                serde_json::json!({}),
                TraceContext::new("unavailable")
            ))
            .await,
        Err(ServiceError::ServiceUnavailable(_))
    ));
}

#[tokio::test]
async fn identity_account_provider_snapshot_is_bounded_and_cleanup_releases_references() {
    let provider = IdentityAccountSystemServiceProvider::mock();
    for id in ["one", "two"] {
        provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new("account.read_account"),
                serde_json::json!({"payload": "x".repeat(10_000)}),
                TraceContext::new(id),
            ))
            .await
            .unwrap();
    }
    assert_eq!(provider.snapshot().await["active_reference_count"], "2");
    provider.cleanup().await.unwrap();
    assert_eq!(provider.snapshot().await["active_reference_count"], "0");
}

#[tokio::test]
async fn identity_account_commands_replay_through_the_canonical_service_runtime() {
    let events = Arc::new(InMemoryServiceRuntimeEventSink::new());
    let runtime = ServiceRuntime::new(ServiceRuntimeConfig {
        event_sink: Some(events.clone()),
        ..Default::default()
    });
    let provider = Arc::new(IdentityAccountSystemServiceProvider::mock());
    let service_id = runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(
                provider.descriptor(),
                provider,
            )),
            Default::default(),
        )
        .await
        .unwrap();
    runtime
        .start(&service_id, TraceContext::new("account-runtime-start"))
        .await
        .unwrap();

    for command in IDENTITY_ACCOUNT_COMMANDS {
        let trace_id = format!("account-runtime-{command}");
        let reply = runtime
            .call(
                &service_id,
                ServiceBusSource::new("identity-account-conformance"),
                ServiceCommand::with_trace(
                    ServiceCommandName::new(*command),
                    serde_json::json!({"password": "secret-marker", "provider_payload": "raw-marker"}),
                    TraceContext::new(trace_id.clone()),
                ),
            )
            .await
            .unwrap();
        assert_eq!(reply.status, "ok");
        assert!(!reply.output.unwrap().to_string().contains("marker"));
        assert!(events.events().unwrap().iter().any(|event| {
            event.trace_id.as_deref() == Some(trace_id.as_str())
                && event.operation == "service_runtime.call.completed"
        }));
    }
    let observable = format!("{:?}", events.events().unwrap());
    assert!(!observable.contains("secret-marker"));
    assert!(!observable.contains("raw-marker"));
}
