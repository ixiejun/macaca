//! Provider-neutral contracts for `service.plugin_marketplace`.
//!
//! The marketplace service owns source admission, entitlement/license/metering
//! gates, plugin lifecycle orchestration, bundled capability summaries, rollback
//! mementos, and sanitized audit records.  It never embeds application-specific
//! behavior and never serializes package bytes, credentials, raw signatures, or
//! executable plugin payloads.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    PluginControlRecord, PluginId, PluginInstallRequest, PluginInstallSourceKind, PluginManifest,
    PluginVersion,
};

use super::WorkbenchServiceDescriptor;

pub const SERVICE_ID: &str = "service.plugin_marketplace";
pub const CAPABILITY_ID: &str = "capability.plugin_marketplace";
pub const MARKETPLACE_ADD_COMMAND: &str = "marketplace.add";
pub const MARKETPLACE_REMOVE_COMMAND: &str = "marketplace.remove";
pub const MARKETPLACE_UPGRADE_COMMAND: &str = "marketplace.upgrade";
pub const PLUGIN_LIST_COMMAND: &str = "plugin.list";
pub const PLUGIN_READ_COMMAND: &str = "plugin.read";
pub const PLUGIN_INSTALL_COMMAND: &str = "plugin.install";
pub const PLUGIN_UNINSTALL_COMMAND: &str = "plugin.uninstall";
pub const PLUGIN_ENABLE_COMMAND: &str = "plugin.enable";
pub const PLUGIN_DISABLE_COMMAND: &str = "plugin.disable";
pub const PLUGIN_AUTH_STATUS_COMMAND: &str = "plugin.auth.status";
pub const SNAPSHOT_COMMAND: &str = "service.snapshot";

/// Compatibility command names emitted by the first descriptor skeleton.
pub const LEGACY_MARKETPLACE_ADD_COMMAND: &str = "plugin_marketplace.add";
pub const LEGACY_MARKETPLACE_REMOVE_COMMAND: &str = "plugin_marketplace.remove";

pub const COMMANDS: &[&str] = &[
    MARKETPLACE_ADD_COMMAND,
    MARKETPLACE_REMOVE_COMMAND,
    MARKETPLACE_UPGRADE_COMMAND,
    LEGACY_MARKETPLACE_ADD_COMMAND,
    LEGACY_MARKETPLACE_REMOVE_COMMAND,
    PLUGIN_LIST_COMMAND,
    PLUGIN_READ_COMMAND,
    PLUGIN_INSTALL_COMMAND,
    PLUGIN_UNINSTALL_COMMAND,
    PLUGIN_ENABLE_COMMAND,
    PLUGIN_DISABLE_COMMAND,
    PLUGIN_AUTH_STATUS_COMMAND,
    SNAPSHOT_COMMAND,
];

/// Marketplace source class.  The enum models Adapter boundaries without
/// binding the OS to one concrete store, enterprise catalog, or local layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarketplaceSourceKind {
    Local,
    Remote,
    EnterpriseManaged,
    Store,
    Developer,
}

/// Declarative marketplace descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketplaceDescriptor {
    pub marketplace_id: String,
    pub kind: MarketplaceSourceKind,
    pub enabled: bool,
    pub source_policy: MarketplaceSourcePolicy,
    pub metadata: BTreeMap<String, String>,
}

/// Executable source policy evaluated before plugin installation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketplaceSourcePolicy {
    pub allowed_source_kinds: Vec<PluginInstallSourceKind>,
    pub require_signature: bool,
    pub require_entitlement: bool,
    pub require_license: bool,
    pub require_metering: bool,
    pub disabled_by_admin: bool,
}

