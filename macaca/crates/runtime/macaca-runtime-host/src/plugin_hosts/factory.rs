//! Abstract Factory for plugin host skeletons.
//!
//! The factory branches only on protocol runtime kind. It never looks at
//! provider, application, workflow, gateway, driver, chain, model, or business
//! names, which keeps the host selection generic for all Macaca applications.

use std::sync::Arc;

use macaca_proto::{PluginHostDescriptor, PluginRuntimeKind};
use tracing::{info, warn};

use super::hosts::{
    BuiltInAdapterPluginRuntimeHost, DescriptorPluginRuntimeHost, PluginHostRuntime,
    ProcessPluginRuntimeHost, RemoteProxyPluginRuntimeHost, UnavailablePluginRuntimeHost,
    WasmPluginRuntimeHost,
};

/// Abstract Factory that creates host strategies from runtime descriptors.
#[derive(Debug, Default, Clone)]
pub struct PluginHostRuntimeFactory;

impl PluginHostRuntimeFactory {
    /// Create a host strategy from a complete host descriptor.
    pub fn create(&self, descriptor: &PluginHostDescriptor) -> Arc<dyn PluginHostRuntime> {
        self.create_for_kind(&descriptor.runtime_kind)
    }

    /// Create a host strategy from a runtime kind when manifest context is not needed.
    pub fn create_for_kind(&self, kind: &PluginRuntimeKind) -> Arc<dyn PluginHostRuntime> {
        match kind {
            PluginRuntimeKind::DescriptorOnly => {
                info!(runtime_kind = %kind, "descriptor plugin runtime host selected");
                Arc::new(DescriptorPluginRuntimeHost)
            }
            PluginRuntimeKind::BuiltInAdapter => {
                info!(runtime_kind = %kind, "built-in adapter plugin runtime host selected");
                Arc::new(BuiltInAdapterPluginRuntimeHost)
            }
            PluginRuntimeKind::Wasm => {
                warn!(runtime_kind = %kind, "WASM host skeleton selected as unavailable-safe");
                Arc::new(WasmPluginRuntimeHost)
            }
            PluginRuntimeKind::Process => {
                warn!(runtime_kind = %kind, "process host skeleton selected as unavailable-safe");
                Arc::new(ProcessPluginRuntimeHost)
            }
            PluginRuntimeKind::RemoteProxy => {
                warn!(runtime_kind = %kind, "remote proxy host skeleton selected as unavailable-safe");
                Arc::new(RemoteProxyPluginRuntimeHost)
            }
            PluginRuntimeKind::Native | PluginRuntimeKind::Custom(_) => {
                Arc::new(UnavailablePluginRuntimeHost::new(
                    kind.clone(),
                    "native/custom plugin execution is not enabled in host skeleton v1",
                ))
            }
        }
    }
}
