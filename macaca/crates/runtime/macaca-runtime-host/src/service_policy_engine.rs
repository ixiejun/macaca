//! Layered service policy engine for router admission.
//!
//! Policy evaluation follows a deterministic merge order:
//! platform baseline -> tenant override -> app override.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::RwLock;

use macaca_proto::KernelServiceId;

/// One policy layer represented as pure data.
#[derive(Debug, Clone, Default)]
pub struct ServicePolicyLayer {
    /// Services explicitly allowed by this layer.
    pub allow_services: BTreeSet<String>,
    /// Services explicitly denied by this layer.
    pub deny_services: BTreeSet<String>,
    /// Optional timeout override.
    pub timeout_ms: Option<u64>,
    /// Optional retry override.
    pub max_retries: Option<u32>,
}

/// Canonical input to the policy decision engine.
#[derive(Debug, Clone)]
pub struct ServicePolicyInput {
    pub app_id: Option<String>,
    pub tenant_id: Option<String>,
    pub service_id: KernelServiceId,
}

/// Policy decision payload returned to router callers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServicePolicyDecision {
    pub allow: bool,
    pub reason_code: String,
    pub timeout_ms: u64,
    pub max_retries: u32,
}

/// Policy engine interface so other engines can be plugged in later.
pub trait ServicePolicyEngine: Send + Sync {
    fn evaluate(&self, input: &ServicePolicyInput) -> ServicePolicyDecision;
}

/// In-memory deterministic policy engine for runtime host.
pub struct InMemoryServicePolicyEngine {
    baseline: RwLock<ServicePolicyLayer>,
    tenant_overrides: RwLock<BTreeMap<String, ServicePolicyLayer>>,
    app_overrides: RwLock<BTreeMap<String, ServicePolicyLayer>>,
}

impl InMemoryServicePolicyEngine {
    /// Create an engine with default permissive baseline.
    pub fn new() -> Self {
        Self {
            baseline: RwLock::new(ServicePolicyLayer::default()),
            tenant_overrides: RwLock::new(BTreeMap::new()),
            app_overrides: RwLock::new(BTreeMap::new()),
        }
    }

    /// Replace platform baseline policy.
    pub fn set_baseline(&self, baseline: ServicePolicyLayer) {
        *self
            .baseline
            .write()
            .expect("policy baseline lock poisoned") = baseline;
    }

    /// Set/replace one tenant override layer.
    pub fn set_tenant_override(&self, tenant_id: impl Into<String>, layer: ServicePolicyLayer) {
        self.tenant_overrides
            .write()
            .expect("policy tenant lock poisoned")
            .insert(tenant_id.into(), layer);
    }

    /// Set/replace one app override layer.
    pub fn set_app_override(&self, app_id: impl Into<String>, layer: ServicePolicyLayer) {
        self.app_overrides
            .write()
            .expect("policy app lock poisoned")
            .insert(app_id.into(), layer);
    }
}

impl Default for InMemoryServicePolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ServicePolicyEngine for InMemoryServicePolicyEngine {
    fn evaluate(&self, input: &ServicePolicyInput) -> ServicePolicyDecision {
        let baseline = self
            .baseline
            .read()
            .expect("policy baseline lock poisoned")
            .clone();
        let tenant = input.tenant_id.as_ref().and_then(|id| {
            self.tenant_overrides
                .read()
                .expect("policy tenant lock poisoned")
                .get(id)
                .cloned()
        });
        let app = input.app_id.as_ref().and_then(|id| {
            self.app_overrides
                .read()
                .expect("policy app lock poisoned")
                .get(id)
                .cloned()
        });
        let mut merged = baseline;
        if let Some(layer) = tenant {
            merged.allow_services.extend(layer.allow_services);
            merged.deny_services.extend(layer.deny_services);
            merged.timeout_ms = layer.timeout_ms.or(merged.timeout_ms);
            merged.max_retries = layer.max_retries.or(merged.max_retries);
        }
        if let Some(layer) = app {
            merged.allow_services.extend(layer.allow_services);
            merged.deny_services.extend(layer.deny_services);
            merged.timeout_ms = layer.timeout_ms.or(merged.timeout_ms);
            merged.max_retries = layer.max_retries.or(merged.max_retries);
        }
        let service_key = input.service_id.to_string();
        if merged.deny_services.contains(&service_key) {
            return ServicePolicyDecision {
                allow: false,
                reason_code: "policy_denied_explicit".into(),
                timeout_ms: merged.timeout_ms.unwrap_or(30_000),
                max_retries: merged.max_retries.unwrap_or(0),
            };
        }
        if !merged.allow_services.is_empty() && !merged.allow_services.contains(&service_key) {
            return ServicePolicyDecision {
                allow: false,
                reason_code: "policy_denied_not_allowed".into(),
                timeout_ms: merged.timeout_ms.unwrap_or(30_000),
                max_retries: merged.max_retries.unwrap_or(0),
            };
        }
        ServicePolicyDecision {
            allow: true,
            reason_code: "policy_allowed".into(),
            timeout_ms: merged.timeout_ms.unwrap_or(30_000),
            max_retries: merged.max_retries.unwrap_or(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_override_denies_even_when_baseline_allows() {
        let engine = InMemoryServicePolicyEngine::new();
        engine.set_baseline(ServicePolicyLayer {
            allow_services: BTreeSet::from([String::from("service.allowed")]),
            ..ServicePolicyLayer::default()
        });
        engine.set_app_override(
            "app-a",
            ServicePolicyLayer {
                deny_services: BTreeSet::from([String::from("service.allowed")]),
                ..ServicePolicyLayer::default()
            },
        );
        let decision = engine.evaluate(&ServicePolicyInput {
            app_id: Some("app-a".into()),
            tenant_id: None,
            service_id: KernelServiceId::new("service.allowed"),
        });
        assert!(!decision.allow);
        assert_eq!(decision.reason_code, "policy_denied_explicit");
    }
}
