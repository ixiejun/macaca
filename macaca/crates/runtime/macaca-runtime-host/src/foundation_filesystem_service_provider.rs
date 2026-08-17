//! Runtime-host Bridge for provider-neutral foundation filesystem services.
//!
//! This adapter is the only integration point between the generic service runtime
//! and a concrete filesystem Strategy. SDK, shell, application-framework, and
//! kernel layers remain unaware of whether the provider is local, remote, mock,
//! plugin-backed, or unavailable.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_foundation_filesystem::{
    FilesystemResourceLedger, FilesystemService, UnavailableFilesystemProvider,
};
use macaca_kernel::SystemService;
use macaca_proto::{
    FilesystemProviderCapability, FilesystemProviderSnapshot, FilesystemResourceLimits,
    FilesystemResourceReservation, ServiceCallResult, ServiceCommand, ServiceDescriptor,
    ServiceHealth, ServiceResult,
};

/// Runtime composition Bridge that owns filesystem provider lifecycle delegation.
pub struct FoundationFilesystemSystemServiceProvider {
    provider: Arc<dyn FilesystemService>,
    resource_ledger: FilesystemResourceLedger,
}

impl FoundationFilesystemSystemServiceProvider {
    /// Inject an approved provider Strategy from the runtime-host composition root.
    pub fn new(provider: Arc<dyn FilesystemService>) -> Self {
        Self::with_resource_ledger(
            provider,
            FilesystemResourceLedger::new(default_resource_limits()),
        )
    }

    /// Inject a bounded resource ledger from the runtime-host composition root.
    pub fn with_resource_ledger(
        provider: Arc<dyn FilesystemService>,
        resource_ledger: FilesystemResourceLedger,
    ) -> Self {
        Self {
            provider,
            resource_ledger,
        }
    }

    /// Build the fail-closed fallback when no filesystem provider was installed.
    pub fn unavailable() -> Self {
        Self::new(Arc::new(UnavailableFilesystemProvider::default()))
    }

    /// Return sanitized Memento data for health and replay diagnostics.
    pub fn snapshot(&self) -> FilesystemProviderSnapshot {
        self.provider.snapshot()
    }

    /// Return provider capabilities without exposing host roots or native handles.
    pub fn provider_capabilities(&self) -> FilesystemProviderCapability {
        self.provider.provider_capabilities()
    }

    /// Delegate lifecycle-owned watch cancellation to the selected Strategy.
    pub async fn cancel_watch(&self, watch_checkpoint: &str) -> ServiceResult<()> {
        self.provider.cancel_watch(watch_checkpoint).await
    }
}

#[async_trait]
impl SystemService for FoundationFilesystemSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.provider.descriptor()
    }

    async fn start(&self) -> ServiceResult<()> {
        tracing::info!(
            service_id = "service.foundation.filesystem",
            "foundation filesystem service started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        // The guard remains live across the provider await. A router timeout or
        // task cancellation drops it during future unwinding, releasing counters
        // before a later filesystem call can observe stale reservation state.
        let _lease = side_effect_reservation(&command)
            .map(|reservation| self.resource_ledger.reserve(reservation))
            .transpose()?;
        self.provider.call(command).await
    }

    async fn stop(&self) -> ServiceResult<()> {
        self.provider.shutdown().await?;
        tracing::info!(
            service_id = "service.foundation.filesystem",
            "foundation filesystem service stopped"
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

/// Conservative default limits for a generic host composition.
fn default_resource_limits() -> FilesystemResourceLimits {
    FilesystemResourceLimits {
        max_byte_units: 16 * 1024 * 1024,
        max_entry_units: 10_000,
        max_recursive_operations: 8,
        max_watch_slots: 64,
        max_snapshot_units: 16 * 1024 * 1024,
        max_mutation_operations: 1_024,
        max_request_units: 4_096,
    }
}

/// Derive only bounded counters; paths, handles, and content references stay private.
fn side_effect_reservation(command: &ServiceCommand) -> Option<FilesystemResourceReservation> {
    let name = command.name.as_str();
    let side_effect = matches!(
        name,
        "filesystem.write_file"
            | "filesystem.append_file"
            | "filesystem.create_directory"
            | "filesystem.copy_path"
            | "filesystem.move_path"
            | "filesystem.delete_path"
            | "filesystem.create_temp"
            | "filesystem.watch_path"
            | "filesystem.snapshot_tree"
            | "filesystem.restore_snapshot"
    );
    side_effect.then(|| FilesystemResourceReservation {
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
        recursive_operations: u32::from(
            command
                .payload
                .get("recursive")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
        ),
        watch_slots: u32::from(name == "filesystem.watch_path"),
        snapshot_units: if name == "filesystem.snapshot_tree" {
            command
                .payload
                .get("max_bytes")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0)
                .min(16 * 1024 * 1024)
        } else {
            0
        },
        mutation_operations: u32::from(name != "filesystem.watch_path"),
        request_units: 1,
    })
}
