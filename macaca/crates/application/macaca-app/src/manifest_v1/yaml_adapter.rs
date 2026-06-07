//! YAML application adapter for Application Manifest v1.
//!
//! YAML applications are still first-class inputs, but new Application Platform
//! features should reason over Manifest v1 and ability descriptors.  This file
//! applies the Adapter pattern: it projects the legacy YAML model into
//! provider-neutral Manifest v1 data while preserving existing runtime
//! behavior in the legacy loader.  It also returns a Memento-style report so
//! inferred defaults and compatibility facts are auditable without logging raw
//! manifest bodies, prompt templates, secrets, environment values, or host
//! payloads.

use macaca_proto::{
    AbilityActivation, AbilityCapabilityDeclaration, AbilityImplementationKind,
    AbilityPermissionDeclaration, AbilityServiceRequirement, ApplicationAbilityDescriptor,
    ApplicationAbilityKind, ApplicationCompatibilityDeclaration, ApplicationManifestV1,
    ApplicationPermissionDeclaration, ApplicationRuntimeProfile, CapabilityId, DeveloperId,
    KernelServiceId, PackageId, PackageRuntimeKind,
};
use macaca_sdk::AgentConfig;
use tracing::info;

use crate::model::{AgentSource, AppManifest, EntrypointType, InlineAgentConfig};
use crate::service_capability::{
    expand_service_capabilities, DomainPackCatalog, InMemoryDomainPackCatalog,
};

/// Safe diagnostic emitted while projecting YAML data into Manifest v1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlProjectionDiagnostic {
    pub code: String,
    pub subject: String,
    pub message: String,
}

impl YamlProjectionDiagnostic {
    fn new(
        code: impl Into<String>,
        subject: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            subject: subject.into(),
            message: message.into(),
        }
    }
}

/// Conversion report that records safe facts about the YAML projection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct YamlToApplicationManifestV1Report {
    pub application_id: String,
    pub package_id: String,
    pub ability_count: usize,
    pub inferred_defaults: Vec<YamlProjectionDiagnostic>,
    pub compatibility_warnings: Vec<YamlProjectionDiagnostic>,
    pub legacy_only_fields: Vec<YamlProjectionDiagnostic>,
}

impl YamlToApplicationManifestV1Report {
    fn push_default(&mut self, subject: impl Into<String>, message: impl Into<String>) {
        self.inferred_defaults.push(YamlProjectionDiagnostic::new(
            "inferred_default",
            subject,
            message,
        ));
    }

    fn push_legacy_only(&mut self, subject: impl Into<String>, message: impl Into<String>) {
        self.legacy_only_fields.push(YamlProjectionDiagnostic::new(
            "legacy_only_field",
            subject,
            message,
        ));
    }
}

/// Projection result used by package and ABI adapters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyAppManifestProjection {
    pub manifest: ApplicationManifestV1,
    pub report: YamlToApplicationManifestV1Report,
}

/// Adapter that converts legacy YAML app manifests into Manifest v1.
pub struct YamlApplicationManifestAdapter {
    manifest: AppManifest,
    resolved_agents: Vec<AgentConfig>,
}

impl YamlApplicationManifestAdapter {
    /// Create an adapter from the parsed legacy YAML manifest.
    pub fn new(manifest: AppManifest) -> Self {
        Self {
            manifest,
            resolved_agents: Vec::new(),
        }
    }

    /// Attach resolved file-based agents when the caller has already performed
    /// legacy file resolution.  The adapter uses the resolved facts for
    /// capabilities and permissions but never stores raw agent configuration
    /// bodies in Manifest v1 metadata.
    pub fn with_resolved_agents(mut self, resolved_agents: Vec<AgentConfig>) -> Self {
        self.resolved_agents = resolved_agents;
        self
    }

    /// Project YAML application data into Manifest v1 using an empty catalog.
    pub fn project(self) -> LegacyAppManifestProjection {
        self.project_with_catalog(&InMemoryDomainPackCatalog::with_builtin_defaults())
    }

