//! Application model types (**Aggregate Root** module tree).
//!
//! This module owns provider-neutral DTOs for application manifests, agents,
//! workflows, capabilities, and runtime load state. The tree applies domain-driven
//! decomposition so each file stays under the 500-line constitution while the
//! public `macaca_app::model::*` surface remains stable via Facade re-exports.
//!
//! # Module tree (P5 iteration 98)
//! - `core` — layer/status/ui enums
//! - `agent_config` — inline agent and capability reference DTOs
//! - `autonomy` — heartbeat/autonomy declaration blocks
//! - `capability` — Composite capability tree + flatten adapter
//! - `manifest` — `AppManifest` aggregate root
//! - `workflow` — entrypoint and workflow graph DTOs
//! - `loaded` — runtime loaded-application read model
//! - `tests` — serde and capability contract tests

mod agent_config;
mod autonomy;
mod capability;
mod core;
mod loaded;
mod manifest;
mod workflow;

#[cfg(test)]
mod tests;

pub use agent_config::{
    AgentSkillsConfig, AgentSource, AppContextConfig, AppLlmConfig, CapabilityRef,
    InlineAgentConfig,
};
pub use autonomy::{
    AppAutonomyConfig, AppHeartbeatAgentConfig, AppHeartbeatCadenceConfig, AppHeartbeatConfig,
    AppHeartbeatGateConfig,
};
pub use capability::{AppCapabilityNode, AppCapabilitySet, AppCapabilitySource};
pub use core::{AppLayer, AppStatus, UiType};
pub use loaded::LoadedApp;
pub use manifest::AppManifest;
pub use workflow::{
    EntrypointConfig, EntrypointType, ResourceConfig, WorkflowDefinition, WorkflowStep,
};
