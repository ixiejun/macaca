//! Runtime-host provider for `service.app_protocol`.
//!
//! This provider is the host-owned Adapter between bidirectional client
//! transports and Macaca focused service clients.  It owns connection and
//! subscription state, JSON-RPC framing, bounded queue diagnostics, and
//! notification fan-out.  It intentionally does not execute interaction,
//! filesystem, process, approval, tool, plugin, MCP, skill, Git, review, or
//! diagnostics semantics; requests for those capabilities are surfaced as
//! explicit `Forwarded` protocol responses that the shell/facade layer can
//! dispatch to the owning focused client.

use std::collections::BTreeMap;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::workbench::app_protocol::*;
use macaca_proto::{
    CleanupPolicy, ServiceCallResult, ServiceCommand, ServiceDescriptor, ServiceError,
    ServiceHealth, ServiceResult, TraceContext, WorkbenchCommandResult, WorkbenchCommandStatus,
};
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

pub(crate) const DEFAULT_CONNECTION_LIMIT: usize = 256;
pub(crate) const DEFAULT_SUBSCRIPTION_LIMIT: usize = 4096;
pub(crate) const RETRY_AFTER_MS: u64 = 250;

/// Provider-owned state for the bidirectional protocol gateway.
pub struct AppProtocolSystemServiceProvider {
    descriptor: ServiceDescriptor,
    pub(crate) connections: RwLock<BTreeMap<String, AppProtocolConnectionRecord>>,
    pub(crate) subscriptions: RwLock<BTreeMap<String, AppProtocolSubscriptionRecord>>,
    pub(crate) notification_tx: broadcast::Sender<AppProtocolNotification>,
    pub(crate) connection_limit: usize,
    pub(crate) subscription_limit: usize,
    unavailable_reason: Option<String>,
}

impl AppProtocolSystemServiceProvider {
    /// Create a provider with production defaults and bounded notification
    /// buffering.  Transport listeners are installed by composition roots; this
    /// provider only owns the service contract implementation.
    pub fn new() -> Self {
        Self::with_limits(DEFAULT_CONNECTION_LIMIT, DEFAULT_SUBSCRIPTION_LIMIT, 1024)
    }

    /// Create a provider with explicit limits for overload tests.
    pub fn with_limits(
        connection_limit: usize,
        subscription_limit: usize,
        outbound_queue_limit: usize,
    ) -> Self {
        let (notification_tx, _) = broadcast::channel(outbound_queue_limit.max(1));
        Self {
            descriptor: descriptor().base,
            connections: RwLock::new(BTreeMap::new()),
            subscriptions: RwLock::new(BTreeMap::new()),
            notification_tx,
            connection_limit: connection_limit.max(1),
            subscription_limit: subscription_limit.max(1),
            unavailable_reason: None,
        }
    }

    /// Create a Null Object provider for hosts that deliberately do not expose
    /// a bidirectional app protocol gateway.  Calls return structured
    /// unavailable data instead of pretending a transport exists.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        let mut provider = Self::with_limits(1, 1, 1);
        provider.unavailable_reason = Some(reason.into());
        provider
    }

    /// Subscribe to protocol notifications after they have been redacted and
    /// matched to active subscriptions.
    pub fn subscribe_notifications(&self) -> broadcast::Receiver<AppProtocolNotification> {
        self.notification_tx.subscribe()
    }

    pub(crate) fn trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
        command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)
    }

    pub(crate) fn result(
        output: AppProtocolResponse,
        trace: TraceContext,
        status: WorkbenchCommandStatus,
    ) -> ServiceResult<ServiceCallResult> {
        let result = WorkbenchCommandResult {
            trace: trace.clone(),
            status,
            output: Some(output),
            summary: None,
            audit_ref: Some(format!("app-protocol:{}", trace.trace_id)),
            completed_at: chrono::Utc::now(),
        };
        Ok(ServiceCallResult {
            output: serde_json::to_value(result)
                .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?,
            trace,
            status: "ok".into(),
            metadata: BTreeMap::new(),
            cleanup_hint: Some(CleanupPolicy::None),
        })
    }

    pub(crate) fn overload(
        trace: TraceContext,
        reason: AppProtocolRetryableReason,
    ) -> ServiceResult<ServiceCallResult> {
        warn!(
            trace_id = %trace.trace_id,
            reason = ?reason,
            retry_after_ms = RETRY_AFTER_MS,
            "app protocol gateway rejecting retryable overload"
        );
        Self::result(
            AppProtocolResponse::Overloaded {
                reason,
                retry_after_ms: RETRY_AFTER_MS,
            },
            trace,
            WorkbenchCommandStatus::Failed {
                reason: "app protocol gateway is overloaded".into(),
            },
        )
    }
}

impl Default for AppProtocolSystemServiceProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SystemService for AppProtocolSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.descriptor.id,
            connection_limit = self.connection_limit,
            subscription_limit = self.subscription_limit,
            "app protocol service provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        if let Some(reason) = &self.unavailable_reason {
            warn!(
                service_id = %self.descriptor.id,
                command = %command.name,
                trace_id = %trace.trace_id,
                reason = %reason,
                "app protocol unavailable provider rejected command"
            );
            return Self::result(
                AppProtocolResponse::Health(self.snapshot(true).await),
                trace,
                WorkbenchCommandStatus::Unavailable {
                    reason: reason.clone(),
                },
            );
        }
        info!(
            service_id = %self.descriptor.id,
            command = %command.name,
            trace_id = %trace.trace_id,
            "app protocol service command accepted"
        );
        match command.name.as_str() {
            INITIALIZE_COMMAND => self.initialize(command.payload).await,
            SUBSCRIPTION_CREATE_COMMAND => self.create_subscription(command.payload).await,
            SUBSCRIPTION_CLOSE_COMMAND => self.close_subscription(command.payload).await,
            NOTIFICATION_EMIT_COMMAND => self.emit_notification(command.payload).await,
            HEALTH_READ_COMMAND => self.health_read(command.payload).await,
            JSON_RPC_RECEIVE_COMMAND => self.receive_json_rpc(command.payload).await,
            SNAPSHOT_COMMAND => self.snapshot_command(command.payload).await,
            other => Err(ServiceError::UnsupportedCommand(other.into())),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "app protocol service provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "app protocol service provider cleanup completed");
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        if let Some(reason) = &self.unavailable_reason {
            Ok(ServiceHealth::Unavailable {
                reason: reason.clone(),
            })
        } else {
            Ok(ServiceHealth::Healthy)
        }
    }
}
