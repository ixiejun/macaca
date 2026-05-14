//! Sanitized Application Service projections owned by Application Framework.
//!
//! This module applies the Projection and Adapter patterns.  Raw YAML
//! application manifests are first adapted into Manifest v1, then projected
//! into bounded service DTOs for Web, CLI, Gateway, and framework adapters.
//! The projection deliberately exposes only identifiers, names, counts,
//! declared capability names, policy presence flags, and digests.  It never
//! copies prompt bodies, raw manifest bodies, raw agent configs, environment
//! values, API keys, secrets, private keys, raw host payloads, or unbounded
//! user input into service views or logs.

use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};
use std::path::Path;

use macaca_proto::{
    ApplicationAbilityDescriptor, ApplicationAbilityMetadataView,
    ApplicationContextPolicyMetadataView, ApplicationEntryMetadataView, ApplicationId,
    ApplicationManifestDigestView, ApplicationManifestV1, ApplicationMcpOverlayMetadataView,
    ApplicationMetadataView, ApplicationServiceAgentView, ApplicationServiceAppView,
    ApplicationServiceRuntimeView, ApplicationSkillPolicyMetadataView,
    ApplicationToolPolicyMetadataView, ApplicationUiBridgeView, ApplicationUiRuntimeView,
    ApplicationUiSandboxView, ApplicationUiSurfaceView, ApplicationUiThemeView, PackageRuntimeKind,
};
use tracing::info;

use crate::consumption::app_entry_agent_name;
use crate::manifest_v1::{LegacyAppManifestProjection, YamlApplicationManifestAdapter};
use crate::model::{AgentSource, AppManifest, AppStatus};
use crate::service_capability::{expand_service_capabilities, InMemoryDomainPackCatalog};
use crate::ui_runtime::{
    AppUiCspMode, AppUiFramework, AppUiNetworkPolicy, AppUiRuntimeKind, AppUiSandboxIsolation,
    AppUiSurfaceChrome, AppUiSurfaceMode, AppUiThemeMode,
};
use crate::ApplicationRuntimeKindSpec;

/// Project an application manifest into the legacy app summary view.
///
/// This function is intentionally metadata-only.  It does not resolve file
/// agents, instantiate runtimes, read prompts, inspect providers, or execute
/// tools.  Callers that need resolved agent execution still use the existing
/// runtime paths while shells can use this view for safe presentation.
pub fn app_manifest_to_service_app_view(
    manifest: &AppManifest,
    app_dir: Option<&Path>,
    status: AppStatus,
) -> ApplicationServiceAppView {
    let projection = YamlApplicationManifestAdapter::new(manifest.clone()).project();
    manifest_v1_to_service_app_view(&projection.manifest, manifest, app_dir, status)
}

/// Project an application manifest into the complete sanitized metadata view.
///
/// The returned memento can cross service boundaries safely because every
/// field is bounded and derived from declarations rather than runtime/provider
/// payloads.  The `include_*` flags let future remote transports avoid sending
/// unused optional sections without creating new command shapes.
pub fn app_manifest_to_metadata_view(
    manifest: &AppManifest,
    app_dir: Option<&Path>,
    status: AppStatus,
    include_abilities: bool,
    include_policy: bool,
    include_overlay: bool,
    include_digest: bool,
) -> ApplicationMetadataView {
    let projection = YamlApplicationManifestAdapter::new(manifest.clone()).project();
    let application =
        manifest_v1_to_service_app_view(&projection.manifest, manifest, app_dir, status);
    let entry = entry_view(&projection.manifest, manifest);
    let abilities = if include_abilities {
        projection
            .manifest
            .abilities
            .iter()
            .map(ability_view)
            .collect()
    } else {
        Vec::new()
    };
    let (tool_policy, context_policy, skill_policy, mcp_overlay) = if include_policy {
        (
            tool_policy_view(&projection.manifest),
            context_policy_view(manifest, &projection.manifest),
            skill_policy_view(manifest),
            if include_overlay {
                mcp_overlay_view(manifest)
            } else {
                ApplicationMcpOverlayMetadataView::default()
            },
        )
    } else {
        (
            ApplicationToolPolicyMetadataView::default(),
            ApplicationContextPolicyMetadataView::default(),
            ApplicationSkillPolicyMetadataView::default(),
            ApplicationMcpOverlayMetadataView::default(),
        )
    };
    let manifest_digest = include_digest.then(|| digest_view(manifest, &projection));
    let diagnostics = projection
        .report
        .inferred_defaults
        .iter()
        .chain(projection.report.compatibility_warnings.iter())
        .chain(projection.report.legacy_only_fields.iter())
        .map(|diagnostic| format!("{}:{}", diagnostic.code, diagnostic.subject))
        .collect();

    info!(
        app_id = %application.id,
        ability_count = abilities.len(),
        digest_included = manifest_digest.is_some(),
        "projected sanitized application metadata view"
    );

    ApplicationMetadataView {
        application,
        entry,
        abilities,
        tool_policy,
        context_policy,
        skill_policy,
        mcp_overlay,
        manifest_digest,
        diagnostics,
    }
}

