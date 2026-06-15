//! Public projection entry points (Facade API over Adapter + Projection pipeline).
//!
//! Each function adapts YAML via `YamlApplicationManifestAdapter`, then
//! delegates to specialized sub-projectors.  `tracing::info` records audit nodes at
//! metadata assembly boundaries without logging sensitive manifest payloads.

use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};
use std::path::Path;

use macaca_proto::{
    ApplicationAbilityDescriptor, ApplicationAbilityMetadataView,
    ApplicationContextPolicyMetadataView, ApplicationEntryMetadataView,
    ApplicationHeartbeatAgentView, ApplicationId, ApplicationManifestDigestView,
    ApplicationManifestV1, ApplicationMcpOverlayMetadataView, ApplicationMetadataView,
    ApplicationServiceAgentView, ApplicationServiceAppView, ApplicationServiceRuntimeView,
    ApplicationSkillPolicyMetadataView, ApplicationToolPolicyMetadataView, ApplicationUiBridgeView,
    ApplicationUiRuntimeView, ApplicationUiSandboxView, ApplicationUiSurfaceView,
    ApplicationUiThemeView, ApplicationWorkbenchMetadataView, PackageRuntimeKind,
};
use tracing::info;

use crate::consumption::app_entry_agent_name;
use crate::manifest_v1::{YamlApplicationManifestAdapter, YamlApplicationManifestProjection};
use crate::model::{AgentSource, AppManifest, AppStatus};
use crate::service_capability::{
    expand_service_capabilities, DomainPackCatalog, InMemoryDomainPackCatalog,
};
use crate::ApplicationRuntimeKindSpec;

use super::app_view::manifest_v1_to_service_app_view;
use super::policy::{
    ability_view, context_policy_view, mcp_overlay_view, skill_policy_view, tool_policy_view,
    workbench_metadata_view,
};
use super::support::{digest_view, entry_view};

/// Project an application manifest into the application summary view.
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
    app_manifest_to_service_app_view_with_catalog(
        manifest,
        app_dir,
        status,
        &InMemoryDomainPackCatalog::with_builtin_defaults(),
    )
}

/// Project an application manifest using a host-installed domain-pack catalog.
///
/// Composition roots pass the same catalog wired into `AppRuntime` so service
/// diagnostics, WASM policy sync, and UI allowlists observe identical pack
/// resolution results.
pub fn app_manifest_to_service_app_view_with_catalog(
    manifest: &AppManifest,
    app_dir: Option<&Path>,
    status: AppStatus,
    catalog: &dyn DomainPackCatalog,
) -> ApplicationServiceAppView {
    let projection =
        YamlApplicationManifestAdapter::new(manifest.clone()).project_with_catalog(catalog);
    manifest_v1_to_service_app_view(&projection.manifest, manifest, app_dir, status, catalog)
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
    app_manifest_to_metadata_view_with_catalog(
        manifest,
        app_dir,
        status,
        include_abilities,
        include_policy,
        include_overlay,
        include_digest,
        &InMemoryDomainPackCatalog::with_builtin_defaults(),
    )
}

/// Project metadata using a host-installed domain-pack catalog.
pub fn app_manifest_to_metadata_view_with_catalog(
    manifest: &AppManifest,
    app_dir: Option<&Path>,
    status: AppStatus,
    include_abilities: bool,
    include_policy: bool,
    include_overlay: bool,
    include_digest: bool,
    catalog: &dyn DomainPackCatalog,
) -> ApplicationMetadataView {
    let projection =
        YamlApplicationManifestAdapter::new(manifest.clone()).project_with_catalog(catalog);
    let application =
        manifest_v1_to_service_app_view(&projection.manifest, manifest, app_dir, status, catalog);
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
        .chain(projection.report.projection_warnings.iter())
        .chain(projection.report.source_only_fields.iter())
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
        workbench: workbench_metadata_view(&projection.manifest),
        execution_control: projection.manifest.execution_control.clone(),
        manifest_digest,
        diagnostics,
    }
}
