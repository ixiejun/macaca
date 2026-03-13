//! `aos-driver` — Software Driver framework for Agent OS.
//!
//! Provides a pluggable abstraction for controlling external software.
//! Drivers expose their capabilities as `Tool` instances, integrating
//! seamlessly with the Agent OS tool system.

pub mod driver;
pub mod registry;
pub mod toolset;
pub mod builtin;

pub use driver::{SoftwareDriver, DriverManifest, DriverType};
pub use registry::DriverRegistry;
pub use toolset::DriverToolSet;
