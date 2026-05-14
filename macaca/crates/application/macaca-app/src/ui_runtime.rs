//! Application-owned UI runtime declarations.
//!
//! Macaca hosts application UI as an operating-system concern, but the
//! application owns its visual implementation. This module keeps the manifest
//! contract data-only: it records where a web bundle lives, which sandbox
//! strategy the shell must use, and which host bridge capabilities the UI may
//! request. Runtime hosts and presentation shells consume these values through
//! sanitized projections instead of branching on application-specific names or
//! business services.

use std::path::{Component, Path};

use macaca_proto::{MacacaError, MacacaResult};
use serde::{Deserialize, Serialize};

/// Manifest declaration for an application-owned UI bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppUiRuntimeConfig {
    /// UI runtime strategy. The first supported strategy is a static web bundle
    /// hosted in an iframe/WebView and connected through the Macaca bridge.
    pub runtime: AppUiRuntimeKind,
    /// Shell placement strategy for the loaded UI runtime.
    ///
    /// `runtime` answers how the UI is loaded. `surface` answers where that UI
    /// belongs in the host shell. Keeping the two axes separate lets Macaca
    /// route full product-like applications away from the chat workspace while
    /// keeping chat-first applications on the existing session shell.
    #[serde(default)]
    pub surface: AppUiSurfaceConfig,
    /// Developer-facing framework metadata. The host never switches behavior
    /// based on this value; it is for diagnostics, templates, and tooling.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub framework: Option<AppUiFramework>,
    /// Package-relative HTML entry point, for example `dist/ui/index.html`.
    pub entry: String,
    /// Package-relative asset allowlist. Globs are preserved as declarations;
    /// host routes still normalize every requested file before serving it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assets: Vec<String>,
    /// Sandboxing and network policy requested by the application.
    #[serde(default)]
    pub sandbox: AppUiSandboxConfig,
    /// Host bridge capabilities requested by the application-owned UI.
    #[serde(default)]
    pub bridge: AppUiBridgeConfig,
    /// Theme ownership declaration. The first slice treats this as metadata so
    /// app-owned brands are not constrained by the host shell.
    #[serde(default)]
    pub theme: AppUiThemeConfig,
}

/// Supported application-owned UI runtime kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppUiRuntimeKind {
    /// Static web bundle loaded by Web iframe or Desktop WebView hosts.
    WebBundle,
}

/// Framework metadata for diagnostics and scaffolding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppUiFramework {
    React,
    Vue,
    Svelte,
    Vanilla,
    Other,
}

/// Placement contract for an application-owned UI declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppUiSurfaceConfig {
    /// Workspace strategy requested by the application.
    #[serde(default)]
    pub mode: AppUiSurfaceMode,
    /// Chrome ownership requested by the application.
    #[serde(default)]
    pub chrome: AppUiSurfaceChrome,
}

impl Default for AppUiSurfaceConfig {
    fn default() -> Self {
        Self {
            mode: AppUiSurfaceMode::Session,
            chrome: AppUiSurfaceChrome::Host,
        }
    }
}

/// Supported workspace placement strategies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppUiSurfaceMode {
    /// The application owns the full workspace area. Host shells should keep
    /// global navigation but avoid rendering chat-thread widgets inside the
    /// app-owned workspace.
    Application,
    /// The host chat/session shell remains primary, and app UI may extend
    /// declared shell slots in later phases.
    Session,
}

impl Default for AppUiSurfaceMode {
    fn default() -> Self {
        Self::Session
    }
}

/// Supported workspace chrome ownership strategies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppUiSurfaceChrome {
    /// The application bundle owns workspace-local navigation, headers, and
    /// action chrome.
    AppOwned,
    /// The host shell owns workspace chrome, which is the compatibility default
    /// for chat-first applications.
    Host,
}

impl Default for AppUiSurfaceChrome {
    fn default() -> Self {
        Self::Host
    }
}

/// Sandboxing policy for a UI bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppUiSandboxConfig {
    /// Isolation container requested by the app. Web currently supports iframe.
    #[serde(default)]
    pub isolation: AppUiSandboxIsolation,
    /// Content Security Policy preset requested by the app.
    #[serde(default)]
    pub csp: AppUiCspMode,
    /// Network policy declaration consumed by admission and shell routes.
    #[serde(default)]
    pub network: AppUiNetworkPolicy,
}

impl Default for AppUiSandboxConfig {
    fn default() -> Self {
        Self {
            isolation: AppUiSandboxIsolation::Iframe,
            csp: AppUiCspMode::Strict,
            network: AppUiNetworkPolicy::Denied,
        }
    }
}

/// Supported UI isolation containers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppUiSandboxIsolation {
    Iframe,
}

impl Default for AppUiSandboxIsolation {
    fn default() -> Self {
        Self::Iframe
    }
}

/// CSP presets understood by Macaca shells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppUiCspMode {
    Strict,
}

impl Default for AppUiCspMode {
    fn default() -> Self {
        Self::Strict
    }
}

/// Network policy for application-owned UI bundles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppUiNetworkPolicy {
    Denied,
    Declared,
}

