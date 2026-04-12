//! `aos-driver` — Software Driver framework for Agent OS.
//!
//! Provides a pluggable abstraction for controlling external software.
//! Drivers expose their capabilities as `Tool` instances, integrating
//! seamlessly with the Agent OS tool system.

pub mod builtin;
pub mod driver;
pub mod registry;
pub mod toolset;

pub use driver::{DriverManifest, DriverType, SoftwareDriver};
pub use registry::DriverRegistry;
pub use toolset::DriverToolSet;
