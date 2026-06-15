//! Context provider runtime Facade for operator routes.
//!
//! Context provider diagnostics need host-owned registries, but HTTP routes
//! should only render sanitized snapshots. This module owns the shell-local
//! Adapter around those registries so `AppState` can stay a composition holder
//! instead of a large behavior object.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tokio::sync::RwLock;

use async_trait::async_trait;
use macaca_host_composition::context::{
    ContextAdapterSafetyPolicy, ContextEngineInfo, ContextEngineRegistry, ContextFallbackPolicy,
    ContextProviderRegistry, ProviderHealthLedger, ProviderHealthSnapshot,
};
use macaca_proto::config::ContextConfig;

/// Operator-visible installation record for one external adapter engine.
#[derive(Clone, Debug, Serialize)]
pub struct ExternalAdapterRuntimeInstallation {
    /// Stable engine metadata reported by the installed adapter wrapper.
    pub engine: ContextEngineInfo,
    /// Transport family used to reach the adapter implementation.
    pub transport: String,
    /// Which runtime mechanism installed this adapter into the current process.
    pub installation_source: String,
    /// Coarse-grained lifecycle state exported to operator diagnostics.
    pub runtime_state: String,
    /// Default safety guardrails currently associated with this adapter.
    pub default_safety_policy: ContextAdapterSafetyPolicy,
    /// Default fallback policy currently associated with this adapter.
    pub default_fallback_policy: ContextFallbackPolicy,
    /// Current circuit-breaker state. The current route exposes metadata only.
    pub circuit_breaker_runtime_state: String,
    /// Last time the inventory row was synchronized from runtime state.
    pub last_sync_epoch_ms: u128,
}

/// Dedicated inventory for installed external adapter engines.
#[derive(Default)]
pub struct ExternalAdapterRuntimeRegistry {
    rows: RwLock<HashMap<String, ExternalAdapterRuntimeInstallation>>,
}

impl ExternalAdapterRuntimeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create the runtime inventory with a precomputed installation snapshot.
    pub fn with_installations(
        self,
        installations: Vec<ExternalAdapterRuntimeInstallation>,
    ) -> Self {
        let rows = installations
            .into_iter()
            .map(|installation| (installation.engine.id.clone(), installation))
            .collect();
        Self {
            rows: RwLock::new(rows),
        }
    }

    fn now_ms() -> u128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(0)
    }

    /// Synchronize explicit inventory with non-builtin registry overlay engines.
    pub async fn sync_registry_overlay_engines(
        &self,
        builtin_engine_ids: &HashSet<String>,
        registry_engine_rows: &[ContextEngineInfo],
    ) {
        let now_ms = Self::now_ms();
        let mut rows = self.rows.write().await;
        let active_overlay_ids: HashSet<_> = registry_engine_rows
            .iter()
            .filter(|engine| !builtin_engine_ids.contains(&engine.id))
            .map(|engine| engine.id.clone())
            .collect();

        rows.retain(|engine_id, _| active_overlay_ids.contains(engine_id));

        for engine in registry_engine_rows
            .iter()
            .filter(|engine| !builtin_engine_ids.contains(&engine.id))
        {
            let existing = rows.get(&engine.id).cloned();
            rows.insert(
                engine.id.clone(),
                ExternalAdapterRuntimeInstallation {
                    engine: engine.clone(),
                    transport: existing
                        .as_ref()
                        .map(|row| row.transport.clone())
                        .unwrap_or_else(|| "registry_overlay".to_string()),
                    installation_source: existing
                        .as_ref()
                        .map(|row| row.installation_source.clone())
                        .unwrap_or_else(|| "context_engine_registry_overlay".to_string()),
                    runtime_state: existing
                        .as_ref()
                        .map(|row| row.runtime_state.clone())
                        .unwrap_or_else(|| "installed".to_string()),
                    default_safety_policy: existing
                        .as_ref()
                        .map(|row| row.default_safety_policy.clone())
                        .unwrap_or_default(),
                    default_fallback_policy: existing
                        .as_ref()
                        .map(|row| row.default_fallback_policy.clone())
                        .unwrap_or_default(),
                    circuit_breaker_runtime_state: existing
                        .as_ref()
                        .map(|row| row.circuit_breaker_runtime_state.clone())
                        .unwrap_or_else(|| "not_implemented".to_string()),
                    last_sync_epoch_ms: now_ms,
                },
            );
        }
    }

    /// Return a stable, operator-facing snapshot sorted by engine id.
    pub async fn snapshot(&self) -> Vec<ExternalAdapterRuntimeInstallation> {
        let mut rows: Vec<_> = self.rows.read().await.values().cloned().collect();
        rows.sort_by(|left, right| left.engine.id.cmp(&right.engine.id));
        rows
    }
}

