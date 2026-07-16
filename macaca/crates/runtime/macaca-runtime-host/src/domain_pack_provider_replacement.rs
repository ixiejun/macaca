//! Generic replacement providers for domain-pack service conformance.
//!
//! Domain packs are optional serviceized capabilities.  This module owns only provider-neutral
//! mock and unavailable adapters used by package conformance, runtime bootstrap tests, and SDK
//! diagnostics.  It deliberately avoids business-domain command semantics: concrete packs still
//! own their DTOs, policies, providers, and examples.

use std::{collections::BTreeMap, sync::Arc};

use async_trait::async_trait;
use macaca_kernel::{DomainPackProviderRegistration, SystemService};
use macaca_proto::{
    domain_pack_command_trace, CleanupPolicy, DomainPackProviderSnapshot, KernelServiceId,
    ServiceCallResult, ServiceCommand, ServiceDescriptor, ServiceHealth, ServiceLifecycleState,
    ServiceResult,
};
use tracing::info;

/// Generic deterministic provider used by domain-pack contract and integration tests.
///
/// This provider is a test Strategy behind the normal `SystemService` boundary.  It proves
/// provider replacement without embedding business behavior for any concrete pack: every command
/// receives the same bounded, descriptor-derived envelope, and the original command payload is
/// intentionally not echoed into logs, snapshots, or service results.
pub struct DomainPackMockSystemServiceProvider {
    descriptor: ServiceDescriptor,
    pack_id: String,
}

impl DomainPackMockSystemServiceProvider {
    /// Build a mock provider from descriptor data supplied by a package or test composition root.
    pub fn new(mut descriptor: ServiceDescriptor, pack_id: impl Into<String>) -> Self {
        let pack_id = pack_id.into();
        descriptor.lifecycle_state = ServiceLifecycleState::Registered;
        descriptor.health = ServiceHealth::Healthy;
        descriptor
            .metadata
            .insert("provider_class".into(), "mock".into());
        descriptor
            .metadata
            .insert("pack_id".into(), pack_id.clone());
        Self {
            descriptor,
            pack_id,
        }
    }

    /// Return a bounded snapshot that identifies replacement state without leaking payloads.
    pub fn snapshot(&self, trace_id: impl Into<String>) -> DomainPackProviderSnapshot {
        DomainPackProviderSnapshot {
            pack_id: self.pack_id.clone(),
            service_id: self.descriptor.id.to_string(),
            provider_class: "mock".into(),
            health: "available".into(),
            unavailable_reason: None,
            trace_id: Some(trace_id.into()),
        }
    }

    fn service_id(&self) -> KernelServiceId {
        self.descriptor.id.clone()
    }
}

#[async_trait]
impl SystemService for DomainPackMockSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.service_id(),
            pack_id = %self.pack_id,
            "domain_pack_mock_provider_started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = domain_pack_command_trace(&command)?;
        info!(
            service_id = %self.service_id(),
            pack_id = %self.pack_id,
            command = %command.name,
            trace_id = %trace.trace_id,
            "domain_pack_mock_provider_call_completed"
        );

        let mut metadata = BTreeMap::new();
        metadata.insert("provider_class".into(), "mock".into());
        metadata.insert("pack_id".into(), self.pack_id.clone());

        Ok(ServiceCallResult {
            output: serde_json::json!({
                "status": "ok",
                "pack_id": self.pack_id.clone(),
                "service_id": self.descriptor.id.to_string(),
                "command": command.name.to_string(),
                "mock": true,
            }),
            trace,
            status: "ok".into(),
            metadata,
            cleanup_hint: Some(CleanupPolicy::None),
        })
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.service_id(),
            pack_id = %self.pack_id,
            "domain_pack_mock_provider_stopped"
        );
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.service_id(),
            pack_id = %self.pack_id,
            "domain_pack_mock_provider_cleanup_completed"
        );
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(ServiceHealth::Healthy)
    }
}

/// Generic fail-closed provider used when a domain-pack descriptor exists but no callable
/// implementation is installed in the active composition root.
///
/// This is a Null Object plus Adapter implementation: it satisfies the `SystemService`
/// lifecycle contract so registries, SDKs, and shells can inspect a normal service boundary,
/// while every command returns a bounded `unavailable` result instead of crashing, hanging,
/// silently falling back, or pretending success.  It intentionally does not parse command
/// payloads, provider names, application names, or business-domain fields.
pub struct DomainPackUnavailableSystemServiceProvider {
    descriptor: ServiceDescriptor,
    pack_id: String,
    reason_code: String,
}