impl Default for AppUiNetworkPolicy {
    fn default() -> Self {
        Self::Denied
    }
}

/// Bridge capability declaration for a UI bundle.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppUiBridgeConfig {
    /// Capabilities required for the UI to function.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<String>,
    /// Capabilities that may improve the UI but are not required.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub optional: Vec<String>,
}

impl AppUiBridgeConfig {
    /// Return true when a capability was declared in either required or
    /// optional lists. This helper is string-based because bridge capabilities
    /// are stable protocol identifiers, not Rust enum variants.
    pub fn declares(&self, capability: &str) -> bool {
        self.required.iter().any(|item| item == capability)
            || self.optional.iter().any(|item| item == capability)
    }
}

/// Theme ownership declaration for app-owned UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppUiThemeConfig {
    #[serde(default)]
    pub mode: AppUiThemeMode,
}

impl Default for AppUiThemeConfig {
    fn default() -> Self {
        Self {
            mode: AppUiThemeMode::AppOwned,
        }
    }
}

/// Theme strategy declared by the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppUiThemeMode {
    AppOwned,
    HostAdaptive,
}

impl Default for AppUiThemeMode {
    fn default() -> Self {
        Self::AppOwned
    }
}

/// Validate one optional UI runtime declaration during admission.
pub fn validate_ui_runtime_config(config: Option<&AppUiRuntimeConfig>) -> MacacaResult<()> {
    let Some(config) = config else {
        return Ok(());
    };
    validate_package_relative_path("ui.entry", &config.entry)?;
    for asset in &config.assets {
        validate_package_relative_path("ui.assets", asset.trim_end_matches("/**"))?;
    }
    for capability in config
        .bridge
        .required
        .iter()
        .chain(config.bridge.optional.iter())
    {
        if capability.trim().is_empty() {
            return Err(MacacaError::Config(
                "ui.bridge contains an empty capability identifier".into(),
            ));
        }
    }
    tracing::info!(
        ui_runtime = ?config.runtime,
        framework = ?config.framework,
        entry = %config.entry,
        required_bridge_count = config.bridge.required.len(),
        optional_bridge_count = config.bridge.optional.len(),
        "application-owned UI runtime declaration admitted"
    );
    Ok(())
}

/// Validate that a manifest path stays inside the application package.
///
/// The check is intentionally lexical rather than filesystem based so it can
/// run before installation and before the package directory exists on disk.
fn validate_package_relative_path(field: &str, value: &str) -> MacacaResult<()> {
    if value.trim().is_empty() {
        return Err(MacacaError::Config(format!("{field} must not be empty")));
    }
    let path = Path::new(value);
    if path.is_absolute() {
        return Err(MacacaError::Config(format!(
            "{field} must be package-relative"
        )));
    }
    for component in path.components() {
        if matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        ) {
            return Err(MacacaError::Config(format!(
                "{field} must not escape the application package"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_config() -> AppUiRuntimeConfig {
        AppUiRuntimeConfig {
            runtime: AppUiRuntimeKind::WebBundle,
            surface: AppUiSurfaceConfig::default(),
            framework: Some(AppUiFramework::React),
            entry: "dist/ui/index.html".into(),
            assets: vec!["dist/ui/assets/**".into()],
            sandbox: AppUiSandboxConfig::default(),
            bridge: AppUiBridgeConfig {
                required: vec!["service.call".into()],
                optional: vec!["trace.emit".into()],
            },
            theme: AppUiThemeConfig::default(),
        }
    }

    #[test]
    fn ui_runtime_accepts_package_relative_entry_and_bridge_capabilities() {
        validate_ui_runtime_config(Some(&valid_config())).unwrap();
    }

    #[test]
    fn ui_runtime_rejects_entry_path_escape() {
        let mut config = valid_config();
        config.entry = "../escape.html".into();
        let error = validate_ui_runtime_config(Some(&config)).unwrap_err();
        assert!(error.to_string().contains("package"));
    }

    #[test]
    fn ui_runtime_rejects_empty_bridge_capability() {
        let mut config = valid_config();
        config.bridge.required.push(" ".into());
        let error = validate_ui_runtime_config(Some(&config)).unwrap_err();
        assert!(error.to_string().contains("bridge"));
    }

    #[test]
    fn ui_runtime_defaults_to_session_surface_for_compatibility() {
        let yaml = r#"
runtime: web_bundle
entry: dist/ui/index.html
"#;
        let config: AppUiRuntimeConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.surface.mode, AppUiSurfaceMode::Session);
        assert_eq!(config.surface.chrome, AppUiSurfaceChrome::Host);
    }

    #[test]
    fn ui_runtime_accepts_application_surface_declaration() {
        let yaml = r#"
runtime: web_bundle
surface:
  mode: application
  chrome: app_owned
entry: dist/ui/index.html
"#;
        let config: AppUiRuntimeConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.surface.mode, AppUiSurfaceMode::Application);
        assert_eq!(config.surface.chrome, AppUiSurfaceChrome::AppOwned);
        validate_ui_runtime_config(Some(&config)).unwrap();
    }
}