/// Provider-neutral read model for the context provider runtime route.
#[derive(Clone, Debug, Serialize)]
pub struct ContextProviderRuntimeSnapshot {
    pub default_engine: String,
    pub fallback_engine: String,
    pub emit_reports: bool,
    pub configured_provider_families: serde_json::Value,
    pub knowledge_digest_enabled: bool,
    pub active_vector_memory_enabled: bool,
    pub preflight_recall_enabled: bool,
    pub default_external_adapter_safety_policy: ContextAdapterSafetyPolicy,
    pub default_external_adapter_fallback_policy: ContextFallbackPolicy,
    pub builtin_engine_descriptors: Vec<ContextEngineInfo>,
    pub registry_engine_descriptors: Vec<ContextEngineInfo>,
    pub registry_engine_ids: Vec<String>,
    pub external_adapter_runtime: Vec<serde_json::Value>,
    pub builtin_family_descriptors: serde_json::Value,
    pub registry_family_descriptors: serde_json::Value,
    pub registry_family_ids: Vec<String>,
    pub health: HashMap<String, ProviderHealthSnapshot>,
}

/// Provider-neutral port for context runtime diagnostics.
///
/// Web route state depends on this Facade instead of a host-backed concrete
/// client. The composition root installs the runtime-backed adapter, while
/// routes and `AppState` only request a sanitized read model.
#[async_trait]
pub trait SystemContextRuntimeClient: Send + Sync {
    /// Build a sanitized context-provider runtime snapshot for operator routes.
    async fn snapshot(&self) -> ContextProviderRuntimeSnapshot;
}

/// In-process context runtime snapshot Adapter.
pub struct WebContextRuntimeClient {
    context_config: ContextConfig,
    context_provider_registry: Arc<ContextProviderRegistry>,
    context_engine_registry: Arc<ContextEngineRegistry>,
    external_adapter_runtime_registry: Arc<ExternalAdapterRuntimeRegistry>,
    provider_health_ledger: Arc<ProviderHealthLedger>,
}

impl WebContextRuntimeClient {
    /// Create the context runtime snapshot client from bootstrap-owned registries.
    pub fn new(
        context_config: ContextConfig,
        context_provider_registry: Arc<ContextProviderRegistry>,
        context_engine_registry: Arc<ContextEngineRegistry>,
        external_adapter_runtime_registry: Arc<ExternalAdapterRuntimeRegistry>,
        provider_health_ledger: Arc<ProviderHealthLedger>,
    ) -> Self {
        Self {
            context_config,
            context_provider_registry,
            context_engine_registry,
            external_adapter_runtime_registry,
            provider_health_ledger,
        }
    }

