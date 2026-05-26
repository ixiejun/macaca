//! Availability Specification evaluator for `service.tool` planning.
//!
//! The evaluator consumes caller-provided signals only.  It does not read host
//! env vars, open secret stores, probe binaries, or contact providers.  That
//! keeps planning deterministic, cacheable, and safe to run in tests, shells,
//! background tasks, and future remote service hosts.

use std::collections::BTreeSet;

use macaca_proto::{AvailabilityExpression, ToolHiddenReason};

#[derive(Debug, Clone, Default)]
pub struct AvailabilitySignalSet {
    pub config_keys: BTreeSet<String>,
    pub secret_refs: BTreeSet<String>,
    pub auth_refs: BTreeSet<String>,
    pub env_keys: BTreeSet<String>,
    pub binaries: BTreeSet<String>,
    pub healthy_services: BTreeSet<String>,
    pub platform_os: Option<String>,
    pub resources: BTreeSet<String>,
    pub entitlements: BTreeSet<String>,
    pub plugins: BTreeSet<String>,
    pub manifest_capabilities: BTreeSet<String>,
    pub agent_policies: BTreeSet<String>,
    pub session_context_keys: BTreeSet<String>,
}

impl AvailabilitySignalSet {
    pub fn with_binary(mut self, binary: impl Into<String>) -> Self {
        self.binaries.insert(binary.into());
        self
    }

    pub fn with_auth(mut self, auth_ref: impl Into<String>) -> Self {
        self.auth_refs.insert(auth_ref.into());
        self
    }

    pub fn with_service(mut self, service_id: impl Into<String>) -> Self {
        self.healthy_services.insert(service_id.into());
        self
    }
}

#[derive(Debug, Clone, Default)]
pub struct ToolAvailabilityEvaluator {
    signals: AvailabilitySignalSet,
}

impl ToolAvailabilityEvaluator {
    pub fn new(signals: AvailabilitySignalSet) -> Self {
        Self { signals }
    }

    pub fn hidden_reason(
        &self,
        expressions: &[AvailabilityExpression],
    ) -> Option<ToolHiddenReason> {
        for expression in expressions {
            if let Some(reason) = self.evaluate(expression) {
                return Some(reason);
            }
        }
        None
    }

    fn evaluate(&self, expression: &AvailabilityExpression) -> Option<ToolHiddenReason> {
        match expression {
            AvailabilityExpression::Config { key } => missing(
                &self.signals.config_keys,
                key,
                ToolHiddenReason::MissingConfig,
            ),
            AvailabilityExpression::Secret { key_ref } => missing(
                &self.signals.secret_refs,
                key_ref,
                ToolHiddenReason::MissingSecret,
            ),
            AvailabilityExpression::Auth { provider_ref } => missing(
                &self.signals.auth_refs,
                provider_ref,
                ToolHiddenReason::MissingAuth,
            ),
            AvailabilityExpression::Env { key } => {
                missing(&self.signals.env_keys, key, ToolHiddenReason::MissingConfig)
            }
            AvailabilityExpression::Binary { binary } => missing(
                &self.signals.binaries,
                binary,
                ToolHiddenReason::MissingBinary,
            ),
            AvailabilityExpression::ServiceHealth { service_id } => missing(
                &self.signals.healthy_services,
                service_id,
                ToolHiddenReason::ServiceUnavailable,
            ),
            AvailabilityExpression::Platform { os } => match self.signals.platform_os.as_deref() {
                Some(current) if current == os => None,
                _ => Some(ToolHiddenReason::UnsupportedPlatform),
            },
            AvailabilityExpression::Resource { resource } => missing(
                &self.signals.resources,
                resource,
                ToolHiddenReason::ResourceUnavailable,
            ),
            AvailabilityExpression::Entitlement { entitlement } => missing(
                &self.signals.entitlements,
                entitlement,
                ToolHiddenReason::EntitlementMissing,
            ),
            AvailabilityExpression::Plugin { plugin_id } => missing(
                &self.signals.plugins,
                plugin_id,
                ToolHiddenReason::PluginUnavailable,
            ),
            AvailabilityExpression::Manifest { capability } => missing(
                &self.signals.manifest_capabilities,
                capability,
                ToolHiddenReason::ManifestDenied,
            ),
            AvailabilityExpression::AgentPolicy { policy_ref } => missing(
                &self.signals.agent_policies,
                policy_ref,
                ToolHiddenReason::PolicyDenied,
            ),
            AvailabilityExpression::SessionContext { key } => missing(
                &self.signals.session_context_keys,
                key,
                ToolHiddenReason::MissingConfig,
            ),
            AvailabilityExpression::All { items } => {
                items.iter().find_map(|item| self.evaluate(item))
            }
            AvailabilityExpression::Any { items } => {
                if items.iter().any(|item| self.evaluate(item).is_none()) {
                    None
                } else {
                    items.first().and_then(|item| self.evaluate(item))
                }
            }
            AvailabilityExpression::Not { item } => {
                if self.evaluate(item).is_none() {
                    Some(ToolHiddenReason::PolicyDenied)
                } else {
                    None
                }
            }
        }
    }
}

fn missing(
    set: &BTreeSet<String>,
    value: &str,
    reason: ToolHiddenReason,
) -> Option<ToolHiddenReason> {
    if set.contains(value) {
        None
    } else {
        Some(reason)
    }
}
