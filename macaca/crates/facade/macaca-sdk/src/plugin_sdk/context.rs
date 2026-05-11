//! Plugin runtime context builder.
//!
//! `PluginContext` is a developer-facing Facade over protocol metadata. It
//! mirrors the useful shape of plugin contexts from reference systems while
//! staying data-only: no internal registry, service runtime, kernel handle, or
//! host object is exposed to plugin packages.

use macaca_proto::{
    PluginAbiVersion, PluginHostDescriptor, PluginHostEntryPoint, PluginHostResourceLease,
    PluginHostTimeoutPolicy, PluginId, PluginRuntimeKind, PluginVersion,
};

/// Provider-neutral context passed to SDK builders and contract fixtures.
#[derive(Debug, Clone)]
pub struct PluginContext {
    plugin_id: PluginId,
    plugin_version: PluginVersion,
    runtime_kind: PluginRuntimeKind,
    abi_version: PluginAbiVersion,
    entry_point: Option<PluginHostEntryPoint>,
    timeout_policy: PluginHostTimeoutPolicy,
    resource_leases: Vec<PluginHostResourceLease>,
}

impl PluginContext {
    /// Create a context for one plugin identity and runtime kind.
    pub fn new(
        plugin_id: PluginId,
        plugin_version: PluginVersion,
        runtime_kind: PluginRuntimeKind,
    ) -> Self {
        Self {
            plugin_id,
            plugin_version,
            runtime_kind,
            abi_version: PluginAbiVersion::default(),
            entry_point: None,
            timeout_policy: PluginHostTimeoutPolicy::Default,
            resource_leases: Vec::new(),
        }
    }

    /// Override the ABI version for contract tests or future compatibility probes.
    pub fn with_abi_version(mut self, abi_version: PluginAbiVersion) -> Self {
        self.abi_version = abi_version;
        self
    }

    /// Attach host entry metadata without executing it.
    pub fn with_entry_point(mut self, entry_point: PluginHostEntryPoint) -> Self {
        self.entry_point = Some(entry_point);
        self
    }

    /// Attach a bounded timeout policy used by host supervisors.
    pub fn with_timeout_policy(mut self, timeout_policy: PluginHostTimeoutPolicy) -> Self {
        self.timeout_policy = timeout_policy;
        self
    }

    /// Add one declarative resource lease for host admission and diagnostics.
    pub fn with_resource_lease(mut self, lease: PluginHostResourceLease) -> Self {
        self.resource_leases.push(lease);
        self
    }

    /// Convert the context into a protocol host descriptor.
    pub fn host_descriptor(&self) -> PluginHostDescriptor {
        PluginHostDescriptor {
            plugin_id: self.plugin_id.clone(),
            plugin_version: self.plugin_version.clone(),
            runtime_kind: self.runtime_kind.clone(),
            abi_version: self.abi_version.clone(),
            entry_point: self.entry_point.clone(),
            timeout_policy: self.timeout_policy.clone(),
            resource_leases: self.resource_leases.clone(),
            metadata: Default::default(),
        }
    }
}
