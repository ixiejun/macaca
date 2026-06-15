//! SDK Alert Service client.
//!
//! Alert delivery is an operating-system capability, not presentation-shell
//! logic.  This client gives shells and applications a focused Facade over the
//! generic `SystemServiceClient` so callers can raise provider-neutral alerts
//! without constructing kernel alert managers, runtime-host providers, webhook
//! transports, or log sinks directly.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{Alert, MacacaError, MacacaResult, TraceContext};
use tracing::{info, warn};

use crate::service_client::{ServiceCallCommand, SystemServiceClient};

/// Stable service id for the generic alert delivery service.
///
/// This is an OS capability identifier rather than a provider or application
/// name.  Runtime-host owns the concrete service descriptor and transport
/// Strategy, while the SDK only targets the service contract.
pub const ALERT_SERVICE_ID: &str = "service.alert";
pub const ALERT_RAISE_COMMAND: &str = "alert.raise";
pub const ALERT_RESOLVE_COMMAND: &str = "alert.resolve";
pub const ALERT_HEALTH_COMMAND: &str = "alert.health";
pub const ALERT_SNAPSHOT_COMMAND: &str = "alert.snapshot";

/// Focused Alert client consumed by shells and upper SDK users.
#[async_trait]
pub trait SystemAlertClient: Send + Sync {
    async fn raise(&self, alert: Alert, trace: TraceContext) -> MacacaResult<()>;
}

/// Null-object Alert client used when alert service dispatch is unavailable.
#[derive(Debug, Clone, Default)]
pub struct UnavailableSystemAlertClient;

#[async_trait]
impl SystemAlertClient for UnavailableSystemAlertClient {
    async fn raise(&self, alert: Alert, trace: TraceContext) -> MacacaResult<()> {
        warn!(
            trace_id = %trace.trace_id,
            severity = ?alert.severity,
            source = %alert.source,
            "sdk alert client unavailable for alert raise"
        );
        Err(MacacaError::Config("Alert service is unavailable".into()))
    }
}

/// Runtime-backed Alert client implemented over the generic service-call bus.
#[derive(Clone)]
pub struct ServiceBackedAlertClient {
    service: Arc<dyn SystemServiceClient>,
}

impl ServiceBackedAlertClient {
    /// Create a service-backed client from a generic service dispatcher.
    pub fn new(service: Arc<dyn SystemServiceClient>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl SystemAlertClient for ServiceBackedAlertClient {
    async fn raise(&self, alert: Alert, trace: TraceContext) -> MacacaResult<()> {
        let payload = serde_json::to_value(&alert)?;
        info!(
            trace_id = %trace.trace_id,
            severity = ?alert.severity,
            source = %alert.source,
            "sdk alert client dispatching alert raise"
        );
        self.service
            .call_service(
                &ServiceCallCommand::new(ALERT_SERVICE_ID, ALERT_RAISE_COMMAND, payload)?
                    .with_trace(trace),
            )
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use macaca_proto::MacacaResult;

    use super::*;
    use crate::service_client::{
        ServiceCallResult, ServiceInspectionCommand, ServiceInspectionResult,
    };

    /// Capturing service client used to verify the focused Alert facade.
    struct CapturingServiceClient {
        last_call: Mutex<Option<ServiceCallCommand>>,
    }

    impl CapturingServiceClient {
        fn new() -> Self {
            Self {
                last_call: Mutex::new(None),
            }
        }
    }

    #[async_trait]
    impl SystemServiceClient for CapturingServiceClient {
        async fn inspect_services(
            &self,
            command: &ServiceInspectionCommand,
        ) -> MacacaResult<ServiceInspectionResult> {
            Ok(ServiceInspectionResult {
                scope: command.scope.clone(),
                services: vec![ALERT_SERVICE_ID.into()],
            })
        }

        async fn call_service(
            &self,
            command: &ServiceCallCommand,
        ) -> MacacaResult<ServiceCallResult> {
            *self.last_call.lock().expect("last_call mutex") = Some(command.clone());
            Ok(ServiceCallResult {
                service_id: command.service_id.clone(),
                output: serde_json::json!({"delivered": true}),
            })
        }
    }

    #[tokio::test]
    async fn service_backed_alert_client_dispatches_raise_command_with_trace() {
        let service = Arc::new(CapturingServiceClient::new());
        let client = ServiceBackedAlertClient::new(service.clone());
        let trace = TraceContext::new("alert-client-test");
        client
            .raise(Alert::warning("title", "message", "source"), trace.clone())
            .await
            .unwrap();

        let captured = service
            .last_call
            .lock()
            .expect("last_call mutex")
            .clone()
            .expect("captured alert call");
        assert_eq!(captured.service_id, ALERT_SERVICE_ID);
        assert_eq!(captured.command_name, ALERT_RAISE_COMMAND);
        assert_eq!(captured.trace.unwrap().trace_id, trace.trace_id);
    }

    #[tokio::test]
    async fn unavailable_alert_client_returns_structured_error() {
        let error = UnavailableSystemAlertClient
            .raise(
                Alert::warning("title", "message", "source"),
                TraceContext::new("alert-unavailable-test"),
            )
            .await
            .unwrap_err();
        assert!(error.to_string().contains("Alert service is unavailable"));
    }
}
