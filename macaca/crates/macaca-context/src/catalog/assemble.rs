//! **Director** that composes a `Vec<Arc<dyn ContextProvider>>` from merged [`macaca_proto::config::ContextConfig`].
//!
//! Selection is strictly configuration-driven: either explicit `provider_families` rows or the
//! neutral [`super::constants::BUILTIN_FAMILY_ORDER`].  Missing optional dependencies simply skip
//! a family and emit **[`ContextDecisionReport`]** info rows — they MUST NOT abort the assembly.

use std::path::PathBuf;
use std::sync::Arc;

use macaca_proto::config::{ContextConfig, ContextProviderFamilyConfig};
use macaca_proto::MacacaResult;

use crate::capability::{
    mcp_capability_provider_arc, runtime_tool_capability_provider_arc, skill_capability_provider_arc,
};
use crate::catalog::constants::{
    BUILTIN_FAMILY_ORDER, FAMILY_AGENT_PROFILE, FAMILY_MCP_CAPABILITY, FAMILY_MEMORY_ACTIVE_RECALL,
    FAMILY_RUNTIME_TOOL_CAPABILITY, FAMILY_SKILL_CAPABILITY,
};
use crate::catalog::descriptor::MACACA_CONTEXT_CRATE_SEMVER;
use crate::catalog::versioned::VersionedContextProvider;
use crate::composer::ContextProvider;
use crate::governance::{ContextProviderRegistry, ProviderFactoryInput};
use crate::memory_active_recall_provider::memory_active_recall_provider_arc;
use crate::profile::profile_provider_arc;
use crate::report::{ContextDecisionReport, ContextDecisionSeverity};

/// Environment-specific dependencies for instantiating built-in families.
///
/// Every field is optional so [`Self::kernel_minimal`] can service bare `AgenticLoop` kernels while
/// fully populated structs back the HTTP/framework hot path.
#[derive(Clone)]
pub struct ProviderAssemblyEnvironment {
    pub agent_profile_root: Option<PathBuf>,
    pub agent_profile: macaca_proto::config::AgentProfileContextConfig,
    pub skill_capability_catalog: Option<Arc<crate::capability::SkillCapabilityCatalog>>,
    pub mcp_capability_catalog: Option<Arc<crate::capability::McpCapabilityCatalog>>,
    pub runtime_tool_capability_catalog: Option<Arc<crate::capability::RuntimeToolCapabilityCatalog>>,
    pub ready_mcp_server_ids: Option<Arc<Vec<String>>>,
    pub memory_recall_capability: Option<Arc<dyn crate::memory::ActiveRecallCapability>>,
    pub active_vector_memory: macaca_proto::config::ActiveVectorMemoryContextConfig,
}

impl ProviderAssemblyEnvironment {
    /// Empty environment: assembly yields no built-in providers (plugins may still attach).
    pub fn kernel_minimal() -> Self {
        Self {
            agent_profile_root: None,
            agent_profile: Default::default(),
            skill_capability_catalog: None,
            mcp_capability_catalog: None,
            runtime_tool_capability_catalog: None,
            ready_mcp_server_ids: None,
            memory_recall_capability: None,
            active_vector_memory: Default::default(),
        }
    }
}

fn implicit_family_rows() -> Vec<ContextProviderFamilyConfig> {
    BUILTIN_FAMILY_ORDER
        .iter()
        .map(|id| ContextProviderFamilyConfig {
            family_id: (*id).to_string(),
            enabled: true,
            params: serde_json::json!({}),
        })
        .collect()
}

fn effective_family_rows(cfg: &ContextConfig) -> Vec<ContextProviderFamilyConfig> {
    if cfg.provider_families.is_empty() {
        implicit_family_rows()
    } else {
        cfg.provider_families.clone()
    }
}

/// Shallow-merge factory input JSON for registry-backed plugin families only.
fn merge_params_global(global: &serde_json::Value, row: &serde_json::Value) -> serde_json::Value {
    use serde_json::{Map, Value};
    match (global.as_object(), row.as_object()) {
        (Some(ao), Some(bo)) => {
            let mut out: Map<String, Value> = ao.clone();
            for (k, v) in bo {
                out.insert(k.clone(), v.clone());
            }
            Value::Object(out)
        }
        _ => row.clone(),
    }
}

async fn try_registry_factory(
    registry: &ContextProviderRegistry,
    family: &ContextProviderFamilyConfig,
    base_input: &ProviderFactoryInput,
) -> Option<MacacaResult<Arc<dyn ContextProvider>>> {
    let factory = registry.get(family.family_id.as_str())?;
    let mut input = base_input.clone();
    input.params = merge_params_global(&base_input.params, &family.params);
    Some(factory.build_provider(&input).await)
}

