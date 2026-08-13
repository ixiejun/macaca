//! Runtime-host Bridge for the provider-neutral foundation time service.
//!
//! This adapter is the composition boundary between the kernel lifecycle
//! contract and injected host, remote, plugin, frozen, or unavailable clocks.
//! It never leaks native clock or timer handles beyond a traced service call.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    ServiceCallResult, ServiceCommand, ServiceDescriptor, ServiceHealth, ServiceResult,
    FOUNDATION_TIME_SERVICE_ID,
};
use macaca_time::{TimeService, UnavailableTimeProvider};
use tracing::info;

/// Adapter/Bridge from the generic service runtime to a replaceable time provider.
pub struct TimeSystemServiceProvider {
    provider: Arc<dyn TimeService>,
}

impl TimeSystemServiceProvider {
    /// Inject a host, remote, plugin, frozen-test, or unavailable time Strategy.
    pub fn new(provider: Arc<dyn TimeService>) -> Self {
        Self { provider }
    }
    /// Construct the fail-closed optional-module state.
    pub fn unavailable() -> Self {
        Self::new(Arc::new(UnavailableTimeProvider::default()))
    }
}

#[async_trait]
impl SystemService for TimeSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.provider.descriptor()
    }
    async fn start(&self) -> ServiceResult<()> {
        info!(
            service_id = FOUNDATION_TIME_SERVICE_ID,
            "time system service started"
        );
        Ok(())
    }
    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        self.provider.call(command).await
    }
    async fn stop(&self) -> ServiceResult<()> {
        self.provider.shutdown().await?;
        info!(
            service_id = FOUNDATION_TIME_SERVICE_ID,
            "time system service stopped and timers released"
        );
        Ok(())
    }
    async fn cleanup(&self) -> ServiceResult<()> {
        info!(
            service_id = FOUNDATION_TIME_SERVICE_ID,
            "time system service cleanup completed"
        );
        Ok(())
    }
    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(self.provider.health())
    }
}

#[cfg(test)]
mod tests {
    use super::TimeSystemServiceProvider;
    use macaca_kernel::SystemService;
    use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};
    use macaca_time::{FrozenTimeProvider, UnavailableTimeProvider};
    use std::sync::Arc;

    #[tokio::test]
    async fn frozen_provider_returns_trace_correlated_clock_decision() {
        let provider = TimeSystemServiceProvider::new(Arc::new(FrozenTimeProvider::new(42)));
        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new("time.now"),
                serde_json::json!({}),
                TraceContext::new("trace-time-frozen"),
            ))
            .await
            .unwrap();
        assert_eq!(result.trace.trace_id.as_str(), "trace-time-frozen");
        assert_eq!(result.output["epoch_millis"], 42);
    }

    #[tokio::test]
    async fn unavailable_provider_fails_closed_with_trace_evidence() {
        let provider =
            TimeSystemServiceProvider::new(Arc::new(UnavailableTimeProvider::new("not_installed")));
        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new("time.now"),
                serde_json::json!({}),
                TraceContext::new("trace-time-unavailable"),
            ))
            .await
            .unwrap();
        assert_eq!(result.status, "unavailable");
        assert_eq!(result.trace.trace_id.as_str(), "trace-time-unavailable");
    }
}
