//! `aos-driver` — Software Driver framework for Agent OS.
//!
//! Provides a pluggable abstraction for controlling external software.
//! Drivers expose their capabilities as `Tool` instances, integrating
//! seamlessly with the Agent OS tool system.

pub mod builtin;
pub mod command;
pub mod driver;
pub mod dynamic_driver;
mod dynamic_proxy;
pub mod factory;
pub mod load_command;
pub mod loader;
pub mod plugin_abi;
pub mod registry;
pub mod runtime;
pub mod sdk;
pub mod service_adapter;
pub mod session;
pub mod toolset;
pub mod trace;

pub use command::DriverCommand;
pub use driver::{DriverManifest, DriverType, SoftwareDriver};
pub use dynamic_driver::DynamicDriver;
pub use factory::{DriverCreateContext, DriverFactory, DynamicDriverFactory};
pub use load_command::{DriverLoadCommand, DriverLoadEntry, DriverLoadReport, DriverLoadStatus};
pub use loader::DriverLoader;
pub use macaca_tools::{
    CompositeToolSet, ToolCatalog, ToolCommand, ToolCommandContext, ToolCommandExecutor,
    ToolSchemaProvider, TraceEvent,
};
pub use registry::DriverRegistry;
pub use runtime::{DriverInventoryItem, DriverRuntime};
pub use service_adapter::driver_service_descriptor;
pub use session::DriverSessionState;
pub use trace::DriverTraceAdapter;

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
