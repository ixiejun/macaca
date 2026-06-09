//! Ability descriptor synthesis for YAML → Manifest v1 projection.
//!
//! Each builder function maps a legacy agent source into a provider-neutral
//! `ApplicationAbilityDescriptor`.  WASM runtime abilities are synthesized
//! generically from service contracts via the **Strategy** catalog interface.

use macaca_proto::{
    AbilityActivation, AbilityCapabilityDeclaration, AbilityImplementationKind,
    AbilityPermissionDeclaration, AbilityServiceRequirement, ApplicationAbilityDescriptor,
    ApplicationAbilityKind, CapabilityId, KernelServiceId,
};
use macaca_proto::AgentConfig;

use crate::model::{AgentSource, AppLayer, InlineAgentConfig};
use crate::service_capability::{expand_service_capabilities, DomainPackCatalog};

use super::entry::sanitize_path_for_id;
use super::types::YamlApplicationManifestAdapter;

impl YamlApplicationManifestAdapter {
    /// Build all projected abilities from a legacy YAML manifest.
    ///
    /// The adapter always projects declared YAML agents, and for L2 WASM
    /// applications with a declared service contract it synthesizes one generic
    /// headless WASM execution ability. This keeps runtime admission and
    /// metadata query surfaces auditable without introducing app-specific
    /// host-side logic.
    pub(super) fn projected_abilities(
        &self,
        entry: &Option<String>,
        catalog: &dyn DomainPackCatalog,
    ) -> Vec<ApplicationAbilityDescriptor> {
        let mut abilities = Vec::new();
        for source in &self.manifest.agents {
            match source {
                AgentSource::Inline(inline) => abilities.push(inline_agent_ability(inline, entry)),
                AgentSource::FilePath(path) => abilities.push(file_agent_ability(path, entry)),
            }
        }
        for agent in &self.resolved_agents {
            abilities.push(resolved_agent_ability(agent, entry));
        }
        if let Some(wasm_ability) = self.wasm_runtime_ability(catalog) {
            abilities.push(wasm_ability);
        }
        abilities.sort_by(|a, b| a.id.cmp(&b.id));
        abilities.dedup_by(|a, b| a.id == b.id);
        abilities
    }

    /// Synthesize one generic WASM runtime ability from service contract declarations.
    ///
    /// The synthesized descriptor is intentionally data-only: it declares runtime
    /// intent and service dependencies but does not embed provider internals,
    /// app names, or workflow-specific behavior.
    fn wasm_runtime_ability(
        &self,
        catalog: &dyn DomainPackCatalog,
    ) -> Option<ApplicationAbilityDescriptor> {
        if !matches!(self.manifest.layer, AppLayer::L2Wasm) {
            return None;
        }
        let contract = self.manifest.service_contract.as_ref()?;
        let capabilities = expand_service_capabilities(Some(contract), catalog);
        let mut ability = ApplicationAbilityDescriptor::new(
            "ability.runtime.wasm".to_string(),
            ApplicationAbilityKind::Headless,
            AbilityImplementationKind::WasmComponent,
        )
        .activation(
            AbilityActivation::new("wasm")
                .entry("service.call")
                .metadata("host.import", "service.call"),
        )
        .service(
            AbilityServiceRequirement::required(
                KernelServiceId::new("service.application.host"),
                "WASM runtime ability requires host command dispatch surface",
            )
            .capability(CapabilityId::new(
                "capability.wasm.host_import.service_call",
            )),
        );
        for service_id in &capabilities.services {
            ability = ability.service(
                AbilityServiceRequirement::required(
                    KernelServiceId::new(service_id.clone()),
                    "Declared by service contract or expanded domain pack",
                )
                .capability(CapabilityId::new(format!(
                    "capability.service.call.{service_id}"
                ))),
            );
        }
        ability.metadata.insert(
            "service_contract_hash".into(),
            capabilities.capabilities_hash,
        );
        ability.metadata.insert(
            "service_contract_count".into(),
            capabilities.services.len().to_string(),
        );
        Some(ability)
    }
}