fn manifest_v1_to_service_app_view(
    manifest_v1: &ApplicationManifestV1,
    legacy: &AppManifest,
    app_dir: Option<&Path>,
    status: AppStatus,
) -> ApplicationServiceAppView {
    // Resolve effective service capabilities for metadata projection only.
    // This keeps app discovery and UI surfaces deterministic without invoking
    // any runtime providers or executing any service calls.
    let capabilities = expand_service_capabilities(
        legacy.service_contract.as_ref(),
        &InMemoryDomainPackCatalog::with_builtin_defaults(),
    );
    let runtime_kind = Some(manifest_v1.runtime.kind.clone());
    let lifecycle_state = crate::lifecycle_from_app_status(status);
    ApplicationServiceAppView {
        id: legacy.id,
        name: manifest_v1.name.clone(),
        version: manifest_v1.version.clone(),
        description: legacy.description.clone(),
        entry_agent: entry_view(manifest_v1, legacy).agent_name,
        agents: agent_views(manifest_v1, legacy),
        runtime: ApplicationServiceRuntimeView {
            runtime_kind,
            lifecycle_state,
            compatibility_status: format!("{status:?}"),
            app_dir: app_dir.map(|path| path.display().to_string()),
            skills_dir: skills_dir(app_dir, legacy),
        },
        ui: ui_runtime_view(legacy),
        diagnostics: runtime_diagnostics(legacy, &manifest_v1.runtime.kind, &capabilities),
    }
}

fn ui_runtime_view(legacy: &AppManifest) -> Option<ApplicationUiRuntimeView> {
    let ui = legacy.ui.as_ref()?;
    let entry_url = format!("/api/apps/{}/ui/assets/{}", legacy.id, ui.entry);
    tracing::info!(
        app_id = %legacy.id,
        ui_runtime = %ui_runtime_kind_label(ui.runtime),
        surface_mode = %ui_surface_mode_label(ui.surface.mode),
        surface_chrome = %ui_surface_chrome_label(ui.surface.chrome),
        bridge_required = ui.bridge.required.len(),
        bridge_optional = ui.bridge.optional.len(),
        "projected sanitized application-owned UI metadata"
    );
    Some(ApplicationUiRuntimeView {
        runtime: ui_runtime_kind_label(ui.runtime).to_string(),
        surface: ApplicationUiSurfaceView {
            mode: ui_surface_mode_label(ui.surface.mode).to_string(),
            chrome: ui_surface_chrome_label(ui.surface.chrome).to_string(),
        },
        framework: ui.framework.map(ui_framework_label).map(str::to_string),
        entry_url,
        sandbox: ApplicationUiSandboxView {
            isolation: ui_sandbox_isolation_label(ui.sandbox.isolation).to_string(),
            csp: ui_csp_label(ui.sandbox.csp).to_string(),
            network: ui_network_label(ui.sandbox.network).to_string(),
        },
        bridge: ApplicationUiBridgeView {
            required: ui.bridge.required.clone(),
            optional: ui.bridge.optional.clone(),
        },
        theme: ApplicationUiThemeView {
            mode: ui_theme_label(ui.theme.mode).to_string(),
        },
    })
}

