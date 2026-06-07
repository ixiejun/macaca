//! Path and config adapter helpers for the web composition root.
//!
//! Pure filesystem/config translation with no application-specific semantics; Skill and Memory
//! services own interpretation of the paths returned here.

use std::path::PathBuf;

use macaca_proto::config::AutonomyConfig;

/// Build provider-neutral Skill package roots for governance restart recovery.
///
/// Web remains a composition root here: it contributes filesystem roots that it
/// already owns through configuration and application startup, but it does not
/// parse Skill metadata, inspect package bodies, or decide governance state.
/// The Skill service provider performs those semantic checks behind the service
/// boundary.
pub(crate) fn materialized_skill_recovery_roots(
    workspace_root: &str,
    application_skill_roots: &[PathBuf],
) -> Vec<PathBuf> {
    let mut roots = application_skill_roots.to_vec();
    let workspace_root = PathBuf::from(workspace_root);
    if let Ok(children) = std::fs::read_dir(&workspace_root) {
        for child in children.flatten() {
            let path = child.path();
            if !path.is_dir() {
                continue;
            }
            if path.file_name().is_some_and(|name| name == "apps") {
                continue;
            }
            roots.push(path.join("skills"));
        }
    }
    roots.sort();
    roots.dedup();
    roots
}

/// Return the local Skill governance event journal path for this host.
///
/// Web is acting only as an approved composition root here: it chooses a
/// workspace-scoped service storage location and passes it to the Skill service
/// provider.  It does not parse the journal, interpret governance events, or
/// branch on application-specific semantics.
pub(crate) fn skill_governance_event_journal_path(workspace_root: &str) -> PathBuf {
    PathBuf::from(workspace_root).join("skill-governance-events.jsonl")
}

/// Resolve the durable Memory file-store base path for local web runs.
///
/// The Memory service can write to both file/session layers and vector
/// providers. Keeping this resolver in the Web composition root lets operators
/// use a relative `memory.file_store_path` in config while still getting a
/// deterministic path under the host data directory. No application id, agent
/// name, or business workflow is hardcoded here; scope-specific isolation stays
/// inside Memory Service commands and providers.
pub(crate) fn configured_memory_base_path(data_dir: &std::path::Path, configured: &str) -> PathBuf {
    let configured = configured.trim();
    if configured.is_empty() {
        return data_dir.join("workspace_memory");
    }
    let path = PathBuf::from(configured);
    if path.is_absolute() {
        path
    } else {
        data_dir.join(path)
    }
}

/// Translate provider-neutral web configuration into runtime-host activation.
///
/// This Adapter keeps `macaca-proto` free of runtime-host concrete enum types
/// while still making local web startup explicit and auditable. Unknown modes
/// deliberately fall back to unavailable instead of trying to infer a provider;
/// that fail-closed behavior is part of the serviceization constitution.
pub(crate) fn autonomy_runtime_config_from_web_config(
    config: &AutonomyConfig,
) -> macaca_runtime_host::AutonomyRuntimeConfig {
    let provider_mode = match config.provider_mode.trim().to_ascii_lowercase().as_str() {
        "local" => macaca_runtime_host::AutonomyProviderMode::Local,
        _ => macaca_runtime_host::AutonomyProviderMode::Unavailable,
    };
    macaca_runtime_host::AutonomyRuntimeConfig {
        provider_mode,
        supervisor_enabled: config.supervisor_enabled,
        scheduler_tick_interval_ms: config.scheduler_tick_interval_ms,
        heartbeat_tick_interval_ms: config.heartbeat_tick_interval_ms,
        max_leases_per_tick: config.max_leases_per_tick,
        dispatch_timeout_ms: config.dispatch_timeout_ms,
        shutdown_grace_ms: config.shutdown_grace_ms,
        recovery_wake_enabled: config.recovery_wake_enabled,
        safe_retention_limit: config.safe_retention_limit,
    }
}
