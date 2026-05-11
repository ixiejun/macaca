//! Plugin SDK / ABI / Hosts v1 runtime-host boundary.
//!
//! This module is the execution-plane seam for Plugin Fabric. It exposes a
//! provider-neutral host trait, an Abstract Factory, and a lifecycle supervisor
//! while keeping all unapproved execution runtimes unavailable-safe. Descriptor
//! and built-in adapter hosts accept lifecycle metadata; WASM, process, native,
//! remote proxy, and custom hosts return structured unavailable results instead
//! of executing bytes, spawning commands, loading libraries, or opening network
//! transports.

mod factory;
mod hosts;
mod supervisor;

pub use factory::PluginHostRuntimeFactory;
pub use hosts::{
    BuiltInAdapterPluginRuntimeHost, DescriptorPluginRuntimeHost, PluginHostRuntime,
    PluginHostRuntimeResult, ProcessPluginRuntimeHost, RemoteProxyPluginRuntimeHost,
    UnavailablePluginRuntimeHost, WasmPluginRuntimeHost,
};
pub use supervisor::PluginHostLifecycleSupervisor;

#[cfg(test)]
mod plugin_hosts_tests;
