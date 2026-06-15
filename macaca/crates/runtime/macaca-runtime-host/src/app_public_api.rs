//! Explicit application framework surface for SDK composition roots.
//!
//! The application crate owns manifest loading, application runtime metadata,
//! and app-owned UI DTOs.  Runtime-host re-exports only the SDK-published
//! surface so the SDK can avoid a direct application-crate dependency.

pub use macaca_app::model::{AgentSource, AppContextConfig};
pub use macaca_app::ui_runtime::{
    validate_ui_runtime_config, AppUiBridgeConfig, AppUiCspMode, AppUiFramework,
    AppUiNetworkPolicy, AppUiPresentationConfig, AppUiRuntimeConfig, AppUiRuntimeKind,
    AppUiSandboxConfig, AppUiSandboxIsolation, AppUiSurfaceChrome, AppUiSurfaceConfig,
    AppUiSurfaceMode, AppUiThemeConfig, AppUiThemeMode,
};
pub use macaca_app::{
    app_agent_manifest_view, app_agent_prompt_semantics, app_entry_agent_name,
    app_task_planning_contract, application_service_descriptor, discovered_app_agent_names,
    expand_service_capabilities, AppLayer, AppLlmConfig, AppLoader, AppRegistry, AppRuntime,
    DiscoveredApp, SharedDomainPackCatalog,
};
