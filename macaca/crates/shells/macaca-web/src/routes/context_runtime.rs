//! Context provider runtime operator snapshot route.

use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::State;
use axum::Json;

use macaca_context::{
    ContextAdapterSafetyPolicy, ContextEngineInfo, ContextFallbackPolicy, ProviderHealthSnapshot,
};

use crate::state::{AppState, ExternalAdapterRuntimeInstallation};

/// Operator snapshot: built-in family descriptors, registry plugins, and rolling health — **no** prompt bodies.
pub async fn get_context_provider_runtime(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let builtin = macaca_context::list_builtin_family_descriptors();
    let builtin_engines = macaca_context::list_builtin_engine_infos();
    let builtin_engine_ids = builtin_engine_id_set(&builtin_engines);
    let registry_rows = state
        .context_provider_registry
        .list_registered_descriptors();
    let registry_engine_rows = state.context_engine_registry.list_engine_infos();
    state
        .external_adapter_runtime_registry
        .sync_registry_overlay_engines(&builtin_engine_ids, &registry_engine_rows)
        .await;
    let external_adapter_installations = state.external_adapter_runtime_registry.snapshot().await;
    let health = state.provider_health_ledger.snapshot();
    let external_adapter_runtime =
        external_adapter_runtime_rows(&external_adapter_installations, &health);
    let context_cfg = &state.config.context;
    Json(serde_json::json!({
        "default_engine": context_cfg.default_engine,
        "fallback_engine": context_cfg.fallback_engine,
        "emit_reports": context_cfg.emit_reports,
        "configured_provider_families": context_cfg.provider_families,
        "knowledge_digest_enabled": context_cfg.knowledge_digest.enabled,
        "active_vector_memory_enabled": context_cfg.active_vector_memory.enabled,
        "preflight_recall_enabled": context_cfg.recall.preflight_recall_enabled,
        "default_external_adapter_safety_policy": ContextAdapterSafetyPolicy::default(),
        "default_external_adapter_fallback_policy": ContextFallbackPolicy::default(),
        "builtin_engine_descriptors": builtin_engines,
        "registry_engine_descriptors": registry_engine_rows,
        "registry_engine_ids": state.context_engine_registry.list_engine_ids(),
        "external_adapter_runtime": external_adapter_runtime,
        "builtin_family_descriptors": builtin,
        "registry_family_descriptors": registry_rows,
        "registry_family_ids": state.context_provider_registry.list_family_ids(),
        "health": health,
    }))
}

fn builtin_engine_id_set(
    builtin_engines: &[ContextEngineInfo],
) -> std::collections::HashSet<String> {
    builtin_engines
        .iter()
        .map(|engine| engine.id.clone())
        .collect()
}

pub(crate) fn external_adapter_runtime_rows(
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
