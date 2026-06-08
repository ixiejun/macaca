//! Core application layer and lifecycle status value objects.
//!
//! These enums classify **how** an application executes (native/WASM/declarative)
//! and **where** it sits in the load/start lifecycle. They are provider-neutral
//! and reused across loader, runtime, and service projection surfaces.

use serde::{Deserialize, Serialize};

/// The execution layer of an application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppLayer {
    /// L1: native Rust agents compiled into the binary.
    L1Native,
    /// L2: WASM-based agents loaded as components.
    L2Wasm,
    /// L3: declarative agents loaded from YAML/TOML config files.
    L3Declarative,
}

/// Status of a loaded application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppStatus {
    /// App manifest has been loaded but agents are not yet started.
    Loaded,
    /// App agents are running.
    Running,
    /// App has been stopped.
    Stopped,
    /// App failed to start or encountered an error.
    Failed,
}

/// UI type for frontend rendering hints declared by an application manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UiType {
    /// Chat interface (default).
    Chat,
    /// Form-based interface.
    Form,
    /// Dashboard interface.
    Dashboard,
    /// Custom interface (frontend handles).
    Custom,
}

impl Default for UiType {
    fn default() -> Self {
        Self::Chat
    }
}
