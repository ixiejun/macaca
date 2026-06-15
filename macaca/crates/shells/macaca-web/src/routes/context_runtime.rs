//! Context provider runtime operator snapshot route.

use std::sync::Arc;

use axum::extract::State;
use axum::Json;

use crate::state::AppState;

/// Operator snapshot: built-in family descriptors, registry plugins, and rolling health — **no** prompt bodies.
pub async fn get_context_provider_runtime(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let snapshot = state.context_provider_runtime_snapshot().await;
    Json(serde_json::json!({
        "default_engine": snapshot.default_engine,
        "fallback_engine": snapshot.fallback_engine,
        "emit_reports": snapshot.emit_reports,
        "configured_provider_families": snapshot.configured_provider_families,
        "knowledge_digest_enabled": snapshot.knowledge_digest_enabled,
        "active_vector_memory_enabled": snapshot.active_vector_memory_enabled,
        "preflight_recall_enabled": snapshot.preflight_recall_enabled,
        "default_external_adapter_safety_policy": snapshot.default_external_adapter_safety_policy,
        "default_external_adapter_fallback_policy": snapshot.default_external_adapter_fallback_policy,
        "builtin_engine_descriptors": snapshot.builtin_engine_descriptors,
        "registry_engine_descriptors": snapshot.registry_engine_descriptors,
        "registry_engine_ids": snapshot.registry_engine_ids,
        "external_adapter_runtime": snapshot.external_adapter_runtime,
        "builtin_family_descriptors": snapshot.builtin_family_descriptors,
        "registry_family_descriptors": snapshot.registry_family_descriptors,
        "registry_family_ids": snapshot.registry_family_ids,
        "health": snapshot.health,
    }))
}
