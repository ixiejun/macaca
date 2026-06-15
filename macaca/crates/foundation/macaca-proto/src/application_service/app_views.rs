//! Sanitized application, agent, runtime, and session view DTOs.
//!
//! Views are read-only projections returned by Application Service providers.
//! They intentionally omit secrets, raw manifests, and host handles so SDK,
//! Web, CLI, and diagnostics consumers share one safe vocabulary.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{ApplicationId, ApplicationLifecycleState, PackageRuntimeKind};

use super::ui_bridge::ApplicationUiRuntimeView;

/// Sanitized heartbeat agent declaration returned by Application Service.
///
/// This view intentionally carries declaration metadata only. It must never
/// contain raw manifest text, prompt templates, HEARTBEAT.md content, secrets,
/// provider payloads, package bytes, or host filesystem handles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationHeartbeatAgentView {
    pub application_id: ApplicationId,
    pub agent_name: String,
    pub enabled: bool,
    /// Manifest-level profile selector kept for admission and traceability.
    pub profile_id: String,
    /// Concrete native Heartbeat profile id registered for this declaration.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub native_profile_id: String,
    /// Concrete wake scope key used by service.heartbeat for this agent.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub wake_scope_key: String,
    /// Optional manifest-declared fixed interval for this agent in seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed_interval_secs: Option<u64>,
    /// Optional manifest-declared cooldown for this agent in seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cooldown_secs: Option<u64>,
    pub metadata: BTreeMap<String, String>,
    pub diagnostics: Vec<String>,
}

/// Sanitized agent view returned by Application Service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationServiceAgentView {
    pub name: String,
    pub capability_names: Vec<String>,
}

/// Sanitized application runtime view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationServiceRuntimeView {
    pub runtime_kind: Option<PackageRuntimeKind>,
    pub lifecycle_state: ApplicationLifecycleState,
    pub runtime_status: String,
    pub app_dir: Option<String>,
    pub skills_dir: Option<String>,
}

/// Sanitized application view used by Web, CLI, and diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationServiceAppView {
    pub id: ApplicationId,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub entry_agent: Option<String>,
    pub agents: Vec<ApplicationServiceAgentView>,
    pub runtime: ApplicationServiceRuntimeView,
    pub ui: Option<ApplicationUiRuntimeView>,
    pub diagnostics: Vec<String>,
}

/// Session envelope tracked by Application Service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationServiceSessionView {
    pub application_id: ApplicationId,
    pub session_id: String,
    pub entry_agent: Option<String>,
    pub status: String,
    pub updated_at: DateTime<Utc>,
}

impl ApplicationServiceSessionView {
    /// Build a new session status view without capturing prompt content.
    pub fn new(
        application_id: ApplicationId,
        session_id: impl Into<String>,
        entry_agent: Option<String>,
        status: impl Into<String>,
    ) -> Self {
        Self {
            application_id,
            session_id: session_id.into(),
            entry_agent,
            status: status.into(),
            updated_at: Utc::now(),
        }
    }
}
