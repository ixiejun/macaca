//! Runtime-host Bridge for provider-neutral key-value state services.
//!
//! This adapter is the sole runtime integration point for KV provider Strategies.
//! It reserves counter-only resources immediately before side effects, while
//! applications and SDKs remain unaware of embedded, remote, mock, or absent
//! state providers.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_foundation_key_value_state::{
    KeyValueResourceLedger, KeyValueStateService, UnavailableKeyValueStateProvider,
};
use macaca_kernel::SystemService;
use macaca_proto::{
    KeyValueResourceLimits, KeyValueResourceReservation, KeyValueStateProviderCapability,
    KeyValueStateProviderSnapshot, ServiceCallResult, ServiceCommand, ServiceDescriptor,
    ServiceHealth, ServiceResult,
};

/// Runtime composition Bridge that owns KV provider lifecycle and resource delegation.
pub struct FoundationKeyValueStateSystemServiceProvider {
    provider: Arc<dyn KeyValueStateService>,
    resource_ledger: KeyValueResourceLedger,
}

impl FoundationKeyValueStateSystemServiceProvider {
    /// Inject an approved provider Strategy from the runtime-host composition root.
    pub fn new(provider: Arc<dyn KeyValueStateService>) -> Self {
        Self::with_resource_ledger(
            provider,
            KeyValueResourceLedger::new(default_resource_limits()),
        )
    }

    /// Inject a bounded counter ledger for policy-controlled provider dispatch.
    pub fn with_resource_ledger(
        provider: Arc<dyn KeyValueStateService>,
        resource_ledger: KeyValueResourceLedger,
    ) -> Self {
        Self {
            provider,
            resource_ledger,
        }
    }

    /// Build the fail-closed fallback when no KV provider was installed.
    pub fn unavailable() -> Self {
        Self::new(Arc::new(UnavailableKeyValueStateProvider::default()))
    }

    /// Return sanitized Memento data for health and replay diagnostics.
    pub fn snapshot(&self) -> KeyValueStateProviderSnapshot {
        self.provider.snapshot()
    }

    /// Return provider capability facts without exposing provider clients or state values.
    pub fn provider_capabilities(&self) -> KeyValueStateProviderCapability {
        self.provider.provider_capabilities()
    }
}

#[async_trait]
impl SystemService for FoundationKeyValueStateSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.provider.descriptor()
    }

    async fn start(&self) -> ServiceResult<()> {
        tracing::info!(
            service_id = "service.foundation.key.value.state",
            "key-value state service started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        // Keeping the lease across the await guarantees timeout/cancellation drops it.
        let _lease = side_effect_reservation(&command)
            .map(|reservation| self.resource_ledger.reserve(reservation))
            .transpose()?;
        self.provider.call(command).await
    }

    async fn stop(&self) -> ServiceResult<()> {
        self.provider.shutdown().await?;
        tracing::info!(
            service_id = "service.foundation.key.value.state",
            "key-value state service stopped"
        );
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        self.provider.shutdown().await
    }
    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(self.provider.health())
    }
}

fn default_resource_limits() -> KeyValueResourceLimits {
    KeyValueResourceLimits {
        max_byte_units: 16 * 1024 * 1024,
        max_entry_units: 10_000,
        max_batch_operations: 1_024,
        max_watch_slots: 64,
        max_snapshot_units: 16 * 1024 * 1024,
        max_mutation_operations: 1_024,
        max_request_units: 4_096,
    }
}

/// Derive bounded counters only; namespace, key, value, and provider data stay private.
fn side_effect_reservation(command: &ServiceCommand) -> Option<KeyValueResourceReservation> {
    let name = command.name.as_str();
    let side_effect = matches!(
        name,
        "kv.put"
            | "kv.delete"
            | "kv.batch_put"
            | "kv.batch_delete"
            | "kv.compare_and_set"
            | "kv.increment"
            | "kv.set_ttl"
            | "kv.watch_namespace"
            | "kv.snapshot_namespace"
            | "kv.restore_namespace"
            | "kv.migrate_namespace"
            | "kv.compact_namespace"
    );
    side_effect.then(|| KeyValueResourceReservation {
        byte_units: command
            .payload
            .get("max_bytes")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            .min(16 * 1024 * 1024),
        entry_units: command
            .payload
            .get("page_size")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            .min(10_000) as u32,
        batch_operations: u32::from(matches!(name, "kv.batch_put" | "kv.batch_delete")),
        watch_slots: u32::from(name == "kv.watch_namespace"),
        snapshot_units: u64::from(name == "kv.snapshot_namespace")
            * command
                .payload
                .get("max_bytes")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0)
                .min(16 * 1024 * 1024),
        mutation_operations: u32::from(name != "kv.watch_namespace"),
        request_units: 1,
    })
}
