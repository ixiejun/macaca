//! Runtime environment provider seam for the Tool Capability Plane.
//!
//! The runtime host owns this Abstract Factory / Adapter layer because it is the
//! approved composition root for local, sandboxed, remote, WASM, browser, and
//! unavailable environment providers.  The code records sanitized lifecycle
//! evidence and never constructs a provider from application-specific logic.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    MacacaResult, ServiceHealth, ToolAuditRef, ToolEnvironmentCleanupCommand,
    ToolEnvironmentCleanupResult, ToolEnvironmentHealthResult, ToolRuntimeEnvironmentDescriptor,
    ToolRuntimeEnvironmentKind, TraceContext,
};

/// Provider contract implemented by built-in, plugin, remote, mock, and
/// unavailable environment adapters.
#[async_trait]
pub trait ToolRuntimeEnvironmentProvider: Send + Sync {
    fn provider_id(&self) -> &str;
    async fn descriptor(&self) -> MacacaResult<ToolRuntimeEnvironmentDescriptor>;
    async fn cleanup(
        &self,
        command: ToolEnvironmentCleanupCommand,
    ) -> MacacaResult<ToolEnvironmentCleanupResult>;
}

/// Null Object provider used when an optional environment backend is absent.
pub struct UnavailableToolRuntimeEnvironmentProvider {
    provider_id: String,
    environment_id: String,
    kind: ToolRuntimeEnvironmentKind,
    reason: String,
}

impl UnavailableToolRuntimeEnvironmentProvider {
    pub fn new(
        provider_id: impl Into<String>,
        environment_id: impl Into<String>,
        kind: ToolRuntimeEnvironmentKind,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            environment_id: environment_id.into(),
            kind,
            reason: reason.into(),
        }
    }
}

#[async_trait]
impl ToolRuntimeEnvironmentProvider for UnavailableToolRuntimeEnvironmentProvider {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    async fn descriptor(&self) -> MacacaResult<ToolRuntimeEnvironmentDescriptor> {
        tracing::warn!(
            provider_id = %self.provider_id,
            environment_id = %self.environment_id,
            reason_code = "environment_provider_unavailable",
            "tool runtime environment provider unavailable"
        );
        Ok(ToolRuntimeEnvironmentDescriptor::unavailable(
            self.environment_id.clone(),
            self.provider_id.clone(),
            self.kind.clone(),
            self.reason.clone(),
        ))
    }

    async fn cleanup(
        &self,
        command: ToolEnvironmentCleanupCommand,
    ) -> MacacaResult<ToolEnvironmentCleanupResult> {
        tracing::info!(
            trace_id = %command.trace.trace_id,
            provider_id = %self.provider_id,
            environment_id = %command.environment_id,
            reason_code = %command.reason_code,
            cleanup_status = "unavailable_noop",
            "tool runtime environment cleanup recorded for unavailable provider"
        );
        Ok(ToolEnvironmentCleanupResult {
            trace: command.trace,
            environment_id: command.environment_id,
            status: "unavailable".into(),
            released_process_handles: Vec::new(),
            released_artifact_roots: Vec::new(),
            audit_refs: vec![ToolAuditRef::new("tool.environment.cleanup.unavailable")],
            captured_at: chrono::Utc::now(),
            metadata: BTreeMap::new(),
        })
    }
}

/// Static descriptor adapter for currently available local providers.
///
/// This adapter keeps local workspace and sandbox descriptors as data.  The
/// provider id, environment id, policies, artifact roots, and process handles
/// are supplied by composition code or tests, not by OS control-flow branches.
pub struct StaticToolRuntimeEnvironmentProvider {
    descriptor: ToolRuntimeEnvironmentDescriptor,
}

impl StaticToolRuntimeEnvironmentProvider {
    pub fn new(descriptor: ToolRuntimeEnvironmentDescriptor) -> Self {
        Self { descriptor }
    }
}

