//! Filesystem service contract and fail-closed unavailable Strategy.

use async_trait::async_trait;
use macaca_proto::{
    CapabilityId, CleanupPolicy, DomainPackProviderCapabilityState, FilesystemProviderCapability,
    FilesystemProviderSnapshot, KernelServiceId, ServiceCallResult, ServiceCapability,
    ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth, ServiceLifecycleState,
    ServiceResult, ServiceScope, ServiceType, TraceSchemaRef, FOUNDATION_FILESYSTEM_COMMANDS,
    FOUNDATION_FILESYSTEM_SERVICE_ID,
};

/// Provider-neutral Command boundary for all filesystem Strategies.
#[async_trait]
pub trait FilesystemService: Send + Sync {
    /// Return descriptor data used by the service runtime registry and discovery clients.
    fn descriptor(&self) -> ServiceDescriptor;
    /// Process one canonical trace-required filesystem command.
    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult>;
    /// Return a bounded service-health projection without host or path information.
    fn health(&self) -> ServiceHealth;
    /// Return replay-safe lifecycle mementos without file bytes, paths, or handles.
    fn snapshot(&self) -> FilesystemProviderSnapshot;
    /// Report provider capability facts without leaking composition details.
    fn provider_capabilities(&self) -> FilesystemProviderCapability;
    /// Release a watch checkpoint where the selected Strategy owns watch state.
    async fn cancel_watch(&self, _watch_checkpoint: &str) -> ServiceResult<()> {
        Err(ServiceError::UnsupportedCommand(
            "filesystem.watch.cancel".into(),
        ))
    }
    /// Stop the provider and release handles, watches, and bounded caches.
    async fn shutdown(&self) -> ServiceResult<()>;
}

/// Null Object used when no filesystem provider is installed in the host.
#[derive(Debug, Clone)]
pub struct UnavailableFilesystemProvider {
    reason: String,
}

impl UnavailableFilesystemProvider {
    /// Construct a fail-closed provider with a bounded safe diagnostic reason.
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

impl Default for UnavailableFilesystemProvider {
    fn default() -> Self {
        Self::new("foundation filesystem provider is not installed")
    }
}

#[async_trait]
impl FilesystemService for UnavailableFilesystemProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        let mut descriptor = ServiceDescriptor::new(
            KernelServiceId::new(FOUNDATION_FILESYSTEM_SERVICE_ID),
            ServiceType::new("foundation.filesystem"),
            TraceSchemaRef::new("macaca.trace.foundation.filesystem.v1"),
        );
        descriptor.lifecycle_state = ServiceLifecycleState::Registered;
        descriptor.health = ServiceHealth::Unavailable {
            reason: self.reason.clone(),
        };
        descriptor.supported_scopes = vec![ServiceScope::Global];
        descriptor.cleanup_policy = CleanupPolicy::None;
        descriptor.capabilities = FOUNDATION_FILESYSTEM_COMMANDS
            .iter()
            .map(|name| ServiceCapability::new(CapabilityId::new(*name), "filesystem command"))
            .collect();
        descriptor
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = command.trace.ok_or(ServiceError::MissingTraceContext)?;
        tracing::warn!(service_id = FOUNDATION_FILESYSTEM_SERVICE_ID, command = %command.name,
            trace_id = %trace.trace_id, "filesystem command rejected: provider unavailable");
        Ok(ServiceCallResult {
            output: serde_json::json!({"status":"unavailable","reason":self.reason}),
            trace,
            status: "unavailable".into(),
            metadata: [(
                "filesystem.audit_event".into(),
                "filesystem_pack_unavailable".into(),
            )]
            .into_iter()
            .collect(),
            cleanup_hint: Some(CleanupPolicy::None),
        })
    }

    fn health(&self) -> ServiceHealth {
        ServiceHealth::Unavailable {
            reason: self.reason.clone(),
        }
    }

    fn snapshot(&self) -> FilesystemProviderSnapshot {
        FilesystemProviderSnapshot {
            descriptor_hash: "foundation-filesystem-unavailable-v1".into(),
            provider_class: "unavailable".into(),
            open_handle_count: 0,
            active_watch_count: 0,
            root_hashes: Default::default(),
        }
    }

    fn provider_capabilities(&self) -> FilesystemProviderCapability {
        FilesystemProviderCapability {
            provider_class: "unavailable".into(),
            supported_commands: Default::default(),
            supported_root_kinds: Default::default(),
            supports_recursive_operations: false,
            supports_watch: false,
            supports_snapshot: false,
            supports_atomic_write: false,
            max_file_bytes: 0,
            max_directory_entries: 0,
            availability: DomainPackProviderCapabilityState::Unavailable,
            unavailable_reason: Some("filesystem provider is not installed".into()),
        }
    }

    async fn shutdown(&self) -> ServiceResult<()> {
        Ok(())
    }
}