fn ui_runtime_kind_label(kind: AppUiRuntimeKind) -> &'static str {
    match kind {
        AppUiRuntimeKind::WebBundle => "web_bundle",
    }
}

fn ui_framework_label(framework: AppUiFramework) -> &'static str {
    match framework {
        AppUiFramework::React => "react",
        AppUiFramework::Vue => "vue",
        AppUiFramework::Svelte => "svelte",
        AppUiFramework::Vanilla => "vanilla",
        AppUiFramework::Other => "other",
    }
}

fn ui_surface_mode_label(mode: AppUiSurfaceMode) -> &'static str {
    match mode {
        AppUiSurfaceMode::Application => "application",
        AppUiSurfaceMode::Session => "session",
    }
}

fn ui_surface_chrome_label(chrome: AppUiSurfaceChrome) -> &'static str {
    match chrome {
        AppUiSurfaceChrome::AppOwned => "app_owned",
        AppUiSurfaceChrome::Host => "host",
    }
}

fn ui_sandbox_isolation_label(isolation: AppUiSandboxIsolation) -> &'static str {
    match isolation {
        AppUiSandboxIsolation::Iframe => "iframe",
    }
}

fn ui_csp_label(csp: AppUiCspMode) -> &'static str {
    match csp {
        AppUiCspMode::Strict => "strict",
    }
}

fn ui_network_label(network: AppUiNetworkPolicy) -> &'static str {
    match network {
        AppUiNetworkPolicy::Denied => "denied",
        AppUiNetworkPolicy::Declared => "declared",
    }
}

fn ui_theme_label(mode: AppUiThemeMode) -> &'static str {
    match mode {
        AppUiThemeMode::AppOwned => "app_owned",
        AppUiThemeMode::HostAdaptive => "host_adaptive",
    }
}

fn agent_views(
    manifest_v1: &ApplicationManifestV1,
    legacy: &AppManifest,
) -> Vec<ApplicationServiceAgentView> {
    let mut views: Vec<_> = legacy
        .agents
        .iter()
        .map(|agent| ApplicationServiceAgentView {
            name: agent_name(agent),
            capability_names: Vec::new(),
        })
        .collect();
    for ability in manifest_v1.abilities.iter() {
        if let Some(name) = ability_agent_name(ability) {
            let capability_names = ability
                .capabilities
                .iter()
                .map(|capability| capability.id.as_str().to_string())
                .collect();
            if let Some(existing) = views.iter_mut().find(|view| view.name == name) {
                existing.capability_names = capability_names;
            } else {
                views.push(ApplicationServiceAgentView {
                    name,
                    capability_names,
                });
            }
        }
    }
    views.sort_by(|left, right| left.name.cmp(&right.name));
    views.dedup_by(|left, right| left.name == right.name);
    views
}

fn entry_view(
    manifest_v1: &ApplicationManifestV1,
    legacy: &AppManifest,
) -> ApplicationEntryMetadataView {
    ApplicationEntryMetadataView {
        agent_name: app_entry_agent_name(legacy)
            .map(str::to_string)
            .or_else(|| manifest_v1.runtime.entry.clone()),
        entry_kind: manifest_v1.runtime.metadata.get("entry.kind").cloned(),
        activation_mode: manifest_v1
            .abilities
            .iter()
            .flat_map(|ability| ability.activation.iter())
            .find(|activation| activation.entry == manifest_v1.runtime.entry)
            .map(|activation| activation.mode.clone()),
    }
}

fn ability_view(ability: &ApplicationAbilityDescriptor) -> ApplicationAbilityMetadataView {
    ApplicationAbilityMetadataView {
        id: ability.id.clone(),
        name: ability_agent_name(ability).unwrap_or_else(|| ability.id.clone()),
        kind: format!("{:?}", ability.kind).to_lowercase(),
        implementation: format!("{:?}", ability.implementation).to_lowercase(),
        is_entry: ability
            .metadata
            .get("entry")
            .is_some_and(|value| value == "true"),
        activation_modes: ability
            .activation
            .iter()
            .map(|activation| activation.mode.clone())
            .collect(),
        capability_names: ability
            .capabilities
            .iter()
            .map(|capability| capability.id.as_str().to_string())
            .collect(),
        required_services: ability
            .services
            .iter()
            .filter(|service| !service.optional)
            .map(|service| service.service.as_str().to_string())
            .collect(),
        permission_names: ability
            .permissions
            .iter()
            .filter(|permission| !permission.optional)
            .map(|permission| permission.name.clone())
            .collect(),
    }
}

