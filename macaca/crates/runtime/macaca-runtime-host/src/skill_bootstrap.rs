//! Host-owned bootstrap helper for local Skill assets.
//!
//! Shells may need a lightweight operator-visible skill list and an initial
//! tool vector for generic toolkit assembly, but they should not construct the
//! Skill catalog or executable Skill registry directly. This module centralizes
//! that local bootstrap logic in runtime-host while preserving the service path
//! for lifecycle, governance, and executable Skill operations.

use std::path::PathBuf;
use std::sync::Arc;

use macaca_memory::MemoryRuntimeFacade;
use macaca_proto::{MacacaResult, SkillCatalogEntryView};
use macaca_skill::{ExecutableSkillToolSet, SkillCatalog};
use macaca_tools::Tool;

use crate::service_runtime::ServiceRuntime;
use crate::skill_service_provider::SkillSystemServiceProvider;
use crate::{ServiceProviderFactoryContext, ServiceProviderInstance, StaticServiceProviderFactory};

/// Runtime-host output for local Skill bootstrap.
pub struct SkillAssetBootstrapBundle {
    /// Read-only catalog entries for shell diagnostics and `/api/skills`.
    pub catalog_entries: Vec<SkillCatalogEntryView>,
    /// Tool adapters built from executable Skill definitions.
    pub executable_tools: Vec<Box<dyn Tool>>,
    /// Number of knowledge Skill records loaded across all configured roots.
    pub knowledge_loaded: usize,
    /// Number of executable Skill definitions loaded across all configured roots.
    pub executable_loaded: usize,
}

/// Register the local Skill service provider on the Service Runtime.
///
/// The helper is an Abstract Factory boundary for shell composition. Web and
/// CLI shells can supply provider-neutral configuration and optional service
/// facades, while runtime-host remains the only layer that constructs the
/// concrete `SkillSystemServiceProvider` and attaches it to the service bus.
pub async fn bootstrap_local_skill_service_provider(
    service_runtime: &ServiceRuntime,
    materialized_skill_roots: Vec<PathBuf>,
    governance_event_journal_path: PathBuf,
    memory_runtime: Option<Arc<dyn MemoryRuntimeFacade>>,
) -> MacacaResult<()> {
    tracing::info!(
        service_id = "skill",
        materialized_skill_roots = materialized_skill_roots.len(),
        governance_journal_configured = true,
        memory_runtime_configured = memory_runtime.is_some(),
        "runtime-host local skill service provider bootstrap requested"
    );

    let provider = SkillSystemServiceProvider::new()
        .with_materialized_skill_roots(materialized_skill_roots)
        .with_governance_event_journal_path(governance_event_journal_path);
    let provider = if let Some(memory_runtime) = memory_runtime {
        provider.with_memory_runtime(memory_runtime)
    } else {
        provider
    };

    service_runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(
                macaca_skill::skill_service_descriptor(),
                Arc::new(provider),
            )),
            ServiceProviderFactoryContext::new(),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;

    tracing::info!(
        service_id = "skill",
        "runtime-host local skill service provider bootstrap completed"
    );
    Ok(())
}

/// Load local Skill catalog entries and executable tool adapters.
///
/// The function applies the Facade pattern around `macaca-skill` constructors:
/// runtime-host owns scanning and registry creation; callers receive only
/// renderable catalog data and generic `Tool` trait objects.
pub async fn bootstrap_local_skill_assets(
    skills_dirs: &[PathBuf],
    trace_label: impl Into<String>,
) -> MacacaResult<SkillAssetBootstrapBundle> {
    let trace_label = trace_label.into();
    tracing::info!(
        roots = skills_dirs.len(),
        trace_label = %trace_label,
        "runtime-host local skill asset bootstrap requested"
    );

    let mut catalog = SkillCatalog::new();
    let mut knowledge_loaded = 0usize;
    let mut executable_loaded = 0usize;
    let mut executable_tools: Vec<Box<dyn Tool>> = Vec::new();

    for dir in skills_dirs {
        if !dir.exists() {
            tracing::debug!(
                path = %dir.display(),
                trace_label = %trace_label,
                "skill asset root skipped because it does not exist"
            );
            continue;
        }

        match catalog.load_from_directory(dir).await {
            Ok(count) => {
                knowledge_loaded += count;
                tracing::info!(
                    path = %dir.display(),
                    count,
                    trace_label = %trace_label,
                    "runtime-host loaded knowledge skills"
                );
            }
            Err(error) => {
                tracing::warn!(
                    path = %dir.display(),
                    error = %error,
                    trace_label = %trace_label,
                    "runtime-host failed to load knowledge skills"
                );
            }
        }

        let mut skill_tools = ExecutableSkillToolSet::new();
        match skill_tools.load_from_directory(dir).await {
            Ok(count) => {
                executable_loaded += count;
                executable_tools.extend(skill_tools.into_tools());
                tracing::info!(
                    path = %dir.display(),
                    count,
                    trace_label = %trace_label,
                    "runtime-host loaded executable skill tools"
                );
            }
            Err(error) => {
                tracing::warn!(
                    path = %dir.display(),
                    error = %error,
                    trace_label = %trace_label,
                    "runtime-host failed to load executable skill tools"
                );
            }
        }
    }

    let catalog_entries = catalog
        .catalog()
        .into_iter()
        .map(|entry| SkillCatalogEntryView {
            name: entry.name,
            description: entry.description,
        })
        .collect();

    tracing::info!(
        knowledge_loaded,
        executable_loaded,
        trace_label = %trace_label,
        "runtime-host local skill asset bootstrap completed"
    );

    Ok(SkillAssetBootstrapBundle {
        catalog_entries,
        executable_tools,
        knowledge_loaded,
        executable_loaded,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn bootstrap_empty_skill_root_returns_empty_snapshot() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = bootstrap_local_skill_assets(
            &[temp.path().to_path_buf()],
            "skill-asset-bootstrap-test",
        )
        .await
        .expect("empty skill root should still bootstrap");

        assert!(bundle.catalog_entries.is_empty());
        assert!(bundle.executable_tools.is_empty());
        assert_eq!(bundle.knowledge_loaded, 0);
        assert_eq!(bundle.executable_loaded, 0);
    }
}
