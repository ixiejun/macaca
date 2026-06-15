//! Runtime-host alert service provider.
//!
//! The provider moves concrete alert delivery out of `macaca-kernel`. Kernel
//! code owns provider-neutral alert identity and deduplication only; this module
//! owns transport strategies such as tracing logs and webhook POSTs behind the
//! `SystemService` boundary. Calls are still admitted through `ServiceRuntime`,
//! so trace, policy, resource, entitlement, metering, and audit decorators run
//! before any transport side effect.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    Alert, AlertSeverity, CapabilityId, CleanupPolicy, KernelServiceId, ServiceCallResult,
    ServiceCapability, ServiceCommand, ServiceCommandName, ServiceDescriptor, ServiceError,
    ServiceHealth, ServiceLifecycleState, ServiceResult, ServiceScope, ServiceType, TraceContext,
    TraceSchemaRef,
};

use crate::service_provider::{
    ServiceProviderFactory, ServiceProviderFactoryContext, ServiceProviderInstance,
};
use crate::service_runtime_error::ServiceRuntimeError;

pub const ALERT_SERVICE_ID: &str = "service.alert";
const ALERT_COMMAND_RAISE: &str = "alert.raise";
const ALERT_COMMAND_RESOLVE: &str = "alert.resolve";
const ALERT_COMMAND_HEALTH: &str = "alert.health";
const ALERT_COMMAND_SNAPSHOT: &str = "alert.snapshot";

/// Transport Strategy used by the alert service provider.
///
/// Strategies are runtime-host owned so the kernel never depends on network
/// clients or provider-specific delivery mechanisms.
#[async_trait]
pub trait AlertDeliveryStrategy: Send + Sync {
    async fn deliver(&self, alert: &Alert, trace: &TraceContext) -> ServiceResult<()>;
    fn name(&self) -> &'static str;
}

/// Structured unavailable Strategy for hosts that disable alert delivery.
pub struct UnavailableAlertDelivery {
    reason: String,
}

impl UnavailableAlertDelivery {
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

#[async_trait]
impl AlertDeliveryStrategy for UnavailableAlertDelivery {
    async fn deliver(&self, _alert: &Alert, _trace: &TraceContext) -> ServiceResult<()> {
        Err(ServiceError::ServiceUnavailable(self.reason.clone()))
    }

    fn name(&self) -> &'static str {
        "unavailable"
    }
}

/// Local tracing Strategy that emits sanitized alert summaries.
pub struct TracingAlertDelivery;

#[async_trait]
impl AlertDeliveryStrategy for TracingAlertDelivery {
    async fn deliver(&self, alert: &Alert, trace: &TraceContext) -> ServiceResult<()> {
        match alert.severity {
            AlertSeverity::Critical => tracing::error!(
                service_id = ALERT_SERVICE_ID,
                trace_id = %trace.trace_id,
                severity = "critical",
                source = %alert.source,
                title = %alert.title,
                "alert service emitted critical alert"
            ),
            AlertSeverity::Warning => tracing::warn!(
                service_id = ALERT_SERVICE_ID,
                trace_id = %trace.trace_id,
                severity = "warning",
                source = %alert.source,
                title = %alert.title,
                "alert service emitted warning alert"
            ),
            AlertSeverity::Info => tracing::info!(
                service_id = ALERT_SERVICE_ID,
                trace_id = %trace.trace_id,
                severity = "info",
                source = %alert.source,
                title = %alert.title,
                "alert service emitted info alert"
            ),
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "tracing"
    }
}

/// Webhook Strategy owned by runtime-host.
///
/// The URL is never logged because it may contain routing secrets. Payloads are
/// bounded to the provider-neutral alert DTO and sent only after ServiceRuntime
/// decorators have admitted the command.
pub struct WebhookAlertDelivery {
    url: String,
    client: reqwest::Client,
}

impl WebhookAlertDelivery {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl AlertDeliveryStrategy for WebhookAlertDelivery {
    async fn deliver(&self, alert: &Alert, trace: &TraceContext) -> ServiceResult<()> {
        let payload = serde_json::json!({
            "severity": format!("{:?}", alert.severity),
            "title": alert.title,
            "message": alert.message,
            "source": alert.source,
            "trace_id": trace.trace_id,
        });
        self.client
            .post(&self.url)
            .json(&payload)
            .send()
            .await
            .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?;
        tracing::info!(
            service_id = ALERT_SERVICE_ID,
            trace_id = %trace.trace_id,
            delivery = self.name(),
            "alert service webhook delivery completed"
        );
        Ok(())
    }