impl DomainPackUnavailableSystemServiceProvider {
    /// Build an unavailable provider from a provider-neutral descriptor.
    pub fn new(
        mut descriptor: ServiceDescriptor,
        pack_id: impl Into<String>,
        reason_code: impl Into<String>,
    ) -> Self {
        let pack_id = pack_id.into();
        let reason_code = reason_code.into();
        descriptor.lifecycle_state = ServiceLifecycleState::Registered;
        descriptor.health = ServiceHealth::Unavailable {
            reason: reason_code.clone(),
        };
        descriptor
            .metadata
            .insert("provider_class".into(), "unavailable".into());
        descriptor
            .metadata
            .insert("pack_id".into(), pack_id.clone());
        descriptor
            .metadata
            .insert("unavailable_reason".into(), reason_code.clone());
        Self {
            descriptor,
            pack_id,
            reason_code,
        }
    }

    /// Return a sanitized snapshot for diagnostics without exposing command payloads.
    pub fn snapshot(&self) -> DomainPackProviderSnapshot {
        DomainPackProviderSnapshot::unavailable(
            self.pack_id.clone(),
            self.descriptor.id.to_string(),
            self.reason_code.clone(),
        )
    }

    fn service_id(&self) -> KernelServiceId {
        self.descriptor.id.clone()
    }
}

#[async_trait]
impl SystemService for DomainPackUnavailableSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.service_id(),
            pack_id = %self.pack_id,
            reason_code = %self.reason_code,
            "domain_pack_unavailable_provider_started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = domain_pack_command_trace(&command)?;
        info!(
            service_id = %self.service_id(),
            pack_id = %self.pack_id,
            command = %command.name,
            trace_id = %trace.trace_id,
            reason_code = %self.reason_code,
            "domain_pack_unavailable_provider_call_rejected"
        );

        let mut metadata = BTreeMap::new();
        metadata.insert("provider_class".into(), "unavailable".into());
        metadata.insert("pack_id".into(), self.pack_id.clone());
        metadata.insert("reason_code".into(), self.reason_code.clone());

        Ok(ServiceCallResult {
            output: serde_json::json!({
                "status": "unavailable",
                "pack_id": self.pack_id.clone(),
                "service_id": self.descriptor.id.to_string(),
                "command": command.name.to_string(),
                "reason_code": self.reason_code.clone(),
            }),
            trace,
            status: "unavailable".into(),
            metadata,
            cleanup_hint: Some(CleanupPolicy::None),
        })
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.service_id(),
            pack_id = %self.pack_id,
            "domain_pack_unavailable_provider_stopped"
        );
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.service_id(),
            pack_id = %self.pack_id,
            "domain_pack_unavailable_provider_cleanup_completed"
        );
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(ServiceHealth::Unavailable {
            reason: self.reason_code.clone(),
        })
    }
}

/// Build a domain-pack registration around the generic unavailable provider.
pub fn unavailable_domain_pack_provider_registration(
    descriptor: ServiceDescriptor,
    pack_id: impl Into<String>,
    reason_code: impl Into<String>,
    trace_suffix: impl Into<String>,
) -> DomainPackProviderRegistration {
    let pack_id = pack_id.into();
    let service = Arc::new(DomainPackUnavailableSystemServiceProvider::new(
        descriptor.clone(),
        pack_id.clone(),
        reason_code,
    ));
    DomainPackProviderRegistration::new(descriptor, service, trace_suffix).with_pack_id(pack_id)
}

/// Build a domain-pack registration around the generic deterministic mock provider.
pub fn mock_domain_pack_provider_registration(
    descriptor: ServiceDescriptor,
    pack_id: impl Into<String>,
    trace_suffix: impl Into<String>,
) -> DomainPackProviderRegistration {
    let pack_id = pack_id.into();
    let service = Arc::new(DomainPackMockSystemServiceProvider::new(
        descriptor.clone(),
        pack_id.clone(),
    ));
    DomainPackProviderRegistration::new(descriptor, service, trace_suffix).with_pack_id(pack_id)
}

#[cfg(test)]
mod tests {
    use macaca_kernel::SystemService;
    use macaca_proto::{
        DomainPackProviderCapabilityState, KernelServiceId, ServiceCommand, ServiceCommandName,
        ServiceDescriptor, ServiceError, ServiceHealth, ServiceType, TraceContext, TraceSchemaRef,
    };

    use super::{DomainPackMockSystemServiceProvider, DomainPackUnavailableSystemServiceProvider};