/// Project an inline YAML agent declaration into a Manifest v1 ability.
fn inline_agent_ability(
    inline: &InlineAgentConfig,
    entry: &Option<String>,
) -> ApplicationAbilityDescriptor {
    let mut ability = base_agent_ability(inline.name.as_str(), "inline", entry);
    ability
        .metadata
        .insert("model.present".into(), "true".into());
    ability
        .metadata
        .insert("permission_level".into(), inline.permission_level.clone());
    if inline.skills.is_some() {
        ability
            .metadata
            .insert("skills.policy.present".into(), "true".into());
    }
    if inline.context_engine.is_some() {
        ability
            .metadata
            .insert("context.engine.present".into(), "true".into());
    }
    for capability in &inline.capabilities {
        ability = ability.capability(AbilityCapabilityDeclaration::new(
            CapabilityId::new(format!("agent.{}.{}", inline.name, capability.name)),
            capability.description.clone(),
        ));
    }
    for tool in &inline.allowed_tools {
        ability = ability.capability(AbilityCapabilityDeclaration::new(
            CapabilityId::new(format!("tool.{tool}")),
            format!("Allowed tool declared by agent {}", inline.name),
        ));
    }
    ability
}

/// Project a file-path YAML agent reference into a Manifest v1 ability.
fn file_agent_ability(path: &str, entry: &Option<String>) -> ApplicationAbilityDescriptor {
    let stable_id = sanitize_path_for_id(path);
    let mut ability = base_agent_ability(stable_id.as_str(), "file", entry);
    ability
        .metadata
        .insert("legacy.agent.path".into(), path.into());
    ability
}

/// Project a resolved file-based agent into a Manifest v1 ability.
fn resolved_agent_ability(
    agent: &AgentConfig,
    entry: &Option<String>,
) -> ApplicationAbilityDescriptor {
    let mut ability = base_agent_ability(agent.name.as_str(), "resolved", entry);
    ability
        .metadata
        .insert("permission_level".into(), agent.permission_level.clone());
    if agent.skills.is_some() {
        ability
            .metadata
            .insert("skills.policy.present".into(), "true".into());
    }
    if agent.network_access {
        ability = ability.permission(AbilityPermissionDeclaration::required(
            "network.access",
            "Resolved YAML agent declares network access",
        ));
    }
    if !agent.allowed_paths.is_empty() {
        ability = ability.permission(AbilityPermissionDeclaration::required(
            "filesystem.scoped",
            "Resolved YAML agent declares scoped filesystem paths",
        ));
    }
    for capability in &agent.capabilities {
        ability = ability.capability(AbilityCapabilityDeclaration::new(
            CapabilityId::new(format!("agent.{}.{}", agent.name, capability.name)),
            capability.description.clone(),
        ));
    }
    for tool in &agent.allowed_tools {
        ability = ability.capability(AbilityCapabilityDeclaration::new(
            CapabilityId::new(format!("tool.{tool}")),
            format!("Allowed tool declared by agent {}", agent.name),
        ));
    }
    ability
}

/// Build the shared agent ability skeleton for all YAML agent sources.
fn base_agent_ability(
    id_fragment: &str,
    source: &str,
    entry: &Option<String>,
) -> ApplicationAbilityDescriptor {
    let mut activation = AbilityActivation::new("agent").entry(id_fragment);
    activation.metadata.insert("source".into(), source.into());
    let mut ability = ApplicationAbilityDescriptor::new(
        format!("ability.agent.{id_fragment}"),
        ApplicationAbilityKind::Agent,
        AbilityImplementationKind::Declarative,
    )
    .activation(activation)
    .service(
        AbilityServiceRequirement::required(
            KernelServiceId::new("service.agent.runtime"),
            "YAML agent ability requires agent runtime service",
        )
        .capability(CapabilityId::new("capability.agent.execute")),
    );
    ability.metadata.insert("source".into(), source.into());
    if entry.as_deref() == Some(id_fragment) {
        ability.metadata.insert("entry".into(), "true".into());
    }
    ability
}
