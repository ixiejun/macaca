use macaca_kernel::SystemService;
use macaca_proto::workbench::app_protocol::*;
use macaca_proto::{
    ApplicationId, BoundedSummary, ServiceCommand, ServiceHealth, TraceContext,
    WorkbenchCommandScope, WorkbenchCommandStatus,
};

use crate::{app_protocol_service_command, AppProtocolSystemServiceProvider};

fn scope() -> WorkbenchCommandScope {
    WorkbenchCommandScope::new(
        ApplicationId::from_name("generic-protocol-app"),
        "session-a",
    )
    .unwrap()
}

fn client() -> AppProtocolClientMetadata {
    AppProtocolClientMetadata {
        client_name: "generic-client".into(),
        client_version: Some("1.0.0".into()),
        transport: AppProtocolTransportKind::WebSocket,
        wants_notifications: true,
    }
}

fn command<T: serde::Serialize>(name: &str, payload: T, trace_id: &str) -> ServiceCommand {
    app_protocol_service_command(
        name,
        AppProtocolCommand::new(scope(), TraceContext::new(trace_id), trace_id, payload).unwrap(),
    )
    .unwrap()
}

async fn call<T: serde::de::DeserializeOwned>(
    provider: &AppProtocolSystemServiceProvider,
    command: ServiceCommand,
) -> T {
    let result = provider.call(command).await.unwrap();
    serde_json::from_value(result.output).unwrap()
}

async fn initialize(provider: &AppProtocolSystemServiceProvider) -> AppProtocolConnectionRecord {
    let result: AppProtocolCommandResult = call(
        provider,
        command(
            INITIALIZE_COMMAND,
            AppProtocolInitializeRequest {
                connection_id: Some("connection-a".into()),
                protocol_version: "app-protocol.v1".into(),
                client: client(),
                requested_capabilities: vec![
                    "capability.interaction_ledger".into(),
                    "capability.app_protocol".into(),
                ],
            },
            "trace-initialize",
        ),
    )
    .await;

    match result.output.unwrap() {
        AppProtocolResponse::Initialized(record) => record,
        _ => panic!("expected initialized response"),
    }
}

#[tokio::test]
async fn initialization_gates_other_protocol_requests() {
    let provider = AppProtocolSystemServiceProvider::new();
    provider.start().await.unwrap();

    let error = provider
        .call(command(
            SUBSCRIPTION_CREATE_COMMAND,
            AppProtocolSubscriptionCreateRequest {
                connection_id: "connection-a".into(),
                target: AppProtocolSubscriptionTarget::Global,
                event_kinds: Vec::new(),
            },
            "trace-before-init",
        ))
        .await
        .unwrap_err();

    assert!(error.to_string().contains("not initialized"));
}

#[tokio::test]
async fn duplicate_initialization_returns_retryable_overload_state() {
    let provider = AppProtocolSystemServiceProvider::new();
    initialize(&provider).await;

    let duplicate: AppProtocolCommandResult = call(
        &provider,
        command(
            INITIALIZE_COMMAND,
            AppProtocolInitializeRequest {
                connection_id: Some("connection-a".into()),
                protocol_version: "app-protocol.v1".into(),
                client: client(),
                requested_capabilities: Vec::new(),
            },
            "trace-duplicate-init",
        ),
    )
    .await;

    assert!(matches!(
        duplicate.output.unwrap(),
        AppProtocolResponse::Overloaded {
            reason: AppProtocolRetryableReason::DuplicateInitialization,
            ..
        }
    ));
}

#[tokio::test]
async fn subscription_close_only_changes_gateway_subscription_state() {
    let provider = AppProtocolSystemServiceProvider::new();
    let connection = initialize(&provider).await;

    let created: AppProtocolCommandResult = call(
        &provider,
        command(
            SUBSCRIPTION_CREATE_COMMAND,
            AppProtocolSubscriptionCreateRequest {
                connection_id: connection.connection_id.clone(),
                target: AppProtocolSubscriptionTarget::Thread {
                    thread_id: "thread-a".into(),
                },
                event_kinds: vec!["interaction.item.appended".into()],
            },
            "trace-subscribe",
        ),
    )
    .await;
    let subscription = match created.output.unwrap() {
        AppProtocolResponse::Subscription(record) => record,
        _ => panic!("expected subscription"),
    };

    let closed: AppProtocolCommandResult = call(
        &provider,
        command(
            SUBSCRIPTION_CLOSE_COMMAND,
            AppProtocolSubscriptionCloseRequest {
                connection_id: connection.connection_id,
                subscription_id: subscription.subscription_id,
                reason: Some("client closed".into()),
            },
            "trace-close-subscription",
        ),
    )
    .await;

    assert!(matches!(
        closed.output.unwrap(),
        AppProtocolResponse::SubscriptionClosed(record)
            if record.status == AppProtocolSubscriptionStatus::Closed
    ));
}

