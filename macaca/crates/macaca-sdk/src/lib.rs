//! `aos-sdk` — declarative agent SDK for Agent OS.
//!
//! Provides YAML/TOML configuration parsing, a fluent builder API,
//! and registration helpers to construct and register agents with
//! the kernel from declarative config files.

pub mod builder;
pub mod config;
pub mod persona;
pub mod registry_api;

pub use builder::{AgentBuilder, DeclarativeAgent};
pub use config::AgentConfig;
pub use persona::AgentPersona;
pub use registry_api::{register_from_config, register_from_file};
