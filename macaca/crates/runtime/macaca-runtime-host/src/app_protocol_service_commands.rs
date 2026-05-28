//! Command handlers and framing helpers for `service.app_protocol`.
//!
//! Keeping command handling outside the provider type keeps the runtime-host
//! lifecycle adapter small.  These handlers are still part of the same service
//! boundary: they mutate only protocol connection/subscription state and
//! translate events into protocol notifications.

use macaca_proto::workbench::app_protocol::*;
use macaca_proto::{ServiceCallResult, ServiceCommand, ServiceError, ServiceResult, TraceContext};
use serde::de::DeserializeOwned;
use serde_json::Value;
use tracing::{info, warn};
use uuid::Uuid;

use crate::app_protocol_service_provider::AppProtocolSystemServiceProvider;

impl AppProtocolSystemServiceProvider {
    pub(crate) async fn initialize(&self, payload: Value) -> ServiceResult<ServiceCallResult> {
        let typed: AppProtocolCommand<AppProtocolInitializeRequest> = decode(payload)?;
        let trace = typed.trace.clone();
        if matches!(
            typed.payload.client.transport,
            AppProtocolTransportKind::Stdio | AppProtocolTransportKind::UnixSocket
        ) {
            let reason =
                "stdio and unix-socket app protocol transports are not installed in this runtime"
                    .to_string();
            warn!(
                trace_id = %trace.trace_id,
                transport = ?typed.payload.client.transport,
                "app protocol transport unavailable"
            );
            return Self::result(
                AppProtocolResponse::TransportUnavailable {
                    transport: typed.payload.client.transport,
                    reason: reason.clone(),
                },
                trace,
                macaca_proto::WorkbenchCommandStatus::Unavailable { reason },
            );
        }
        let connection_id = typed
            .payload
            .connection_id
            .clone()
            .unwrap_or_else(|| format!("app-protocol-{}", Uuid::new_v4()));
        let mut connections = self.connections.write().await;
        if connections.contains_key(&connection_id) {
            warn!(
                connection_id = %connection_id,
                trace_id = %trace.trace_id,
                "app protocol duplicate initialization rejected"
            );
            return Self::overload(trace, AppProtocolRetryableReason::DuplicateInitialization);
        }
        if connections
            .values()
            .filter(|record| record.status == AppProtocolConnectionStatus::Initialized)
            .count()
            >= self.connection_limit
        {
            return Self::overload(trace, AppProtocolRetryableReason::IngressQueueSaturated);
        }

        let now = chrono::Utc::now();
        let negotiated_capabilities = typed
            .payload
            .requested_capabilities
            .iter()
            .filter(|capability| !capability.trim().is_empty())
            .cloned()
            .collect();
        let record = AppProtocolConnectionRecord {
            connection_id: connection_id.clone(),
            scope: typed.scope,
            protocol_version: typed.payload.protocol_version,
            client: typed.payload.client,
            negotiated_capabilities,
            status: AppProtocolConnectionStatus::Initialized,
            created_at: now,
            updated_at: now,
        };
        connections.insert(connection_id.clone(), record.clone());
        info!(
            connection_id = %connection_id,
            trace_id = %trace.trace_id,
            wants_notifications = record.client.wants_notifications,
            "app protocol connection initialized"
        );
        Self::result(
            AppProtocolResponse::Initialized(record),
            trace,
            macaca_proto::WorkbenchCommandStatus::Completed,
        )
    }

    pub(crate) async fn create_subscription(
        &self,
        payload: Value,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: AppProtocolCommand<AppProtocolSubscriptionCreateRequest> = decode(payload)?;
        let trace = typed.trace.clone();
        self.ensure_initialized(&typed.payload.connection_id, &trace)
            .await?;
        let mut subscriptions = self.subscriptions.write().await;
        if subscriptions
            .values()
            .filter(|record| record.status == AppProtocolSubscriptionStatus::Active)
            .count()
            >= self.subscription_limit
        {
            return Self::overload(trace, AppProtocolRetryableReason::OutboundQueueSaturated);
        }

        let record = AppProtocolSubscriptionRecord {
            subscription_id: format!("subscription-{}", Uuid::new_v4()),
            connection_id: typed.payload.connection_id,
            target: typed.payload.target,
            event_kinds: typed.payload.event_kinds,
            status: AppProtocolSubscriptionStatus::Active,
            created_at: chrono::Utc::now(),
            closed_at: None,
        };
        subscriptions.insert(record.subscription_id.clone(), record.clone());
        info!(
            subscription_id = %record.subscription_id,
            connection_id = %record.connection_id,
            trace_id = %trace.trace_id,
            "app protocol subscription created"
        );
        Self::result(
            AppProtocolResponse::Subscription(record),
            trace,
            macaca_proto::WorkbenchCommandStatus::Completed,
        )
    }