    async fn build_snapshot(&self) -> ContextProviderRuntimeSnapshot {
        let builtin_family_descriptors =
            macaca_host_composition::context::list_builtin_family_descriptors();
        let builtin_engine_descriptors =
            macaca_host_composition::context::list_builtin_engine_infos();
        let builtin_engine_ids: HashSet<String> = builtin_engine_descriptors
            .iter()
            .map(|engine| engine.id.clone())
            .collect();
        let registry_family_descriptors =
            self.context_provider_registry.list_registered_descriptors();
        let registry_engine_descriptors = self.context_engine_registry.list_engine_infos();

        self.external_adapter_runtime_registry
            .sync_registry_overlay_engines(&builtin_engine_ids, &registry_engine_descriptors)
            .await;
        let external_adapter_installations =
            self.external_adapter_runtime_registry.snapshot().await;
        let health = self.provider_health_ledger.snapshot();
        let external_adapter_runtime =
            context_external_adapter_runtime_rows(&external_adapter_installations, &health);
        let context_cfg = &self.context_config;

        tracing::info!(
            builtin_engines = builtin_engine_descriptors.len(),
            registry_engines = registry_engine_descriptors.len(),
            external_adapters = external_adapter_runtime.len(),
            health_rows = health.len(),
            "Context provider runtime snapshot prepared for shell route"
        );

        ContextProviderRuntimeSnapshot {
            default_engine: context_cfg.default_engine.clone(),
            fallback_engine: context_cfg.fallback_engine.clone(),
            emit_reports: context_cfg.emit_reports,
            configured_provider_families: sanitized_snapshot_value(
                "configured_provider_families",
                &context_cfg.provider_families,
            ),
            knowledge_digest_enabled: context_cfg.knowledge_digest.enabled,
            active_vector_memory_enabled: context_cfg.active_vector_memory.enabled,
            preflight_recall_enabled: context_cfg.recall.preflight_recall_enabled,
            default_external_adapter_safety_policy: ContextAdapterSafetyPolicy::default(),
            default_external_adapter_fallback_policy: ContextFallbackPolicy::default(),
            builtin_engine_descriptors,
            registry_engine_descriptors,
            registry_engine_ids: self.context_engine_registry.list_engine_ids(),
            external_adapter_runtime,
            builtin_family_descriptors: sanitized_snapshot_value(
                "builtin_family_descriptors",
                &builtin_family_descriptors,
            ),
            registry_family_descriptors: sanitized_snapshot_value(
                "registry_family_descriptors",
                &registry_family_descriptors,
            ),
            registry_family_ids: self.context_provider_registry.list_family_ids(),
            health,
        }
    }
}

#[async_trait]
impl SystemContextRuntimeClient for WebContextRuntimeClient {
    async fn snapshot(&self) -> ContextProviderRuntimeSnapshot {
        self.build_snapshot().await
    }
}

fn sanitized_snapshot_value<T: Serialize>(field: &'static str, value: &T) -> serde_json::Value {
    serde_json::to_value(value).unwrap_or_else(|error| {
        tracing::warn!(
            field,
            error = %error,
            "Context provider runtime snapshot field serialization failed"
        );
        serde_json::Value::Array(Vec::new())
    })
}

/// Build sorted external adapter rows with the latest bounded health snapshot.
pub(crate) fn context_external_adapter_runtime_rows(
    installations: &[ExternalAdapterRuntimeInstallation],
    health: &HashMap<String, ProviderHealthSnapshot>,
) -> Vec<serde_json::Value> {
    let mut rows: Vec<_> = installations
        .iter()
        .map(|installation| {
            let last_health = health.get(&installation.engine.id).cloned();
            let runtime_status = if last_health.is_some() {
                "observed_via_health_ledger".to_string()
            } else {
                installation.runtime_state.clone()
            };
            serde_json::json!({
                "engine": installation.engine,
                "transport": installation.transport,
                "installation_source": installation.installation_source,
                "runtime_status": runtime_status,
                "default_safety_policy": installation.default_safety_policy,
                "default_fallback_policy": installation.default_fallback_policy,
                "circuit_breaker": {
                    "configured_failures": installation.default_safety_policy.circuit_breaker_failures,
                    "runtime_state": installation.circuit_breaker_runtime_state,
                },
                "last_sync_epoch_ms": installation.last_sync_epoch_ms,
                "last_health": last_health,
            })
        })
        .collect();
    rows.sort_by(|a, b| {
        let a_id = a["engine"]["id"].as_str().unwrap_or_default();
        let b_id = b["engine"]["id"].as_str().unwrap_or_default();
        a_id.cmp(b_id)
    });
    rows
}

