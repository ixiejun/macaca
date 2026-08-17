//! Provider-neutral admission, resource, approval, and audit rules for config commands.
//!
//! These Specification helpers execute before a configuration provider is selected.
//! They deliberately accept only bounded request facts and policy evidence, never
//! resolved values, source locations, environment dumps, or secret material. This
//! keeps admission deterministic and lets a service decorator prove that rejected
//! commands cannot create source reloads, watches, snapshots, or provider calls.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::foundation_config::ConfigResultStatus;

const MAX_AUDIT_REFERENCE: usize = 160;

/// Resource units reserved before a config request reaches a provider.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigResourceReservation {
    pub key_units: u32,
    pub source_units: u32,
    pub watch_units: u32,
    pub reload_units: u32,
    pub export_units: u32,
    pub snapshot_units: u32,
    pub request_units: u32,
}

/// Policy-owned ceilings for a tenant, application, session, or task scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigResourceLimits {
    pub max_key_units: u32,
    pub max_source_units: u32,
    pub max_watch_units: u32,
    pub max_reload_units: u32,
    pub max_export_units: u32,
    pub max_snapshot_units: u32,
    pub max_request_units: u32,
}

/// Sanitized facts supplied by the policy and resource decorators.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigPolicyContext {
    pub declared_scopes: BTreeSet<String>,
    pub policy_allowed: bool,
    pub provider_available: bool,
    pub supports_watch: bool,
    pub supports_reload: bool,
    pub supports_redacted_export: bool,
    pub secret_reference_available: bool,
    pub approval_granted: bool,
    pub test_override_allowed: bool,
    pub limits: ConfigResourceLimits,
    pub current: ConfigResourceReservation,
}

/// Bounded request facts checked without interpreting a provider payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigAdmissionRequest {
    pub key_count: u32,
    pub source_count: u32,
    pub watch_units: u32,
    pub export_units: u32,
    pub snapshot_units: u32,
    pub has_valid_key: bool,
    pub has_valid_schema: bool,
    pub selector_supported: bool,
    pub validation_passed: bool,
    pub contains_raw_secret_value: bool,
    pub uses_secret_reference: bool,
    pub external_reload: bool,
    pub broad_export: bool,
    pub test_override: bool,
    pub tenant_wide_change: bool,
}

/// Stable failure states returned before a provider receives a rejected command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigAdmissionFailure {
    PermissionNotDeclared,
    PolicyDenied,
    ApprovalRequired,
    ProviderUnavailable,
    UnsupportedSelector,
    InvalidKey,
    InvalidSchema,
    ValidationFailed,
    SecretValueForbidden,
    QuotaExceeded,
}

impl ConfigAdmissionFailure {
    /// Map a pre-provider decision to the public, provider-neutral result status.
    pub fn status(self) -> ConfigResultStatus {
        match self {
            Self::PermissionNotDeclared | Self::PolicyDenied | Self::ApprovalRequired => {
                ConfigResultStatus::Denied
            }
            Self::ProviderUnavailable => ConfigResultStatus::Unavailable,
            Self::UnsupportedSelector => ConfigResultStatus::UnsupportedSelector,
            Self::InvalidKey => ConfigResultStatus::InvalidKey,
            Self::InvalidSchema => ConfigResultStatus::InvalidSchema,
            Self::ValidationFailed => ConfigResultStatus::ValidationFailed,
            Self::SecretValueForbidden => ConfigResultStatus::SecretValueForbidden,
            Self::QuotaExceeded => ConfigResultStatus::QuotaExceeded,
        }
    }
}

/// Validate policy, approval, source capability, and budgets before dispatching.
///
/// The command string is a descriptor-declared operation name, not a provider or
/// application routing key. Unknown command names are rejected as unavailable so
/// callers never accidentally dispatch a provider-specific extension.
pub fn preflight_command(
    command: &str,
    request: ConfigAdmissionRequest,
    context: &ConfigPolicyContext,
) -> Result<ConfigResourceReservation, ConfigAdmissionFailure> {
    let scope = required_scope(command).ok_or(ConfigAdmissionFailure::ProviderUnavailable)?;
    require_scope(context, scope)?;
    if !context.provider_available {
        return Err(ConfigAdmissionFailure::ProviderUnavailable);
    }
    if !request.has_valid_key && command_requires_key(command) {
        return Err(ConfigAdmissionFailure::InvalidKey);
    }
    if !request.has_valid_schema && command_requires_schema(command) {
        return Err(ConfigAdmissionFailure::InvalidSchema);
    }
    if !request.selector_supported {
        return Err(ConfigAdmissionFailure::UnsupportedSelector);
    }
    if !request.validation_passed {
        return Err(ConfigAdmissionFailure::ValidationFailed);
    }
    if request.contains_raw_secret_value
        || (request.uses_secret_reference && !context.secret_reference_available)
    {
        return Err(ConfigAdmissionFailure::SecretValueForbidden);
    }
    if (command == "config.watch" && !context.supports_watch)
        || (command == "config.reload" && !context.supports_reload)
        || (command == "config.export_redacted" && !context.supports_redacted_export)
    {
        return Err(ConfigAdmissionFailure::ProviderUnavailable);
    }
    if requires_approval(command, request) && !context.approval_granted {
        return Err(ConfigAdmissionFailure::ApprovalRequired);
    }
    if request.test_override && !context.test_override_allowed {
        return Err(ConfigAdmissionFailure::PolicyDenied);
    }
    reserve(
        context,
        ConfigResourceReservation {
            key_units: request.key_count,
            source_units: request.source_count,
            watch_units: request.watch_units,
            reload_units: u32::from(command == "config.reload"),
            export_units: request.export_units,
            snapshot_units: request.snapshot_units,
            request_units: 1,
        },
    )
}