fn tool_policy_view(manifest_v1: &ApplicationManifestV1) -> ApplicationToolPolicyMetadataView {
    let mut names = BTreeSet::new();
    for ability in &manifest_v1.abilities {
        for capability in &ability.capabilities {
            if let Some(name) = capability.id.as_str().strip_prefix("tool.") {
                names.insert(name.to_string());
            }
        }
    }
    let execution_tool_count = names
        .iter()
        .filter(|name| name.ends_with("_execute"))
        .count();
    ApplicationToolPolicyMetadataView {
        declared_tool_names: names.into_iter().collect(),
        execution_tool_count,
    }
}

fn context_policy_view(
    legacy: &AppManifest,
    manifest_v1: &ApplicationManifestV1,
) -> ApplicationContextPolicyMetadataView {
    ApplicationContextPolicyMetadataView {
        context_config_present: legacy.context.is_some(),
        context_engine_declared: manifest_v1.abilities.iter().any(|ability| {
            ability
                .metadata
                .get("context.engine.present")
                .is_some_and(|value| value == "true")
        }),
    }
}

fn skill_policy_view(legacy: &AppManifest) -> ApplicationSkillPolicyMetadataView {
    let mut names: Vec<_> = legacy
        .agents
        .iter()
        .filter_map(|source| match source {
            AgentSource::Inline(inline) if inline.skills.is_some() => Some(inline.name.clone()),
            _ => None,
        })
        .collect();
    names.sort();
    names.dedup();
    ApplicationSkillPolicyMetadataView {
        agents_with_skill_policy: names,
    }
}

fn mcp_overlay_view(legacy: &AppManifest) -> ApplicationMcpOverlayMetadataView {
    let agents = skill_policy_view(legacy).agents_with_skill_policy;
    ApplicationMcpOverlayMetadataView {
        overlay_declared: !agents.is_empty(),
        agents_with_overlay: agents,
    }
}

fn digest_view(
    legacy: &AppManifest,
    projection: &LegacyAppManifestProjection,
) -> ApplicationManifestDigestView {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    legacy.id.hash(&mut hasher);
    legacy.name.hash(&mut hasher);
    legacy.version.hash(&mut hasher);
    legacy.agents.len().hash(&mut hasher);
    projection.manifest.abilities.len().hash(&mut hasher);
    for ability in &projection.manifest.abilities {
        ability.id.hash(&mut hasher);
    }
    ApplicationManifestDigestView {
        algorithm: "std-default-hasher-v1".into(),
        digest: format!("{:016x}", hasher.finish()),
        source_format: projection
            .manifest
            .metadata
            .get("source.format")
            .cloned()
            .unwrap_or_else(|| "unknown".into()),
        ability_count: projection.manifest.abilities.len(),
        agent_count: legacy.agents.len(),
    }
}

fn runtime_diagnostics(
    legacy: &AppManifest,
    runtime_kind: &PackageRuntimeKind,
    capabilities: &crate::service_capability::EffectiveServiceCapabilities,
) -> Vec<String> {
    // Diagnostics are intentionally bounded plain strings. They should help
    // operators debug admission/runtime readiness without leaking payload data.
    let mut diagnostics = vec![format!(
        "effective_service_capabilities_hash={}",
        capabilities.capabilities_hash
    )];
    diagnostics.push(format!(
        "effective_service_count={}",
        capabilities.services.len()
    ));
    if !capabilities.unresolved_packs.is_empty() {
        diagnostics.push(format!(
            "unresolved_domain_packs={}",
            capabilities.unresolved_packs.join(",")
        ));
    }
    let spec = ApplicationRuntimeKindSpec;
    if spec.execution_available_for_runtime(Some(runtime_kind)) {
        diagnostics
    } else {
        diagnostics.push(format!("runtime unavailable for {:?}", legacy.layer));
        diagnostics
    }
}

