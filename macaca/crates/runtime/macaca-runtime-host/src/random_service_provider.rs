//! Runtime-host Adapter for the provider-neutral random service.
//!
//! Runtime-host is the only composition boundary that exposes a concrete random
//! provider to the kernel-visible `SystemService` trait. SDKs, shells, WASM,
//! and applications use traced service calls and therefore cannot obtain native
//! entropy handles or bypass service policy/audit decorators.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    RandomProviderSnapshot, ServiceCallResult, ServiceCommand, ServiceDescriptor, ServiceHealth,
    ServiceResult,
};
use macaca_random::{RandomService, UnavailableRandomProvider};
use tracing::info;

/// Adapter/Bridge from the generic runtime service protocol to `RandomService`.
pub struct RandomSystemServiceProvider {
    provider: Arc<dyn RandomService>,
}

impl RandomSystemServiceProvider {
    /// Wrap an injected host, remote, plugin, test, or unavailable random provider.
    pub fn new(provider: Arc<dyn RandomService>) -> Self {
        Self { provider }
    }

    /// Build the fail-closed optional-module provider.
    pub fn unavailable() -> Self {
        Self::new(Arc::new(UnavailableRandomProvider::default()))
    }

    /// Return the provider's sanitized replay Memento without generated values.
    pub fn snapshot(&self) -> RandomProviderSnapshot {
        self.provider.snapshot()
    }
}

#[async_trait]
impl SystemService for RandomSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.provider.descriptor()
    }

    async fn start(&self) -> ServiceResult<()> {
        info!(
            service_id = "service.foundation.random",
            "random system service started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        self.provider.call(command).await
    }

    async fn stop(&self) -> ServiceResult<()> {
        self.provider.shutdown().await?;
        info!(
            service_id = "service.foundation.random",
            "random system service stopped"
        );
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        info!(
            service_id = "service.foundation.random",
            "random system service cleanup completed"
        );
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(self.provider.health())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use macaca_kernel::SystemService;
    use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};
    use macaca_random::{
        DeterministicRandomProvider, HostRandomProvider, UnavailableRandomProvider,
    };

    use super::RandomSystemServiceProvider;

    #[tokio::test]
    async fn host_provider_returns_trace_correlated_output_without_loggable_input() {
        let provider = RandomSystemServiceProvider::new(Arc::new(HostRandomProvider));
        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new("random.bytes"),
                serde_json::json!({"length": 16}),
                TraceContext::new("trace-random-host"),
            ))
            .await
            .unwrap();
        assert_eq!(result.trace.trace_id.as_str(), "trace-random-host");
        assert!(!result.output.to_string().contains("seed"));
    }

    #[tokio::test]
    async fn unavailable_provider_fails_closed_with_trace_evidence() {
        let provider = RandomSystemServiceProvider::new(Arc::new(UnavailableRandomProvider::new(
            "not_installed",
        )));
        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new("random.bytes"),
                serde_json::json!({"length": 16}),
                TraceContext::new("trace-random-unavailable"),
            ))
            .await
            .unwrap();
        assert_eq!(result.status, "unavailable");
        assert_eq!(result.trace.trace_id.as_str(), "trace-random-unavailable");
    }

    #[test]
    fn provider_snapshot_excludes_random_material() {
        let provider = RandomSystemServiceProvider::new(Arc::new(HostRandomProvider));
        let snapshot = provider.snapshot();
        assert_eq!(snapshot.provider_class, "host-csprng");
        assert!(snapshot.stream_position_hashes.is_empty());
    }

    #[tokio::test]
    async fn deterministic_provider_lifecycle_clears_snapshot_state_on_stop() {
        let provider =
            RandomSystemServiceProvider::new(Arc::new(DeterministicRandomProvider::default()));
        provider.start().await.unwrap();
        provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new("random.test_stream_bytes"),
                serde_json::json!({"stream_id":"test-stream","length":16}),
                TraceContext::new("trace-random-lifecycle"),
            ))
            .await
            .unwrap();
        assert!(!provider.snapshot().stream_position_hashes.is_empty());
        assert!(matches!(
            provider.health().await.unwrap(),
            macaca_proto::ServiceHealth::Healthy
        ));
        provider.stop().await.unwrap();
        assert!(provider.snapshot().stream_position_hashes.is_empty());
    }
}