    pub(crate) async fn close_subscription(
        &self,
        payload: Value,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: AppProtocolCommand<AppProtocolSubscriptionCloseRequest> = decode(payload)?;
        let trace = typed.trace.clone();
        self.ensure_initialized(&typed.payload.connection_id, &trace)
            .await?;
        let mut subscriptions = self.subscriptions.write().await;
        let record = subscriptions
            .get_mut(&typed.payload.subscription_id)
            .ok_or_else(|| {
                ServiceError::InvalidArgument(format!(
                    "unknown app protocol subscription '{}'",
                    typed.payload.subscription_id
                ))
            })?;
        if record.connection_id != typed.payload.connection_id {
            return Err(ServiceError::InvalidArgument(
                "subscription does not belong to connection".into(),
            ));
        }
        record.status = AppProtocolSubscriptionStatus::Closed;
        record.closed_at = Some(chrono::Utc::now());
        let record = record.clone();
        info!(
            subscription_id = %record.subscription_id,
            connection_id = %record.connection_id,
            trace_id = %trace.trace_id,
            "app protocol subscription closed"
        );
        Self::result(
            AppProtocolResponse::SubscriptionClosed(record),
            trace,
            macaca_proto::WorkbenchCommandStatus::Completed,
        )
    }

    pub(crate) async fn emit_notification(
        &self,
        payload: Value,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: AppProtocolCommand<AppProtocolNotificationEmitRequest> = decode(payload)?;
        let trace = typed.trace.clone();
        let subscriptions = self.subscriptions.read().await;
        let notifications = subscriptions
            .values()
            .filter(|subscription| self.subscription_matches(subscription, &typed.payload.event))
            .map(|subscription| AppProtocolNotification {
                notification_id: format!("notification-{}", Uuid::new_v4()),
                subscription_id: subscription.subscription_id.clone(),
                connection_id: subscription.connection_id.clone(),
                event: typed.payload.event.clone(),
            })
            .collect::<Vec<_>>();

        for notification in &notifications {
            let _ = self.notification_tx.send(notification.clone());
        }
        info!(
            notification_count = notifications.len(),
            trace_id = %trace.trace_id,
            "app protocol notifications emitted"
        );
        Self::result(
            AppProtocolResponse::Notifications(notifications),
            trace,
            macaca_proto::WorkbenchCommandStatus::Completed,
        )
    }

    pub(crate) async fn health_read(&self, payload: Value) -> ServiceResult<ServiceCallResult> {
        let typed: AppProtocolCommand<AppProtocolHealthReadRequest> = decode(payload)?;
        let trace = typed.trace.clone();
        if let Some(connection_id) = typed.payload.connection_id {
            self.ensure_initialized(&connection_id, &trace).await?;
        }
        let snapshot = self.snapshot(false).await;
        info!(trace_id = %trace.trace_id, "app protocol health read completed");
        Self::result(
            AppProtocolResponse::Health(snapshot),
            trace,
            macaca_proto::WorkbenchCommandStatus::Completed,
        )
    }

    pub(crate) async fn snapshot_command(
        &self,
        payload: Value,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: AppProtocolCommand<AppProtocolSnapshotRequest> = decode(payload)?;
        let trace = typed.trace.clone();
        let snapshot = self.snapshot(typed.payload.include_closed).await;
        Self::result(
            AppProtocolResponse::Snapshot(snapshot),
            trace,
            macaca_proto::WorkbenchCommandStatus::Completed,
        )
    }

    pub(crate) async fn receive_json_rpc(
        &self,
        payload: Value,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: AppProtocolCommand<AppProtocolJsonRpcReceiveRequest> = decode(payload)?;
        let trace = typed.trace.clone();
        self.ensure_initialized(&typed.payload.connection_id, &trace)
            .await?;
        let frame = parse_json_rpc_frame(&typed.payload.frame)?;
        info!(
            connection_id = %typed.payload.connection_id,
            method = %frame.method,
            trace_id = %trace.trace_id,
            "app protocol json-rpc frame received"
        );
        let response = match frame.method.as_str() {
            "app_protocol.health" => AppProtocolJsonRpcResponseFrame {
                jsonrpc: "2.0".into(),
                id: frame.id,
                result: Some(
                    serde_json::to_value(self.snapshot(false).await)
                        .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?,
                ),
                error: None,
            },
            method if method.starts_with("service.") => AppProtocolJsonRpcResponseFrame {
                jsonrpc: "2.0".into(),
                id: frame.id,
                result: Some(serde_json::json!({
                    "forwarded": true,
                    "service_id": method,
                    "params": frame.params,
                })),
                error: None,
            },
            _ => AppProtocolJsonRpcResponseFrame {
                jsonrpc: "2.0".into(),
                id: frame.id,
                result: None,
                error: Some(AppProtocolJsonRpcError {
                    code: -32601,
                    message: "unsupported app protocol method".into(),
                    retryable: false,
                    reason: None,
                }),
            },
        };
        Self::result(
            AppProtocolResponse::JsonRpc(response),
            trace,
            macaca_proto::WorkbenchCommandStatus::Completed,
        )
    }

