//! Core YAML → Manifest v1 projection orchestration.
//!
//! `project_with_catalog` applies the **Adapter** pattern: it walks legacy YAML
//! fields, emits sanitized Manifest v1 metadata, and records a Memento-style
//! report.  Critical execution nodes are logged via `tracing::info` for audit.

use macaca_proto::{
    ApplicationCompatibilityDeclaration, ApplicationManifestV1, DeveloperId, PackageId,
    PackageRuntimeKind,
};
use tracing::info;

use crate::service_capability::{DomainPackCatalog, InMemoryDomainPackCatalog};

use super::entry::{application_permissions, entry_kind, entry_value};
use super::types::{
    LegacyAppManifestProjection, YamlApplicationManifestAdapter,
    YamlToApplicationManifestV1Report,
};

impl YamlApplicationManifestAdapter {
    /// Project YAML application data into Manifest v1 using an empty catalog.
    ///
    /// Convenience wrapper that injects builtin-default domain packs so unit
    /// tests and simple callers do not need to construct a catalog explicitly.
    pub fn project(self) -> LegacyAppManifestProjection {
        self.project_with_catalog(&InMemoryDomainPackCatalog::with_builtin_defaults())
    }

    /// Project YAML application data using a host-installed domain-pack catalog.
    ///
    /// WASM runtime ability synthesis expands `service_contract.use_packs`
    /// through the injected catalog so composition roots can register optional
    /// domain extensions without embedding pack ids in this adapter.
    pub fn project_with_catalog(
        self,
        catalog: &dyn DomainPackCatalog,
    ) -> LegacyAppManifestProjection {
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

        let mut runtime = macaca_proto::ApplicationRuntimeProfile::new(
            PackageRuntimeKind::Yaml,
            "1",
        );
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
}