#[async_trait]
impl ToolRuntimeEnvironmentProvider for StaticToolRuntimeEnvironmentProvider {
    fn provider_id(&self) -> &str {
        &self.descriptor.provider_id
    }

    async fn descriptor(&self) -> MacacaResult<ToolRuntimeEnvironmentDescriptor> {
        Ok(self.descriptor.clone())
    }

    async fn cleanup(
        &self,
        command: ToolEnvironmentCleanupCommand,
    ) -> MacacaResult<ToolEnvironmentCleanupResult> {
        let released_process_handles = self
            .descriptor
            .process_handles
            .iter()
            .map(|handle| handle.handle_ref.clone())
            .collect::<Vec<_>>();
        let released_artifact_roots = self
            .descriptor
            .artifact_roots
            .iter()
            .map(|root| root.root_ref.clone())
            .collect::<Vec<_>>();
        tracing::info!(
            trace_id = %command.trace.trace_id,
            provider_id = %self.descriptor.provider_id,
            environment_id = %command.environment_id,
            reason_code = %command.reason_code,
            process_count = released_process_handles.len(),
            artifact_root_count = released_artifact_roots.len(),
            cleanup_status = "released",
            "tool runtime environment cleanup released provider-owned resources"
        );
        Ok(ToolEnvironmentCleanupResult {
            trace: command.trace,
            environment_id: command.environment_id,
            status: "released".into(),
            released_process_handles,
            released_artifact_roots,
            audit_refs: vec![ToolAuditRef::new("tool.environment.cleanup.released")],
            captured_at: chrono::Utc::now(),
            metadata: BTreeMap::new(),
        })
    }
}

/// Registry-style service used by `service.tool` planning and diagnostics.
pub struct ToolRuntimeEnvironmentService {
    providers: Vec<Arc<dyn ToolRuntimeEnvironmentProvider>>,
}

impl ToolRuntimeEnvironmentService {
    pub fn new(providers: Vec<Arc<dyn ToolRuntimeEnvironmentProvider>>) -> Self {
        Self { providers }
    }

    pub async fn health(&self, trace: TraceContext) -> MacacaResult<ToolEnvironmentHealthResult> {
        let mut descriptors = Vec::new();
        for provider in &self.providers {
            descriptors.push(provider.descriptor().await?);
        }
        let unavailable_count = descriptors
            .iter()
            .filter(|descriptor| matches!(descriptor.health, ServiceHealth::Unavailable { .. }))
            .count();
        tracing::info!(
            trace_id = %trace.trace_id,
            provider_count = self.providers.len(),
            unavailable_count,
            "tool runtime environment health snapshot captured"
        );
        Ok(ToolEnvironmentHealthResult {
            trace,
            descriptors,
            captured_at: chrono::Utc::now(),
            audit_refs: vec![ToolAuditRef::new("tool.environment.health")],
            metadata: BTreeMap::new(),
        })
    }

    pub async fn cleanup(
        &self,
        command: ToolEnvironmentCleanupCommand,
    ) -> MacacaResult<ToolEnvironmentCleanupResult> {
        for provider in &self.providers {
            let descriptor = provider.descriptor().await?;
            if descriptor.environment_id == command.environment_id {
                return provider.cleanup(command).await;
            }
        }
        tracing::warn!(
            trace_id = %command.trace.trace_id,
            environment_id = %command.environment_id,
            reason_code = "environment_provider_not_found",
            "tool runtime environment cleanup target was not registered"
        );
        Ok(ToolEnvironmentCleanupResult {
            trace: command.trace,
            environment_id: command.environment_id,
            status: "unavailable".into(),
            released_process_handles: Vec::new(),
            released_artifact_roots: Vec::new(),
            audit_refs: vec![ToolAuditRef::new("tool.environment.cleanup.not_found")],
            captured_at: chrono::Utc::now(),
            metadata: BTreeMap::new(),
        })
    }
}