    fn name(&self) -> &'static str {
        "webhook"
    }
}

/// SystemService implementation for alert delivery.
pub struct AlertSystemServiceProvider {
    descriptor: ServiceDescriptor,
    delivery: Arc<dyn AlertDeliveryStrategy>,
}

impl AlertSystemServiceProvider {
    pub fn tracing() -> Self {
        Self::new(Arc::new(TracingAlertDelivery))
    }

    pub fn webhook(url: impl Into<String>) -> Self {
        Self::new(Arc::new(WebhookAlertDelivery::new(url)))
    }

    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(Arc::new(UnavailableAlertDelivery::new(reason)))
    }

    pub fn new(delivery: Arc<dyn AlertDeliveryStrategy>) -> Self {
        Self {
            descriptor: alert_service_descriptor(),
            delivery,
        }
    }

    fn trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
        command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)
    }

    fn result(
        trace: TraceContext,
        status: impl Into<String>,
        output: serde_json::Value,
    ) -> ServiceResult<ServiceCallResult> {
        Ok(ServiceCallResult {
            output,
            trace,
            status: status.into(),
            metadata: BTreeMap::new(),
            cleanup_hint: Some(CleanupPolicy::None),
        })
    }

    async fn raise(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        let alert: Alert = serde_json::from_value(command.payload)
            .map_err(|error| ServiceError::InvalidArgument(error.to_string()))?;
        tracing::info!(
            service_id = %self.descriptor.id,
            command = ALERT_COMMAND_RAISE,
            trace_id = %trace.trace_id,
            delivery = self.delivery.name(),
            severity = ?alert.severity,
            source = %alert.source,
            "alert service delivery requested"
        );
        self.delivery.deliver(&alert, &trace).await?;
        Self::result(
            trace,
            "ok",
            serde_json::json!({
                "delivered": true,
                "delivery": self.delivery.name(),
            }),
        )
    }

    fn resolve(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        let alert: Alert = serde_json::from_value(command.payload)
            .map_err(|error| ServiceError::InvalidArgument(error.to_string()))?;
        tracing::debug!(
            service_id = %self.descriptor.id,
            command = ALERT_COMMAND_RESOLVE,
            trace_id = %trace.trace_id,
            delivery = self.delivery.name(),
            severity = ?alert.severity,
            source = %alert.source,
            "alert service resolved delivery strategy without side effects"
        );
        Self::result(
            trace,
            "ok",
            serde_json::json!({
                "service_id": ALERT_SERVICE_ID,
                "delivery": self.delivery.name(),
                "accepted": true,
                "side_effect": false,
            }),
        )
    }

    fn health_result(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        Self::result(
            trace,
            "ok",
            serde_json::json!({
                "service_id": ALERT_SERVICE_ID,
                "delivery": self.delivery.name(),
                "health": "healthy",
            }),
        )
    }

    fn snapshot(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        Self::result(
            trace,
            "ok",
            serde_json::json!({
                "service_id": ALERT_SERVICE_ID,
                "delivery": self.delivery.name(),
                "health": "healthy",
            }),
        )
    }
}

#[async_trait]
impl SystemService for AlertSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        tracing::info!(service_id = %self.descriptor.id, "alert service provider started");
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        match command.name.as_str() {
            ALERT_COMMAND_RAISE => self.raise(command).await,
            ALERT_COMMAND_RESOLVE => self.resolve(command),
            ALERT_COMMAND_HEALTH => self.health_result(command),
            ALERT_COMMAND_SNAPSHOT => self.snapshot(command),
            other => Err(ServiceError::UnsupportedCommand(other.into())),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        tracing::info!(service_id = %self.descriptor.id, "alert service provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(ServiceHealth::Healthy)
    }
}

/// Abstract Factory for alert service providers.
#[derive(Clone)]
pub struct AlertServiceProviderFactory {
    service: Arc<AlertSystemServiceProvider>,
}

