//! Compatibility checker host context.
//!
//! The context captures host-side facts for one certification run.  It is
//! value-only so tests, CLI, Web UI, and Store tooling can construct it without
//! depending on concrete services or providers.

use std::collections::BTreeSet;

use macaca_proto::{EntitlementState, KernelServiceId};

/// Host capabilities and versions used for one certification run.
#[derive(Debug, Clone)]
pub struct CompatibilityHostContext {
    pub os_version: String,
    pub supported_manifest_versions: BTreeSet<String>,
    pub supported_abi_versions: BTreeSet<String>,
    pub available_services: BTreeSet<KernelServiceId>,
    pub available_optional_modules: BTreeSet<KernelServiceId>,
    pub entitlement_states: BTreeSet<String>,
}

impl Default for CompatibilityHostContext {
    fn default() -> Self {
        Self {
            os_version: "1".into(),
            supported_manifest_versions: BTreeSet::from(["1".into()]),
            supported_abi_versions: BTreeSet::from(["1".into()]),
            available_services: BTreeSet::new(),
            available_optional_modules: BTreeSet::new(),
            entitlement_states: BTreeSet::new(),
        }
    }
}

impl CompatibilityHostContext {
    /// Add one available required service.
    pub fn with_service(mut self, service: impl Into<String>) -> Self {
        self.available_services
            .insert(KernelServiceId::new(service.into()));
        self
    }

    /// Add one available optional module.
    pub fn with_optional_module(mut self, service: impl Into<String>) -> Self {
        self.available_optional_modules
            .insert(KernelServiceId::new(service.into()));
        self
    }

    /// Add an entitlement state that the host can prove for paid packages.
    pub fn with_entitlement_state(mut self, state: EntitlementState) -> Self {
        self.entitlement_states.insert(state.to_string());
        self
    }
}
