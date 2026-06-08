//! Application-owned UI runtime metadata projection (Strategy label mappers).
//!
//! Maps strongly-typed UI runtime enums into provider-neutral string labels for
//! service DTO transport.  Logs projection audit nodes with app_id only.

use macaca_proto::{
    ApplicationUiBridgeView, ApplicationUiRuntimeView, ApplicationUiSandboxView,
    ApplicationUiSurfaceView, ApplicationUiThemeView,
};
use tracing::info;

use crate::model::AppManifest;
use crate::ui_runtime::{
    AppUiCspMode, AppUiFramework, AppUiNetworkPolicy, AppUiRuntimeKind, AppUiSandboxIsolation,
    AppUiSurfaceChrome, AppUiSurfaceMode, AppUiThemeMode,
};


pub(super) fn ui_runtime_view(legacy: &AppManifest) -> Option<ApplicationUiRuntimeView> {
    let ui = legacy.ui.as_ref()?;
    let entry_url = ui
        .entry
        .as_ref()
        .map(|entry| format!("/api/apps/{}/ui/assets/{entry}", legacy.id))
        .unwrap_or_default();
    tracing::info!(
        app_id = %legacy.id,
        ui_runtime = %ui_runtime_kind_label(ui.runtime),
        surface_mode = %ui_surface_mode_label(ui.surface.mode),
        surface_chrome = %ui_surface_chrome_label(ui.surface.chrome),
        bridge_required = ui.bridge.required.len(),
        bridge_optional = ui.bridge.optional.len(),
        "projected sanitized application-owned UI metadata"
    );
    Some(ApplicationUiRuntimeView {
        runtime: ui_runtime_kind_label(ui.runtime).to_string(),
        surface: ApplicationUiSurfaceView {
            mode: ui_surface_mode_label(ui.surface.mode).to_string(),
            chrome: ui_surface_chrome_label(ui.surface.chrome).to_string(),
        },
        framework: ui.framework.map(ui_framework_label).map(str::to_string),
        entry_url,
        sandbox: ApplicationUiSandboxView {
            isolation: ui_sandbox_isolation_label(ui.sandbox.isolation).to_string(),
            csp: ui_csp_label(ui.sandbox.csp).to_string(),
            network: ui_network_label(ui.sandbox.network).to_string(),
        },
        bridge: ApplicationUiBridgeView {
            required: ui.bridge.required.clone(),
            optional: ui.bridge.optional.clone(),
        },
        theme: ApplicationUiThemeView {
            mode: ui_theme_label(ui.theme.mode).to_string(),
        },
    })
}

fn ui_runtime_kind_label(kind: AppUiRuntimeKind) -> &'static str {
    match kind {
        AppUiRuntimeKind::WebBundle => "web_bundle",
        AppUiRuntimeKind::BuiltinKit => "builtin_kit",
    }
}

fn ui_framework_label(framework: AppUiFramework) -> &'static str {
    match framework {
        AppUiFramework::React => "react",
        AppUiFramework::Vue => "vue",
        AppUiFramework::Svelte => "svelte",
        AppUiFramework::Vanilla => "vanilla",
        AppUiFramework::Other => "other",
    }
}

fn ui_surface_mode_label(mode: AppUiSurfaceMode) -> &'static str {
    match mode {
        AppUiSurfaceMode::Application => "application",
        AppUiSurfaceMode::Session => "session",
    }
}

fn ui_surface_chrome_label(chrome: AppUiSurfaceChrome) -> &'static str {
    match chrome {
        AppUiSurfaceChrome::AppOwned => "app_owned",
        AppUiSurfaceChrome::Host => "host",
    }
}

fn ui_sandbox_isolation_label(isolation: AppUiSandboxIsolation) -> &'static str {
    match isolation {
        AppUiSandboxIsolation::Iframe => "iframe",
    }
}

fn ui_csp_label(csp: AppUiCspMode) -> &'static str {
    match csp {
        AppUiCspMode::Strict => "strict",
    }
}

fn ui_network_label(network: AppUiNetworkPolicy) -> &'static str {
    match network {
        AppUiNetworkPolicy::Denied => "denied",
        AppUiNetworkPolicy::Declared => "declared",
    }
}

fn ui_theme_label(mode: AppUiThemeMode) -> &'static str {
    match mode {
        AppUiThemeMode::AppOwned => "app_owned",
        AppUiThemeMode::HostAdaptive => "host_adaptive",
    }
}

