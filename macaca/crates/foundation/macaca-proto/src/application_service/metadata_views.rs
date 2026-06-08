//! Sanitized application metadata policy view DTOs.
//!
//! Metadata views expose declaration summaries (counts, names, presence flags)
//! without shipping raw manifest bodies, prompt templates, or credential data.

use serde::{Deserialize, Serialize};

use crate::ExecutionControlPolicy;

use super::app_views::ApplicationServiceAppView;

/// Sanitized ability view returned by Application metadata queries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationAbilityMetadataView {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub implementation: String,
    pub is_entry: bool,
    pub activation_modes: Vec<String>,
    pub capability_names: Vec<String>,
    pub required_services: Vec<String>,
    pub permission_names: Vec<String>,
}

/// Sanitized entry metadata for shells that need routing hints.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationEntryMetadataView {
    pub agent_name: Option<String>,
    pub entry_kind: Option<String>,
    pub activation_mode: Option<String>,
}

/// Sanitized tool policy metadata.  It exposes declared names and counts only.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationToolPolicyMetadataView {
    pub declared_tool_names: Vec<String>,
    pub execution_tool_count: usize,
}

/// Sanitized context policy metadata.  It reports presence, not config bodies.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationContextPolicyMetadataView {
    pub context_config_present: bool,
    pub context_engine_declared: bool,
}

/// Sanitized AgentSkills metadata.  It reports policy presence and agent names.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationSkillPolicyMetadataView {
    pub agents_with_skill_policy: Vec<String>,
}

/// Sanitized MCP overlay metadata.  It reports overlay presence only.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationMcpOverlayMetadataView {
    pub overlay_declared: bool,
    pub agents_with_overlay: Vec<String>,
}

/// Sanitized Application Framework workbench declaration metadata.
///
/// The view is intentionally ref/count oriented.  It lets SDK clients, shells,
/// runtime-host adapters, context/tool planning, and app protocol subscribers
/// learn what the application declared without receiving raw manifests, prompt
/// bodies, plugin packages, MCP definitions, skill contents, or credentials.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationWorkbenchMetadataView {
    pub declared_capabilities: Vec<String>,
    pub permission_profiles: Vec<String>,
    pub tool_families: Vec<String>,
    pub service_dependencies: Vec<String>,
    pub optional_provider_requirements: Vec<String>,
    pub plugin_dependencies: Vec<String>,
    pub mcp_dependencies: Vec<String>,
    pub skill_bundles: Vec<String>,
    pub event_subscriptions: Vec<String>,
    pub ui_surfaces: Vec<String>,
}

/// Manifest digest metadata for cache keys and audit without raw manifest data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationManifestDigestView {
    pub algorithm: String,
    pub digest: String,
    pub source_format: String,
    pub ability_count: usize,
    pub agent_count: usize,
}

/// Complete sanitized metadata view for Web, CLI, Gateway, and adapters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationMetadataView {
    pub application: ApplicationServiceAppView,
    pub entry: ApplicationEntryMetadataView,
    pub abilities: Vec<ApplicationAbilityMetadataView>,
    pub tool_policy: ApplicationToolPolicyMetadataView,
    pub context_policy: ApplicationContextPolicyMetadataView,
    pub skill_policy: ApplicationSkillPolicyMetadataView,
    pub mcp_overlay: ApplicationMcpOverlayMetadataView,
    pub workbench: ApplicationWorkbenchMetadataView,
    /// Manifest-declared execution-control default policy projected for shells
    /// and runtime adapters. Absent when the application has not opted in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_control: Option<ExecutionControlPolicy>,
    pub manifest_digest: Option<ApplicationManifestDigestView>,
    pub diagnostics: Vec<String>,
}
