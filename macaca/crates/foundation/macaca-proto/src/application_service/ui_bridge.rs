//! Application-owned UI runtime and host bridge DTOs (DTO + Specification).
//!
//! UI views describe manifest-declared presentation surfaces in a shell-safe form.
//! Bridge request/response envelopes let application UIs call host capabilities
//! without importing framework, domain, or runtime internals.

use serde::{Deserialize, Serialize};

use crate::ApplicationHostCommandResult;

/// Sanitized application-owned UI runtime view.
///
/// This DTO is intentionally presentation-shell safe.  It carries only bounded
/// manifest declarations and host URLs that Web/Desktop shells can interpret
/// generically.  It never exposes raw host filesystem handles, secrets,
/// provider configuration, prompt text, or domain-specific service knowledge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationUiRuntimeView {
    pub runtime: String,
    pub surface: ApplicationUiSurfaceView,
    pub framework: Option<String>,
    pub entry_url: String,
    pub sandbox: ApplicationUiSandboxView,
    pub bridge: ApplicationUiBridgeView,
    pub theme: ApplicationUiThemeView,
}

/// Sanitized placement metadata for an application-owned UI surface.
///
/// Shells use this strategy value to decide whether the loaded UI replaces the
/// workspace or augments the existing chat/session shell. It is intentionally a
/// small string DTO so Web and Desktop hosts share the same protocol without
/// importing application-manifest internals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationUiSurfaceView {
    pub mode: String,
    pub chrome: String,
}

/// Bounded sandbox metadata for an application-owned UI surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationUiSandboxView {
    pub isolation: String,
    pub csp: String,
    pub network: String,
}

/// Declared bridge capabilities for an application-owned UI surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationUiBridgeView {
    pub required: Vec<String>,
    pub optional: Vec<String>,
}

impl ApplicationUiBridgeView {
    /// Check whether a capability was explicitly declared by the application.
    ///
    /// The bridge route uses this fail-closed helper before dispatching any
    /// request to host services.  Keeping it on the DTO makes Web/Desktop
    /// shells share the same policy vocabulary without app-specific branches.
    pub fn declares(&self, capability: &str) -> bool {
        self.required.iter().any(|item| item == capability)
            || self.optional.iter().any(|item| item == capability)
    }
}

/// Theme ownership metadata for application-owned UI surfaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationUiThemeView {
    pub mode: String,
}

/// Generic command sent by an application-owned UI surface to the host bridge.
///
/// The shape is deliberately capability-oriented instead of framework,
/// application, or domain oriented.  React, Vue, Svelte, WebView, and future
/// desktop shells can all send the same envelope while policy and routing stay
/// centralized in the host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationUiBridgeRequest {
    pub bridge_version: String,
    pub session_id: Option<String>,
    pub surface_id: Option<String>,
    pub trace_id: Option<String>,
    pub command_id: String,
    pub capability: String,
    pub service_id: Option<String>,
    pub operation: Option<String>,
    pub payload: serde_json::Value,
}

/// Generic bridge response returned to application-owned UI surfaces.
///
/// Results are transported as data so the host can preserve trace, policy, and
/// audit metadata without giving UI bundles direct access to runtime internals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationUiBridgeResponse {
    pub bridge_version: String,
    pub command_id: String,
    pub accepted: bool,
    pub result: ApplicationHostCommandResult,
}
