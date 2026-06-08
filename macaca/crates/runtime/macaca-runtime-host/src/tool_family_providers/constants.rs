//! Stable identifiers and catalog tables for the industrial tool-family Abstract Factory.
//!
//! This module holds provider-neutral constants only.  Control flow must not branch on
//! application names here; families are enumerated as data so planning, inventory, and
//! governance tests share one source of truth.

/// Runtime-host task service identifier used by family ownership resolution.
pub(crate) const TASK_SERVICE_ID: &str = "service.task";

/// Service boundary for runtime-environment tool adapters (document family, etc.).
pub(crate) const TOOL_RUNTIME_ENVIRONMENT_SERVICE_ID: &str = "service.tool.runtime_environment";

/// Service boundary for managed gateway routes (browser, web, media, etc.).
pub(crate) const TOOL_MANAGED_GATEWAY_SERVICE_ID: &str = "service.tool.managed_gateway";

/// Service boundary for plugin-backed tool families (computer_use, etc.).
pub(crate) const TOOL_PLUGIN_SERVICE_ID: &str = "service.plugin";

/// Audit namespace stamped into sanitized family metadata for trace replay.
///
/// The namespace is metadata, not a routing branch, so `service.tool` can replay which
/// catalog produced a descriptor without learning provider-specific payloads.
pub(crate) const TOOL_FAMILY_AUDIT_NAMESPACE: &str = "service.tool.family";

/// Stable provider-neutral families required by the industrial Tools proposal.
///
/// The list is deliberately data, not control flow.  Adding a future family should extend
/// this table and associated descriptors rather than adding provider-name branches in
/// planning or invocation.
pub const REQUIRED_INDUSTRIAL_TOOL_FAMILIES: &[&str] = &[
    "file",
    "sandbox",
    "shell",
    "approval",
    "hook",
    "config",
    "plugin_marketplace",
    "browser",
    "web",
    "memory",
    "knowledge",
    "code_intelligence",
    "git",
    "review",
    "task",
    "scheduler",
    "skill",
    "mcp",
    "media",
    "document",
    "communication",
    "enterprise_api",
    "code_execution",
    "computer_use",
    "payment_entitlement",
    "diagnostics",
    "realtime",
    "remote_environment",
];