    /// Project YAML application data using a host-installed domain-pack catalog.
    ///
    /// WASM runtime ability synthesis expands `service_contract.use_packs`
    /// through the injected catalog so composition roots can register optional
    /// domain extensions without embedding pack ids in this adapter.
    pub fn project_with_catalog(self, catalog: &dyn DomainPackCatalog) -> LegacyAppManifestProjection {
        let application_id = self.manifest.id.to_string();
        let package_id = format!("application.{application_id}");
        let entry = entry_value(&self.manifest);
        let entry_kind = entry_kind(&self.manifest);
        let mut report = YamlToApplicationManifestV1Report {
            application_id: application_id.clone(),
            package_id: package_id.clone(),
            ..Default::default()
        };
        if entry.is_none() {
            report.push_default(
                application_id.as_str(),
                "No explicit YAML entry was declared; legacy runtime will infer entry agent.",
            );
        }

        let mut runtime = ApplicationRuntimeProfile::new(PackageRuntimeKind::Yaml, "1");
        if let Some(entry) = &entry {
            runtime.entry = Some(entry.clone());
        }
        if let Some(kind) = &entry_kind {
            runtime.metadata.insert("entry.kind".into(), kind.clone());
        }
        runtime
            .metadata
            .insert("source.format".into(), "yaml".into());
        runtime.metadata.insert(
            "legacy.layer".into(),
            format!("{:?}", self.manifest.layer).to_lowercase(),
        );

        let mut projected = ApplicationManifestV1::new(
            PackageId::new(package_id.clone()),
            DeveloperId::new("local.application"),
            self.manifest.name.clone(),
            self.manifest.version.clone(),
            runtime,
            ApplicationCompatibilityDeclaration::new("0.1.0"),
        );
        projected
            .metadata
            .insert("application.id".into(), application_id.clone());
        projected
            .metadata
            .insert("source.format".into(), "yaml".into());
        projected
            .metadata
            .insert("agent.count".into(), self.manifest.agents.len().to_string());
        projected.metadata.insert(
            "resolved.agent.count".into(),
            self.resolved_agents.len().to_string(),
        );
        if let Some(ui_type) = self.manifest.ui_type {
            projected.metadata.insert(
                "legacy.ui_type".into(),
                format!("{ui_type:?}").to_lowercase(),
            );
            report.push_legacy_only(
                application_id.as_str(),
                "YAML ui_type is preserved as sanitized compatibility metadata.",
            );
        }
        if self.manifest.context.is_some() {
            projected
                .metadata
                .insert("legacy.context.present".into(), "true".into());
            report.push_legacy_only(
                application_id.as_str(),
                "YAML context configuration remains owned by legacy runtime compatibility.",
            );
        }
        if self.manifest.resources.is_some() {
            projected
                .metadata
                .insert("legacy.resources.present".into(), "true".into());
            report.push_legacy_only(
                application_id.as_str(),
                "YAML resource paths remain owned by legacy runtime compatibility.",
            );
        }
        if self.manifest.workflows.is_some() {
            projected
                .metadata
                .insert("legacy.workflows.present".into(), "true".into());
        }
        if let Some(workbench) = &self.manifest.workbench {
            if !workbench.is_empty() {
                projected.workbench = Some(workbench.clone());
                projected
                    .tool_families
                    .extend(workbench.tool_families.iter().cloned());
                projected
                    .permission_profiles
                    .extend(workbench.permission_profiles.iter().cloned());
                report.push_legacy_only(
                    application_id.as_str(),
                    "YAML workbench declaration was projected into Manifest v1 policy metadata.",
                );
            }
        }
        if let Some(execution_profile) = &self.manifest.execution_profile {
            projected.execution_profile = Some(execution_profile.clone());
            report.push_legacy_only(
                application_id.as_str(),
                "YAML application execution profile was projected into Manifest v1 policy metadata.",
            );
        }
        if let Some(execution_control) = &self.manifest.execution_control {
            projected.execution_control = Some(execution_control.clone());
            info!(
                application_id = %application_id,
                mode = ?execution_control.mode,
                trigger_count = execution_control.triggers.len(),
                resume_source_count = execution_control.resume_sources.len(),
                "YAML execution_control projected into Manifest v1"
            );
        }

        for permission in application_permissions(&self.resolved_agents) {
            projected = projected.permission(permission);
        }
        for ability in self.projected_abilities(&entry, catalog) {
            projected = projected.ability(ability);
        }
        report.ability_count = projected.abilities.len();

        info!(
            application_id = %application_id,
            package_id = %package_id,
            ability_count = report.ability_count,
            default_count = report.inferred_defaults.len(),
            legacy_only_count = report.legacy_only_fields.len(),
            "YAML application projected to Manifest v1"
        );

        LegacyAppManifestProjection {
            manifest: projected,
            report,
        }
    }

