//! Service app summary projection (Adapter over Manifest v1 + legacy model).
//!
//! Builds `ApplicationServiceAppView` with runtime diagnostics and optional UI
//! metadata.  Capability expansion uses the injected `DomainPackCatalog` Strategy.

use std::path::Path;

use macaca_proto::{
    ApplicationManifestV1, ApplicationServiceAppView, ApplicationServiceRuntimeView,
    PackageRuntimeKind,
};

use crate::model::{AppManifest, AppStatus};
use crate::service_capability::{expand_service_capabilities, DomainPackCatalog};

use super::support::{agent_views, entry_view, runtime_diagnostics, skills_dir};
use super::ui::ui_runtime_view;

/// Project Manifest v1 + legacy model into the bounded service app summary view.
pub(super) fn manifest_v1_to_service_app_view(
    manifest_v1: &ApplicationManifestV1,
    legacy: &AppManifest,
    app_dir: Option<&Path>,
    status: AppStatus,
    catalog: &dyn DomainPackCatalog,
) -> ApplicationServiceAppView {
    // Resolve effective service capabilities for metadata projection only.
    // This keeps app discovery and UI surfaces deterministic without invoking
    // any runtime providers or executing any service calls.
    let capabilities =
        expand_service_capabilities(legacy.service_contract.as_ref(), catalog);
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