#[cfg(test)]
mod tests {
    use super::{
        context_external_adapter_runtime_rows, ExternalAdapterRuntimeInstallation,
        ExternalAdapterRuntimeRegistry,
    };
    use std::collections::{HashMap, HashSet};

    use macaca_host_composition::context::{
        ContextAdapterSafetyPolicy, ContextEngineInfo, ContextFallbackPolicy,
        ProviderHealthSnapshot,
    };

    #[tokio::test]
    async fn external_adapter_runtime_registry_syncs_only_overlay_engines() {
        let registry = ExternalAdapterRuntimeRegistry::new();
        registry
            .sync_registry_overlay_engines(
                &HashSet::from(["passthrough".to_string()]),
                &[
                    ContextEngineInfo::new("passthrough", "Passthrough Context Engine"),
                    ContextEngineInfo::new("custom-external", "Custom External Engine"),
                ],
            )
            .await;

        let snapshot = registry.snapshot().await;
        assert_eq!(snapshot.len(), 1);
        assert_eq!(snapshot[0].engine.id, "custom-external");
        assert_eq!(snapshot[0].runtime_state, "installed");
    }

    #[tokio::test]
    async fn external_adapter_runtime_registry_prunes_removed_overlay_engines() {
        let registry = ExternalAdapterRuntimeRegistry::new();
        registry
            .sync_registry_overlay_engines(
                &HashSet::new(),
                &[ContextEngineInfo::new(
                    "custom-external",
                    "Custom External Engine",
                )],
            )
            .await;
        registry
            .sync_registry_overlay_engines(&HashSet::new(), &[])
            .await;

        assert!(registry.snapshot().await.is_empty());
    }

    #[test]
    fn external_adapter_runtime_rows_exclude_builtin_engines() {
        let rows = context_external_adapter_runtime_rows(
            &[ExternalAdapterRuntimeInstallation {
                engine: ContextEngineInfo::new("custom-external", "Custom External Engine"),
                transport: "registry_overlay".into(),
                installation_source: "context_engine_registry_overlay".into(),
                runtime_state: "installed".into(),
                default_safety_policy: ContextAdapterSafetyPolicy::default(),
                default_fallback_policy: ContextFallbackPolicy::default(),
                circuit_breaker_runtime_state: "not_implemented".into(),
                last_sync_epoch_ms: 7,
            }],
            &HashMap::new(),
        );

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["engine"]["id"], "custom-external");
        assert_eq!(rows[0]["runtime_status"], "installed");
        assert_eq!(
            rows[0]["installation_source"],
            "context_engine_registry_overlay"
        );
        assert_eq!(
            rows[0]["circuit_breaker"]["runtime_state"],
            "not_implemented"
        );
    }

    #[test]
    fn external_adapter_runtime_rows_attach_last_health_when_present() {
        let mut health = HashMap::new();
        health.insert(
            "custom-external".to_string(),
            ProviderHealthSnapshot {
                outcome: "success".into(),
                latency_ms: 12,
                last_seen_epoch_ms: 123,
                implementation_version: Some("1.2.3".into()),
            },
        );
        let rows = context_external_adapter_runtime_rows(
            &[ExternalAdapterRuntimeInstallation {
                engine: ContextEngineInfo::new("custom-external", "Custom External Engine"),
                transport: "registry_overlay".into(),
                installation_source: "context_engine_registry_overlay".into(),
                runtime_state: "installed".into(),
                default_safety_policy: ContextAdapterSafetyPolicy::default(),
                default_fallback_policy: ContextFallbackPolicy::default(),
                circuit_breaker_runtime_state: "not_implemented".into(),
                last_sync_epoch_ms: 7,
            }],
            &health,
        );

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["runtime_status"], "observed_via_health_ledger");
        assert_eq!(rows[0]["last_health"]["outcome"], "success");
        assert_eq!(rows[0]["last_health"]["implementation_version"], "1.2.3");
    }
}