fn skills_dir(app_dir: Option<&Path>, legacy: &AppManifest) -> Option<String> {
    if let Some(resources) = &legacy.resources {
        if let Some(skills) = &resources.skills {
            return Some(
                app_dir
                    .map(|dir| dir.join(skills).display().to_string())
                    .unwrap_or_else(|| skills.clone()),
            );
        }
    }
    app_dir.map(|path| path.join("skills").display().to_string())
}

fn agent_name(agent: &AgentSource) -> String {
    match agent {
        AgentSource::Inline(inline) => inline.name.clone(),
        AgentSource::FilePath(path) => path.clone(),
    }
}

fn ability_agent_name(ability: &ApplicationAbilityDescriptor) -> Option<String> {
    ability
        .activation
        .iter()
        .find(|activation| activation.mode == "agent")
        .and_then(|activation| activation.entry.clone())
}

#[allow(dead_code)]
fn _assert_application_id_copy(_: ApplicationId) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AppLayer, CapabilityRef, InlineAgentConfig};

    fn fixture_manifest() -> AppManifest {
        AppManifest {
            id: ApplicationId::new(),
            name: "safe-app".into(),
            description: Some("Safe app".into()),
            version: "1.0.0".into(),
            layer: AppLayer::L3Declarative,
            ui_type: None,
            agents: vec![AgentSource::Inline(InlineAgentConfig {
                name: "coordinator".into(),
                capabilities: vec![CapabilityRef {
                    name: "plan".into(),
                    description: "Plans work".into(),
                }],
                prompt_template: "SECRET_PROMPT_BODY_SHOULD_NOT_LEAK".into(),
                model: "provider-model".into(),
                permission_level: "admin".into(),
                allowed_tools: vec!["shell_execute".into(), "read_file".into()],
                max_tokens: None,
                temperature: None,
                skills: Some(Default::default()),
                context_engine: Some("private-context-engine".into()),
            })],
            llm_config: None,
            entry_agent: Some("coordinator".into()),
            entrypoint: None,
            workflows: None,
            resources: None,
            context: Some(Default::default()),
            service_contract: None,
            ui: None,
        }
    }

    #[test]
    fn sanitized_metadata_excludes_prompt_and_secret_like_values() {
        let manifest = fixture_manifest();
        let view = app_manifest_to_metadata_view(
            &manifest,
            None,
            AppStatus::Loaded,
            true,
            true,
            true,
            true,
        );
        let encoded = serde_json::to_string(&view).unwrap();
        assert!(!encoded.contains("SECRET_PROMPT_BODY_SHOULD_NOT_LEAK"));
        assert!(!encoded.contains("provider-model"));
        assert!(!encoded.contains("admin"));
        assert!(!encoded.contains("private-context-engine"));
        assert_eq!(view.entry.agent_name.as_deref(), Some("coordinator"));
        assert_eq!(view.tool_policy.execution_tool_count, 1);
    }

    #[test]
    fn service_app_view_preserves_safe_agent_and_capability_names() {
        let manifest = fixture_manifest();
        let view = app_manifest_to_service_app_view(&manifest, None, AppStatus::Loaded);
        assert_eq!(view.agents.len(), 1);
        assert_eq!(view.agents[0].name, "coordinator");
        assert!(view.agents[0]
            .capability_names
            .iter()
            .any(|capability| capability.contains("plan")));
    }

    #[test]
    fn service_app_view_contains_effective_service_diagnostics() {
        let mut manifest = fixture_manifest();
        manifest.service_contract = Some(crate::service_capability::AppServiceContractConfig {
            use_packs: vec!["pack.finance.v1".into()],
            required_services: vec!["service.custom.required".into()],
            optional_services: vec![],
            service_policy_overrides: Default::default(),
        });
        let view = app_manifest_to_service_app_view(&manifest, None, AppStatus::Loaded);
        assert!(view
            .diagnostics
            .iter()
            .any(|line| line.starts_with("effective_service_capabilities_hash=")));
        assert!(view
            .diagnostics
            .iter()
            .any(|line| line == "effective_service_count=5"));
    }
}