#[tokio::test]
async fn notification_routing_preserves_trace_and_bounded_payload() {
    let provider = AppProtocolSystemServiceProvider::new();
    let mut notifications = provider.subscribe_notifications();
    let connection = initialize(&provider).await;
    let created: AppProtocolCommandResult = call(
        &provider,
        command(
            SUBSCRIPTION_CREATE_COMMAND,
            AppProtocolSubscriptionCreateRequest {
                connection_id: connection.connection_id,
                target: AppProtocolSubscriptionTarget::Thread {
                    thread_id: "thread-a".into(),
                },
                event_kinds: vec!["interaction.item.appended".into()],
            },
            "trace-subscribe-for-notification",
        ),
    )
    .await;
    assert!(matches!(
        created.output.unwrap(),
        AppProtocolResponse::Subscription(_)
    ));

    let emitted: AppProtocolCommandResult = call(
        &provider,
        command(
            NOTIFICATION_EMIT_COMMAND,
            AppProtocolNotificationEmitRequest {
                event: AppProtocolEventEnvelope {
                    event_kind: "interaction.item.appended".into(),
                    source_service_id: "service.interaction".into(),
                    thread_id: Some("thread-a".into()),
                    turn_id: Some("turn-a".into()),
                    item_id: Some("item-a".into()),
                    trace_id: "trace-item".into(),
                    audit_ref: Some("audit:item".into()),
                    summary: Some(BoundedSummary {
                        text: "[artifact-backed interaction item]".into(),
                        truncated: true,
                        original_byte_len: Some(16384),
                        artifact_ref: None,
                    }),
                    emitted_at: chrono::Utc::now(),
                },
            },
            "trace-emit",
        ),
    )
    .await;

    let count = match emitted.output.unwrap() {
        AppProtocolResponse::Notifications(items) => items.len(),
        _ => panic!("expected notifications"),
    };
    assert_eq!(count, 1);
    assert_eq!(
        notifications.recv().await.unwrap().event.trace_id,
        "trace-item"
    );
}

#[tokio::test]
async fn json_rpc_health_frame_is_decoded_and_encoded() {
    let provider = AppProtocolSystemServiceProvider::new();
    initialize(&provider).await;

    let result: AppProtocolCommandResult = call(
        &provider,
        command(
            JSON_RPC_RECEIVE_COMMAND,
            AppProtocolJsonRpcReceiveRequest {
                connection_id: "connection-a".into(),
                frame: serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "app_protocol.health"
                })
                .to_string(),
            },
            "trace-json-rpc",
        ),
    )
    .await;

    assert!(matches!(
        result.output.unwrap(),
        AppProtocolResponse::JsonRpc(frame) if frame.error.is_none() && frame.result.is_some()
    ));
}

#[tokio::test]
async fn subscription_limit_returns_retryable_backpressure_state() {
    let provider = AppProtocolSystemServiceProvider::with_limits(8, 1, 4);
    initialize(&provider).await;

    let first: AppProtocolCommandResult = call(
        &provider,
        command(
            SUBSCRIPTION_CREATE_COMMAND,
            AppProtocolSubscriptionCreateRequest {
                connection_id: "connection-a".into(),
                target: AppProtocolSubscriptionTarget::Global,
                event_kinds: Vec::new(),
            },
            "trace-first-subscription",
        ),
    )
    .await;
    assert!(matches!(
        first.output.unwrap(),
        AppProtocolResponse::Subscription(_)
    ));

    let overloaded: AppProtocolCommandResult = call(
        &provider,
        command(
            SUBSCRIPTION_CREATE_COMMAND,
            AppProtocolSubscriptionCreateRequest {
                connection_id: "connection-a".into(),
                target: AppProtocolSubscriptionTarget::Global,
                event_kinds: Vec::new(),
            },
            "trace-overload-subscription",
        ),
    )
    .await;

    assert!(matches!(
        overloaded.status,
        WorkbenchCommandStatus::Failed { .. }
    ));
    assert!(matches!(
        overloaded.output.unwrap(),
        AppProtocolResponse::Overloaded {
            reason: AppProtocolRetryableReason::OutboundQueueSaturated,
            ..
        }
    ));
}

#[tokio::test]
async fn unavailable_provider_reports_structured_state() {
    let provider = AppProtocolSystemServiceProvider::unavailable("gateway disabled by host");
    assert!(matches!(
        provider.health().await.unwrap(),
        ServiceHealth::Unavailable { .. }
    ));

    let result: AppProtocolCommandResult = call(
        &provider,
        command(
            HEALTH_READ_COMMAND,
            AppProtocolHealthReadRequest {
                connection_id: None,
            },
            "trace-unavailable-provider",
        ),
    )
    .await;

    assert!(matches!(
        result.status,
        WorkbenchCommandStatus::Unavailable { .. }
    ));
}

#[tokio::test]
async fn unsupported_local_transports_return_unavailable_diagnostics() {
    let provider = AppProtocolSystemServiceProvider::new();
    let result: AppProtocolCommandResult = call(
        &provider,
        command(
            INITIALIZE_COMMAND,
            AppProtocolInitializeRequest {
                connection_id: Some("stdio-connection".into()),
                protocol_version: "app-protocol.v1".into(),
                client: AppProtocolClientMetadata {
                    transport: AppProtocolTransportKind::Stdio,
                    ..client()
                },
                requested_capabilities: Vec::new(),
            },
            "trace-stdio-unavailable",
        ),
    )
    .await;

    assert!(matches!(
        result.status,
        WorkbenchCommandStatus::Unavailable { .. }
    ));
    assert!(matches!(
        result.output.unwrap(),
        AppProtocolResponse::TransportUnavailable {
            transport: AppProtocolTransportKind::Stdio,
            ..
        }
    ));
}
