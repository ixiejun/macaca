//! `aos-driver` — Software Driver framework for Agent OS.
//!
//! Provides a pluggable abstraction for controlling external software.
//! Drivers expose their capabilities as `Tool` instances, integrating
//! seamlessly with the Agent OS tool system.

pub mod builtin;
pub mod driver;
pub mod dynamic_driver;
pub mod loader;
pub mod plugin_abi;
pub mod registry;
pub mod sdk;
pub mod toolset;

pub use driver::{DriverManifest, DriverType, SoftwareDriver};
pub use dynamic_driver::DynamicDriver;
pub use loader::DriverLoader;
pub use macaca_tools::{
    CompositeToolSet, ToolCatalog, ToolCommand, ToolCommandContext, ToolCommandExecutor,
    ToolSchemaProvider, TraceEvent,
};
pub use registry::DriverRegistry;

/// A generic `Send` wrapper for closures that capture FFI raw pointers.
///
/// # Safety
///
/// The caller **must** guarantee that the wrapped value is only accessed from
/// a single thread at a time and that all captured pointers remain valid for
/// the lifetime of the wrapper (e.g. by joining the spawned thread before the
/// pointers are invalidated).
pub struct SendableFn<F>(pub F);
unsafe impl<F> Send for SendableFn<F> {}
impl<F: FnOnce()> SendableFn<F> {
    /// Consume the wrapper and call the inner closure.
    pub fn call(self) {
        (self.0)()
    }
}
pub use toolset::DriverToolSet;
