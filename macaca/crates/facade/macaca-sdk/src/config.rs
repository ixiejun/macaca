//! Declarative agent configuration — SDK re-export of the proto foundation contract.
//!
//! The canonical types and parsers live in `macaca_proto::declarative_agent_config` so
//! `macaca-app` can load manifests without depending on this facade crate. The SDK
//! preserves the historical `macaca_sdk::AgentConfig` import path for shells and tools.

pub use macaca_proto::{
    agent_manifest_from_declarative_config, AgentConfig, AgentSkillsConfig, CapabilityDef,
    DeclarativeAgentConfigValidation, DeclarativeAgentConfigValidator,
};