/// Root composition routine — called from Web + kernel after assembling a [`ProviderFactoryInput`].
pub async fn assemble_context_providers(
    cfg: &ContextConfig,
    env: &ProviderAssemblyEnvironment,
    factory_input: &ProviderFactoryInput,
    registry: Option<&ContextProviderRegistry>,
) -> MacacaResult<(Vec<Arc<dyn ContextProvider>>, Vec<ContextDecisionReport>)> {
    let mut providers: Vec<Arc<dyn ContextProvider>> = Vec::new();
    let mut diagnostics = Vec::new();

    for family in effective_family_rows(cfg) {
        if !family.enabled {
            continue;
        }

        if let Some(reg) = registry {
            if let Some(res) = try_registry_factory(reg, &family, factory_input).await {
                let built = res?;
                providers.push(VersionedContextProvider::new(
                    built,
                    MACACA_CONTEXT_CRATE_SEMVER,
                ));
                continue;
            }
        }

        let fid = family.family_id.as_str();
        let maybe = match fid {
            FAMILY_AGENT_PROFILE => {
                if !env.agent_profile.enabled {
                    diagnostics.push(skip_diag(
                        fid,
                        "agent_profile disabled in AgentProfileContextConfig",
                    ));
                    None
                } else if let Some(root) = env.agent_profile_root.clone() {
                    Some(VersionedContextProvider::new(
                        profile_provider_arc(root, env.agent_profile.clone()),
                        MACACA_CONTEXT_CRATE_SEMVER,
                    ))
                } else {
                    diagnostics.push(skip_diag(fid, "agent_profile_root not available"));
                    None
                }
            }
            FAMILY_SKILL_CAPABILITY => {
                if let (Some(cat), Some(ready)) = (
                    env.skill_capability_catalog.as_ref(),
                    env.ready_mcp_server_ids.as_ref(),
                ) {
                    Some(VersionedContextProvider::new(
                        skill_capability_provider_arc(Arc::clone(cat), Arc::clone(ready)),
                        MACACA_CONTEXT_CRATE_SEMVER,
                    ))
                } else {
                    diagnostics.push(skip_diag(
                        fid,
                        "skill capability catalog or MCP readiness list unavailable",
                    ));
                    None
                }
            }
            FAMILY_MCP_CAPABILITY => {
                if let Some(cat) = env.mcp_capability_catalog.as_ref() {
                    Some(VersionedContextProvider::new(
                        mcp_capability_provider_arc(Arc::clone(cat)),
                        MACACA_CONTEXT_CRATE_SEMVER,
                    ))
                } else {
                    diagnostics.push(skip_diag(fid, "MCP capability catalog unavailable"));
                    None
                }
            }
            FAMILY_RUNTIME_TOOL_CAPABILITY => {
                if let Some(cat) = env.runtime_tool_capability_catalog.as_ref() {
                    Some(VersionedContextProvider::new(
                        runtime_tool_capability_provider_arc(Arc::clone(cat)),
                        MACACA_CONTEXT_CRATE_SEMVER,
                    ))
                } else {
                    diagnostics.push(skip_diag(fid, "runtime tool catalog unavailable"));
                    None
                }
            }
            FAMILY_MEMORY_ACTIVE_RECALL => {
                if !env.active_vector_memory.enabled {
                    diagnostics.push(skip_diag(
                        fid,
                        "active_vector_memory disabled in configuration",
                    ));
                    None
                } else if let Some(cap) = env.memory_recall_capability.as_ref() {
                    Some(VersionedContextProvider::new(
                        memory_active_recall_provider_arc(
                            Arc::clone(cap),
                            env.active_vector_memory.clone(),
                        ),
                        MACACA_CONTEXT_CRATE_SEMVER,
                    ))
                } else {
                    diagnostics.push(skip_diag(
                        fid,
                        "workspace memory / recall capability backend unavailable",
                    ));
                    None
                }
            }
            other => {
                diagnostics.push(ContextDecisionReport {
                    code: "context_provider_family_unknown".into(),
                    severity: ContextDecisionSeverity::Warning,
                    message: format!("unknown provider family id '{other}' — skipped"),
                });
                None
            }
        };

        if let Some(p) = maybe {
            providers.push(p);
        }
    }

    Ok((providers, diagnostics))
}

fn skip_diag(family_id: &str, reason: impl Into<String>) -> ContextDecisionReport {
    ContextDecisionReport {
        code: "context_provider_family_skipped".into(),
        severity: ContextDecisionSeverity::Info,
        message: format!("family {family_id}: {}", reason.into()),
    }
}