    /// Build all projected abilities from a legacy YAML manifest.
    ///
    /// The adapter always projects declared YAML agents, and for L2 WASM
    /// applications with a declared service contract it synthesizes one generic
    /// headless WASM execution ability. This keeps runtime admission and
    /// metadata query surfaces auditable without introducing app-specific
    /// host-side logic.
    fn projected_abilities(
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

    /// Synthesize one generic WASM runtime ability from service contract
    /// declarations. The synthesized descriptor is intentionally data-only:
    /// it declares runtime intent and service dependencies but does not embed
    /// provider internals, app names, or workflow-specific behavior.
    fn wasm_runtime_ability(
        &self,
        catalog: &dyn DomainPackCatalog,
    ) -> Option<ApplicationAbilityDescriptor> {
        if !matches!(self.manifest.layer, crate::model::AppLayer::L2Wasm) {
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

fn file_agent_ability(path: &str, entry: &Option<String>) -> ApplicationAbilityDescriptor {
    let stable_id = sanitize_path_for_id(path);
    let mut ability = base_agent_ability(stable_id.as_str(), "file", entry);
    ability
        .metadata
        .insert("legacy.agent.path".into(), path.into());
    ability
}

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

fn application_permissions(agents: &[AgentConfig]) -> Vec<ApplicationPermissionDeclaration> {
    let mut permissions = Vec::new();
    if agents.iter().any(|agent| agent.network_access) {
        permissions.push(ApplicationPermissionDeclaration::required(
            "network.access",
            "At least one YAML agent declares network access",
        ));
    }
    if agents.iter().any(|agent| !agent.allowed_paths.is_empty()) {
        permissions.push(ApplicationPermissionDeclaration::required(
            "filesystem.scoped",
            "At least one YAML agent declares scoped filesystem paths",
        ));
    }
    permissions
}

fn entry_value(manifest: &AppManifest) -> Option<String> {
    if let Some(entry_agent) = &manifest.entry_agent {
        return Some(entry_agent.clone());
    }
    manifest
        .entrypoint
        .as_ref()
        .map(|entrypoint| match entrypoint.type_ {
            EntrypointType::Workflow | EntrypointType::Agent | EntrypointType::Custom => {
                entrypoint.name.clone()
            }
        })
}

fn entry_kind(manifest: &AppManifest) -> Option<String> {
    if manifest.entry_agent.is_some() {
        return Some("agent".into());
    }
    manifest
        .entrypoint
        .as_ref()
        .map(|entrypoint| match entrypoint.type_ {
            EntrypointType::Workflow => "workflow".into(),
            EntrypointType::Agent => "agent".into(),
            EntrypointType::Custom => "custom".into(),
        })
}

fn sanitize_path_for_id(path: &str) -> String {
    path.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '.'
            }
        })
        .collect::<String>()
        .trim_matches('.')
        .to_string()
}

pub fn app_manifest_projection(manifest: &AppManifest) -> LegacyAppManifestProjection {
    YamlApplicationManifestAdapter::new(manifest.clone()).project()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AppLayer, CapabilityRef};

    #[test]
    fn yaml_projection_creates_agent_ability_for_inline_agent() {
        let manifest = AppManifest {
            id: macaca_proto::ApplicationId::new(),
            name: "fixture".into(),
            description: None,
            version: "1.0.0".into(),
            layer: AppLayer::L3Declarative,
            ui_type: None,
            agents: vec![AgentSource::Inline(InlineAgentConfig {
                name: "agent.fixture".into(),
                capabilities: vec![CapabilityRef {
                    name: "capability.fixture".into(),
                    description: "Fixture capability".into(),
                }],
                prompt_template: "never serialized into projection metadata".into(),
                model: "model.fixture".into(),
                permission_level: "user".into(),
                allowed_tools: vec!["tool.fixture".into()],
                max_tokens: None,
                temperature: None,
                skills: None,
                context_engine: None,
            })],
            llm_config: None,
            entry_agent: Some("agent.fixture".into()),
            entrypoint: None,
            workflows: None,
            resources: None,
            context: None,
            service_contract: None,
            execution_profile: None,
            workbench: None,
            autonomy: None,
            ui: None,
            execution_control: None,
        };

        let projection = YamlApplicationManifestAdapter::new(manifest).project();

        assert_eq!(projection.manifest.abilities.len(), 1);
        assert_eq!(
            projection.manifest.abilities[0].kind,
            ApplicationAbilityKind::Agent
        );
        assert!(projection.manifest.abilities[0]
            .capabilities
            .iter()
            .any(|capability| capability.id.as_str().contains("capability.fixture")));
        assert!(!projection
            .manifest
            .metadata
            .values()
            .any(|value| value.contains("never serialized")));
    }

    #[test]
    fn yaml_projection_synthesizes_wasm_runtime_ability_from_service_contract() {
        let manifest = AppManifest {
            id: macaca_proto::ApplicationId::new(),
            name: "wasm-fixture".into(),
            description: None,
            version: "1.0.0".into(),
            layer: AppLayer::L2Wasm,
            ui_type: None,
            agents: Vec::new(),
            llm_config: None,
            entry_agent: None,
            entrypoint: None,
            workflows: None,
            resources: None,
            context: None,
            service_contract: Some(crate::service_capability::AppServiceContractConfig {
                use_packs: vec!["pack.finance.v1".into()],
                required_services: vec!["service.custom.required".into()],
                optional_services: Vec::new(),
                service_policy_overrides: Default::default(),
            }),
            execution_profile: None,
            workbench: None,
            autonomy: None,
            ui: None,
            execution_control: None,
        };
        let projection = YamlApplicationManifestAdapter::new(manifest).project();
        let ability = projection
            .manifest
            .abilities
            .iter()
            .find(|ability| ability.id == "ability.runtime.wasm")
            .expect("expected synthesized wasm runtime ability");
        assert_eq!(ability.kind, ApplicationAbilityKind::Headless);
        assert_eq!(
            ability.implementation,
            AbilityImplementationKind::WasmComponent
        );
        assert!(ability
            .services
            .iter()
            .any(|service| service.service.as_str() == "service.custom.required"));
        assert!(
            !ability.services.iter().any(|service| {
                service.service.as_str() == "service.market_data"
            }),
            "finance pack services must not expand without an installed catalog entry"
        );
    }
}