impl AlertServiceProviderFactory {
    pub fn tracing() -> Self {
        Self {
            service: Arc::new(AlertSystemServiceProvider::tracing()),
        }
    }

    pub fn webhook(url: impl Into<String>) -> Self {
        Self {
            service: Arc::new(AlertSystemServiceProvider::webhook(url)),
        }
    }

    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            service: Arc::new(AlertSystemServiceProvider::unavailable(reason)),
        }
    }
}

#[async_trait]
impl ServiceProviderFactory for AlertServiceProviderFactory {
    async fn create(
        &self,
        _context: ServiceProviderFactoryContext,
    ) -> Result<ServiceProviderInstance, ServiceRuntimeError> {
        Ok(ServiceProviderInstance::new(
            self.service.descriptor(),
            self.service.clone(),
        ))
    }
}

pub fn alert_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(ALERT_SERVICE_ID),
        ServiceType::new("service.alert"),
        TraceSchemaRef::new("trace.service.alert.v1"),
    );
    descriptor.capabilities.push(ServiceCapability::new(
        CapabilityId::new("capability.alert.delivery"),
        "Deliver provider-neutral alert notifications through a runtime-host strategy.",
    ));
    descriptor.lifecycle_state = ServiceLifecycleState::Registered;
    descriptor.health = ServiceHealth::Unknown;
    descriptor.supported_scopes = vec![ServiceScope::Global];
    descriptor.cleanup_policy = CleanupPolicy::None;
    descriptor
}

pub fn raise_alert_command(alert: Alert, trace: TraceContext) -> ServiceResult<ServiceCommand> {
    let payload = serde_json::to_value(alert)
        .map_err(|error| ServiceError::InvalidArgument(error.to_string()))?;
    Ok(ServiceCommand::with_trace(
        ServiceCommandName::new(ALERT_COMMAND_RAISE),
        payload,
        trace,
    ))
}

pub fn resolve_alert_command(alert: Alert, trace: TraceContext) -> ServiceResult<ServiceCommand> {
    let payload = serde_json::to_value(alert)
        .map_err(|error| ServiceError::InvalidArgument(error.to_string()))?;
    Ok(ServiceCommand::with_trace(
        ServiceCommandName::new(ALERT_COMMAND_RESOLVE),
        payload,
        trace,
    ))
}

pub fn alert_health_command(trace: TraceContext) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(ALERT_COMMAND_HEALTH),
        serde_json::json!({}),
        trace,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::TraceContext;

    #[tokio::test]
    async fn tracing_provider_returns_structured_success() {
        let provider = AlertSystemServiceProvider::tracing();
        let trace = TraceContext::new("alert-trace");
        let command = raise_alert_command(Alert::warning("Budget", "bounded", "test"), trace)
            .expect("command");
        let result = provider.call(command).await.expect("alert result");
        assert_eq!(result.status, "ok");
        assert_eq!(result.output["delivered"], true);
        assert_eq!(result.output["delivery"], "tracing");
    }

    #[tokio::test]
    async fn unavailable_provider_returns_structured_unavailable() {
        let provider = AlertSystemServiceProvider::unavailable("alert delivery disabled");
        let trace = TraceContext::new("alert-unavailable");
        let command = raise_alert_command(Alert::critical("Alert", "blocked", "test"), trace)
            .expect("command");
        let error = provider.call(command).await.unwrap_err();
        assert!(matches!(error, ServiceError::ServiceUnavailable(_)));
    }

    #[tokio::test]
    async fn resolve_and_health_commands_are_side_effect_free() {
        let provider = AlertSystemServiceProvider::tracing();
        let resolve = resolve_alert_command(
            Alert::info("Probe", "bounded", "test"),
            TraceContext::new("alert-resolve"),
        )
        .expect("resolve command");
        let resolved = provider.call(resolve).await.expect("resolve result");
        assert_eq!(resolved.status, "ok");
        assert_eq!(resolved.output["side_effect"], false);

        let health = provider
            .call(alert_health_command(TraceContext::new("alert-health")))
            .await
            .expect("health result");
        assert_eq!(health.status, "ok");
        assert_eq!(health.output["health"], "healthy");
    }
}