impl MarketplaceSourcePolicy {
    /// Build a fail-closed default policy for local/test marketplace providers.
    pub fn local_default() -> Self {
        Self {
            allowed_source_kinds: vec![
                PluginInstallSourceKind::Bundled,
                PluginInstallSourceKind::UserLocal,
                PluginInstallSourceKind::StoreCache,
            ],
            require_signature: true,
            require_entitlement: true,
            require_license: true,
            require_metering: true,
            disabled_by_admin: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketplaceCommand {
    pub descriptor: MarketplaceDescriptor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketplaceUpgradeCommand {
    pub marketplace_id: String,
    pub descriptor: MarketplaceDescriptor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketplaceRemoveCommand {
    pub marketplace_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginMarketplaceListCommand {
    pub include_disabled: bool,
    pub marketplace_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginMarketplaceReadCommand {
    pub plugin_id: PluginId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginMarketplaceInstallCommand {
    pub marketplace_id: String,
    pub request: PluginInstallRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginMarketplaceAuthStatusCommand {
    pub marketplace_id: Option<String>,
    pub plugin_id: Option<PluginId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginMarketplaceSnapshotCommand {
    pub include_audit_tail: bool,
}

/// Capability categories summarized from one plugin manifest.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginBundleCapabilityKind {
    Service,
    Tool,
    Skill,
    McpServer,
    Hook,
    Application,
    AppUi,
    Capability,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginBundleCapabilitySummary {
    pub kind: PluginBundleCapabilityKind,
    pub count: usize,
    pub refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketplaceGateDecision {
    pub gate: String,
    pub status: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginMarketplaceAuditRecord {
    pub audit_ref: String,
    pub operation: String,
    pub marketplace_id: Option<String>,
    pub plugin_id: Option<PluginId>,
    pub version: Option<PluginVersion>,
    pub gate_decisions: Vec<MarketplaceGateDecision>,
    pub capability_summary: Vec<PluginBundleCapabilitySummary>,
    pub trace_id: String,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginRollbackRecord {
    pub rollback_ref: String,
    pub plugin_id: PluginId,
    pub previous_version: Option<PluginVersion>,
    pub installed_version: Option<PluginVersion>,
    pub operation: String,
    pub cleanup_refs: Vec<String>,
    pub trace_id: String,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginAuthStatus {
    pub marketplace_id: Option<String>,
    pub plugin_id: Option<PluginId>,
    pub entitlement: String,
    pub license: String,
    pub metering: String,
    pub auth_required: bool,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginMarketplaceSnapshot {
    pub service_id: String,
    pub healthy: bool,
    pub marketplaces: Vec<MarketplaceDescriptor>,
    pub installed_plugins: Vec<PluginControlRecord>,
    pub rollback_records: Vec<PluginRollbackRecord>,
    pub audit_tail: Vec<PluginMarketplaceAuditRecord>,
    pub captured_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginMarketplaceResponse {
    Marketplace(MarketplaceDescriptor),
    Marketplaces(Vec<MarketplaceDescriptor>),
    PluginList(Vec<PluginControlRecord>),
    PluginRead(Option<PluginControlRecord>),
    PluginState(PluginControlRecord),
    Installed {
        record: PluginControlRecord,
        audit_ref: String,
        rollback_ref: String,
        capability_summary: Vec<PluginBundleCapabilitySummary>,
    },
    Uninstalled {
        event_ref: String,
        rollback_ref: String,
        cleanup_refs: Vec<String>,
    },
    AuthStatus(PluginAuthStatus),
    Snapshot(PluginMarketplaceSnapshot),
    Audit(PluginMarketplaceAuditRecord),
}

/// Build a bundled capability summary without exposing raw manifest payloads.
pub fn summarize_manifest_capabilities(
    manifest: &PluginManifest,
) -> Vec<PluginBundleCapabilitySummary> {
    let mut summaries = Vec::new();
    if !manifest.provides_services.is_empty() {
        summaries.push(PluginBundleCapabilitySummary {
            kind: PluginBundleCapabilityKind::Service,
            count: manifest.provides_services.len(),
            refs: manifest
                .provides_services
                .iter()
                .map(|service| service.service.id.as_str().to_string())
                .collect(),
        });
    }
    if !manifest.provides_capabilities.is_empty() {
        summaries.push(PluginBundleCapabilitySummary {
            kind: PluginBundleCapabilityKind::Capability,
            count: manifest.provides_capabilities.len(),
            refs: manifest
                .provides_capabilities
                .iter()
                .map(|capability| capability.id.as_str().to_string())
                .collect(),
        });
    }
    push_metadata_count(
        manifest,
        &mut summaries,
        "tool",
        PluginBundleCapabilityKind::Tool,
    );
    push_metadata_count(
        manifest,
        &mut summaries,
        "skill",
        PluginBundleCapabilityKind::Skill,
    );
    push_metadata_count(
        manifest,
        &mut summaries,
        "mcp",
        PluginBundleCapabilityKind::McpServer,
    );
    push_metadata_count(
        manifest,
        &mut summaries,
        "hook",
        PluginBundleCapabilityKind::Hook,
    );
    push_metadata_count(
        manifest,
        &mut summaries,
        "app",
        PluginBundleCapabilityKind::Application,
    );
    push_metadata_count(
        manifest,
        &mut summaries,
        "app_ui",
        PluginBundleCapabilityKind::AppUi,
    );
    summaries
}

fn push_metadata_count(
    manifest: &PluginManifest,
    summaries: &mut Vec<PluginBundleCapabilitySummary>,
    key: &str,
    kind: PluginBundleCapabilityKind,
) {
    let metadata_key = format!("bundle.{key}.count");
    let count = manifest
        .metadata
        .get(metadata_key.as_str())
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    if count > 0 {
        summaries.push(PluginBundleCapabilitySummary {
            kind,
            count,
            refs: Vec::new(),
        });
    }
}

pub fn descriptor() -> WorkbenchServiceDescriptor {
    WorkbenchServiceDescriptor::new(
        SERVICE_ID,
        CAPABILITY_ID,
        COMMANDS.to_vec(),
        "service.plugin_marketplace.events.v1",
        "service.plugin_marketplace.audit.v1",
        "service.plugin_marketplace.snapshot.v1",
    )
}
