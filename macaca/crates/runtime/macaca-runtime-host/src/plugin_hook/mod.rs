//! Runtime-host Plugin Hook Bus v1.
//!
//! This module owns the Hook Bus control and dispatch surface.  It does not run
//! arbitrary plugin code in v1.  Instead it stores typed hook descriptors,
//! orders them deterministically, applies timeout/failure strategies, emits
//! traceable audit events, and returns structured no-op or unavailable outcomes
//! until a later execution-plane proposal installs safe host proxies.

mod policy;
mod registry;
mod runner;
mod service;

#[cfg(test)]
mod plugin_hook_tests;

pub use policy::{
    DefaultPluginHookFailureStrategy, DefaultPluginHookTimeoutStrategy, PluginHookFailureStrategy,
    PluginHookTimeoutStrategy,
};
pub use registry::{PluginHookRegistry, PluginHookRegistryBuilder};
pub use runner::{DescriptorOnlyPluginHookExecutor, PluginHookExecutor, PluginHookRunner};
pub use service::{PluginHookBus, PluginHookBusBuilder};