/// Execute a provider closure only after the configuration policy admits it.
///
/// This intentionally small helper is shared by tests and runtime decorators to
/// make the no-side-effect invariant explicit and independently verifiable.
pub fn dispatch_after_preflight<T>(
    decision: Result<ConfigResourceReservation, ConfigAdmissionFailure>,
    provider: impl FnOnce() -> T,
) -> Result<T, ConfigAdmissionFailure> {
    decision.map(|_| provider())
}

/// Reserve request units in value form; durable metering belongs to a decorator.
pub fn reserve(
    context: &ConfigPolicyContext,
    requested: ConfigResourceReservation,
) -> Result<ConfigResourceReservation, ConfigAdmissionFailure> {
    let next = ConfigResourceReservation {
        key_units: context
            .current
            .key_units
            .saturating_add(requested.key_units),
        source_units: context
            .current
            .source_units
            .saturating_add(requested.source_units),
        watch_units: context
            .current
            .watch_units
            .saturating_add(requested.watch_units),
        reload_units: context
            .current
            .reload_units
            .saturating_add(requested.reload_units),
        export_units: context
            .current
            .export_units
            .saturating_add(requested.export_units),
        snapshot_units: context
            .current
            .snapshot_units
            .saturating_add(requested.snapshot_units),
        request_units: context
            .current
            .request_units
            .saturating_add(requested.request_units),
    };
    if next.key_units > context.limits.max_key_units
        || next.source_units > context.limits.max_source_units
        || next.watch_units > context.limits.max_watch_units
        || next.reload_units > context.limits.max_reload_units
        || next.export_units > context.limits.max_export_units
        || next.snapshot_units > context.limits.max_snapshot_units
        || next.request_units > context.limits.max_request_units
    {
        return Err(ConfigAdmissionFailure::QuotaExceeded);
    }
    Ok(next)
}

/// Build the bounded facts that trace and audit observers may retain.
pub fn redacted_config_audit_fields(
    command_name: &str,
    trace_id: &str,
    key_count: u32,
    source_count: u32,
) -> Option<ConfigAuditFields> {
    (safe(command_name) && safe(trace_id)).then(|| ConfigAuditFields {
        command_name: command_name.into(),
        trace_id: trace_id.into(),
        key_count,
        source_count,
        values_redacted: true,
    })
}

/// Sanitized audit payload deliberately lacking keys, values, and source handles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigAuditFields {
    pub command_name: String,
    pub trace_id: String,
    pub key_count: u32,
    pub source_count: u32,
    pub values_redacted: bool,
}

fn required_scope(command: &str) -> Option<&'static str> {
    match command {
        "config.describe_schema"
        | "config.get"
        | "config.get_many"
        | "config.resolve_effective"
        | "config.explain_provenance" => Some("config.read"),
        "config.list_keys" => Some("config.list"),
        "config.validate" => Some("config.validate"),
        "config.watch" => Some("config.watch"),
        "config.reload" => Some("config.reload"),
        "config.snapshot" => Some("config.snapshot"),
        "config.export_redacted" => Some("config.export"),
        _ => None,
    }
}

fn command_requires_key(command: &str) -> bool {
    matches!(
        command,
        "config.get" | "config.get_many" | "config.resolve_effective" | "config.explain_provenance"
    )
}

fn command_requires_schema(command: &str) -> bool {
    matches!(command, "config.describe_schema" | "config.validate")
}

fn requires_approval(command: &str, request: ConfigAdmissionRequest) -> bool {
    (command == "config.reload" && request.external_reload)
        || (command == "config.export_redacted" && request.broad_export)
        || request.test_override
        || request.tenant_wide_change
}

fn require_scope(context: &ConfigPolicyContext, scope: &str) -> Result<(), ConfigAdmissionFailure> {
    if !context.declared_scopes.contains(scope) {
        return Err(ConfigAdmissionFailure::PermissionNotDeclared);
    }
    if !context.policy_allowed {
        return Err(ConfigAdmissionFailure::PolicyDenied);
    }
    Ok(())
}

fn safe(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_AUDIT_REFERENCE
        && !value.chars().any(char::is_control)
        && !["secret", "credential", "payload", "value", "environment"]
            .iter()
            .any(|term| value.to_ascii_lowercase().contains(term))
}