    async fn ensure_initialized(
        &self,
        connection_id: &str,
        trace: &TraceContext,
    ) -> ServiceResult<()> {
        let connections = self.connections.read().await;
        let initialized = connections
            .get(connection_id)
            .map(|record| record.status == AppProtocolConnectionStatus::Initialized)
            .unwrap_or(false);
        if initialized {
            Ok(())
        } else {
            warn!(
                connection_id = %connection_id,
                trace_id = %trace.trace_id,
                "app protocol request rejected before initialization"
            );
            Err(ServiceError::InvalidArgument(format!(
                "connection '{connection_id}' is not initialized"
            )))
        }
    }

    pub(crate) async fn snapshot(&self, include_closed: bool) -> AppProtocolServiceSnapshot {
        let connections = self.connections.read().await;
        let subscriptions = self.subscriptions.read().await;
        let active_connections = connections
            .values()
            .filter(|record| record.status == AppProtocolConnectionStatus::Initialized)
            .count();
        let active_subscriptions = subscriptions
            .values()
            .filter(|record| record.status == AppProtocolSubscriptionStatus::Active)
            .count();
        let closed_connections = if include_closed {
            connections.len().saturating_sub(active_connections)
        } else {
            0
        };
        let closed_subscriptions = if include_closed {
            subscriptions.len().saturating_sub(active_subscriptions)
        } else {
            0
        };
        AppProtocolServiceSnapshot {
            active_connections,
            active_subscriptions,
            closed_connections,
            closed_subscriptions,
            ingress_queue_limit: self.connection_limit,
            outbound_queue_limit: self.subscription_limit,
            captured_at: chrono::Utc::now(),
        }
    }

    fn subscription_matches(
        &self,
        subscription: &AppProtocolSubscriptionRecord,
        event: &AppProtocolEventEnvelope,
    ) -> bool {
        if subscription.status != AppProtocolSubscriptionStatus::Active {
            return false;
        }
        if !subscription.event_kinds.is_empty()
            && !subscription
                .event_kinds
                .iter()
                .any(|kind| kind == &event.event_kind)
        {
            return false;
        }
        match &subscription.target {
            AppProtocolSubscriptionTarget::Global => true,
            AppProtocolSubscriptionTarget::Service { service_id } => {
                service_id == &event.source_service_id
            }
            AppProtocolSubscriptionTarget::Thread { thread_id } => {
                event.thread_id.as_ref() == Some(thread_id)
            }
            AppProtocolSubscriptionTarget::Application { application_id } => {
                // Application-level subscriptions are matched from sanitized
                // owner metadata only. The gateway must not infer ownership
                // from thread ids, paths, payload text, or product workflow.
                event.audit_ref.as_ref() == Some(application_id)
            }
        }
    }
}

fn decode<T>(value: Value) -> ServiceResult<T>
where
    T: DeserializeOwned,
{
    serde_json::from_value(value)
        .map_err(|error| ServiceError::UnsupportedCommand(error.to_string()))
}

fn parse_json_rpc_frame(frame: &str) -> ServiceResult<AppProtocolJsonRpcFrame> {
    let parsed: AppProtocolJsonRpcFrame = serde_json::from_str(frame)
        .map_err(|error| ServiceError::InvalidArgument(error.to_string()))?;
    if parsed.jsonrpc != "2.0" || parsed.method.trim().is_empty() {
        return Err(ServiceError::InvalidArgument(
            "invalid JSON-RPC 2.0 app protocol frame".into(),
        ));
    }
    Ok(parsed)
}

/// Build a trace-scoped app protocol service command for tests and adapters.
pub fn app_protocol_service_command<T: serde::Serialize>(
    command_name: &str,
    command: AppProtocolCommand<T>,
) -> ServiceResult<ServiceCommand> {
    service_command(command_name, command)
        .map_err(|error| ServiceError::AdapterFailure(error.to_string()))
}