    #[tokio::test]
    async fn unavailable_provider_requires_trace_before_reporting_absence() {
        let provider = unavailable_provider();
        let error = provider
            .call(ServiceCommand::without_trace(
                ServiceCommandName::new("any.command"),
                serde_json::json!({ "secret": "must-not-leak" }),
            ))
            .await
            .unwrap_err();

        assert_eq!(error, ServiceError::MissingTraceContext);
    }

    #[tokio::test]
    async fn mock_provider_requires_trace_before_returning_synthetic_output() {
        let provider = mock_provider();
        let error = provider
            .call(ServiceCommand::without_trace(
                ServiceCommandName::new("any.command"),
                serde_json::json!({ "secret": "must-not-leak" }),
            ))
            .await
            .unwrap_err();

        assert_eq!(error, ServiceError::MissingTraceContext);
    }

    #[tokio::test]
    async fn mock_provider_returns_deterministic_bounded_output_without_payload_echo() {
        let provider = mock_provider();
        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new("any.command"),
                serde_json::json!({ "secret": "must-not-leak" }),
                TraceContext::new("trace-domain-pack-mock"),
            ))
            .await
            .unwrap();

        assert_eq!(result.status, "ok");
        assert_eq!(result.output["status"], "ok");
        assert_eq!(result.output["pack_id"], "pack.example.synthetic.v1");
        assert_eq!(result.output["mock"], true);
        assert_eq!(
            result.metadata.get("provider_class").map(String::as_str),
            Some("mock")
        );
        assert!(!result.output.to_string().contains("must-not-leak"));
    }

    #[tokio::test]
    async fn mock_provider_descriptor_snapshot_and_health_match() {
        let provider = mock_provider();
        let descriptor = provider.descriptor();
        let snapshot = provider.snapshot("trace-domain-pack-mock");
        let report = snapshot.capability_report();
        let health = provider.health().await.unwrap();

        assert_eq!(
            descriptor
                .metadata
                .get("provider_class")
                .map(String::as_str),
            Some("mock")
        );
        assert!(matches!(descriptor.health, ServiceHealth::Healthy));
        assert!(matches!(health, ServiceHealth::Healthy));
        assert_eq!(snapshot.provider_class, "mock");
        assert_eq!(report.provider_class, "mock");
        assert_eq!(report.state, DomainPackProviderCapabilityState::Available);
    }

    #[tokio::test]
    async fn unavailable_provider_returns_bounded_diagnostic_without_payload_echo() {
        let provider = unavailable_provider();
        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new("any.command"),
                serde_json::json!({ "secret": "must-not-leak" }),
                TraceContext::new("trace-domain-pack-unavailable"),
            ))
            .await
            .unwrap();

        assert_eq!(result.status, "unavailable");
        assert_eq!(result.output["status"], "unavailable");
        assert_eq!(result.output["pack_id"], "pack.example.synthetic.v1");
        assert_eq!(result.output["command"], "any.command");
        assert_eq!(
            result.metadata.get("provider_class").map(String::as_str),
            Some("unavailable")
        );
        assert!(!result.output.to_string().contains("must-not-leak"));
    }

    #[tokio::test]
    async fn unavailable_provider_descriptor_snapshot_and_health_match() {
        let provider = unavailable_provider();
        let descriptor = provider.descriptor();
        let snapshot = provider.snapshot();
        let report = snapshot.capability_report();
        let health = provider.health().await.unwrap();

        assert_eq!(
            descriptor
                .metadata
                .get("provider_class")
                .map(String::as_str),
            Some("unavailable")
        );
        assert!(matches!(
            descriptor.health,
            ServiceHealth::Unavailable { .. }
        ));
        assert!(matches!(health, ServiceHealth::Unavailable { .. }));
        assert_eq!(snapshot.provider_class, "unavailable");
        assert_eq!(report.provider_class, "unavailable");
        assert_eq!(report.reason_code, "provider_absent");
    }

    fn unavailable_provider() -> DomainPackUnavailableSystemServiceProvider {
        DomainPackUnavailableSystemServiceProvider::new(
            synthetic_descriptor(),
            "pack.example.synthetic.v1",
            "provider_absent",
        )
    }

    fn mock_provider() -> DomainPackMockSystemServiceProvider {
        DomainPackMockSystemServiceProvider::new(
            synthetic_descriptor(),
            "pack.example.synthetic.v1",
        )
    }

    fn synthetic_descriptor() -> ServiceDescriptor {
        ServiceDescriptor::new(
            KernelServiceId::new("service.domain_pack.synthetic"),
            ServiceType::new("domain_pack.synthetic"),
            TraceSchemaRef::new("trace.domain_pack.synthetic.v1"),
        )
    }
}
