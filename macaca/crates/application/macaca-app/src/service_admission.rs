//! Admission specifications for the Route C Application Service.
//!
//! The module applies the Specification pattern to service inputs.  Keeping
//! trace, scope, manifest, and runtime-kind checks in small reusable specs
//! prevents Web, runtime-host, and `AppRuntime` from copying ad-hoc validation
//! branches as the Application Service grows.

use macaca_proto::{
    ApplicationAbilityDescriptor, ApplicationLifecycleState, ApplicationManifestV1, MacacaError,
    MacacaResult, PackageRuntimeKind, TraceContext,
};

use crate::model::{AppLayer, AppManifest, AppStatus};
use crate::ui_runtime::validate_ui_runtime_config;

/// Specification that enforces Route C trace requirements.
#[derive(Debug, Default, Clone, Copy)]
pub struct ApplicationTraceSpec;

impl ApplicationTraceSpec {
    /// Validate that a command is traceable before provider dispatch.
    pub fn validate(&self, trace: &TraceContext) -> MacacaResult<()> {
        if trace.trace_id.trim().is_empty() {
            return Err(MacacaError::Config(
                "application service command requires trace_id".into(),
            ));
        }
        Ok(())
    }
}

/// Specification for application/session scope.
#[derive(Debug, Default, Clone, Copy)]
pub struct ApplicationScopeSpec;

impl ApplicationScopeSpec {
    /// Validate non-empty session scope for session commands.
    pub fn validate_session(&self, session_id: Option<&str>) -> MacacaResult<()> {
        let Some(session_id) = session_id.map(str::trim).filter(|value| !value.is_empty()) else {
            return Err(MacacaError::Config(
                "application service session command requires session_id".into(),
            ));
        };
        tracing::debug!(session_id, "application service session scope accepted");
        Ok(())
    }
}

/// Specification for manifest admission.
#[derive(Debug, Default, Clone, Copy)]
pub struct ApplicationManifestSpec;

impl ApplicationManifestSpec {
    /// Validate manifest fields without logging prompt bodies or full manifests.
    pub fn validate(&self, manifest: &AppManifest) -> MacacaResult<()> {
        if manifest.name.trim().is_empty() {
            return Err(MacacaError::Config(
                "application manifest name must not be empty".into(),
            ));
        }
        if let Some(service_contract) = &manifest.service_contract {
            for service in &service_contract.required_services {
                if service.trim().is_empty() {
                    return Err(MacacaError::Config(
                        "service_contract.required_services contains an empty identifier".into(),
                    ));
                }
            }
            for service in &service_contract.optional_services {
                if service.trim().is_empty() {
                    return Err(MacacaError::Config(
                        "service_contract.optional_services contains an empty identifier".into(),
                    ));
                }
            }
        }
        if let Some(workbench) = &manifest.workbench {
            validate_workbench_declaration(workbench, "application_manifest")?;
        }
        if let Some(execution_profile) = &manifest.execution_profile {
            execution_profile.validate()?;
        }
        validate_ui_runtime_config(manifest.ui.as_ref())?;
        tracing::info!(
            service_id = "application.admission",
            command = "admit_manifest",
            application_id = %manifest.id,
            layer = ?manifest.layer,
            service_contract_declared = manifest.service_contract.is_some(),
            execution_profile_declared = manifest.execution_profile.is_some(),
            workbench_declared = manifest.workbench.is_some(),
            ui_runtime_declared = manifest.ui.is_some(),
            "application manifest admitted by service specification"
        );
        Ok(())
    }
}

/// Specification for runtime kind execution behavior.
#[derive(Debug, Default, Clone, Copy)]
pub struct ApplicationRuntimeKindSpec;

impl ApplicationRuntimeKindSpec {
    /// Return whether execution is available for a manifest layer.
    pub fn execution_available_for_layer(&self, layer: AppLayer) -> bool {
        !matches!(layer, AppLayer::L2Wasm)
    }

    /// Return whether execution is available for an ABI/package runtime kind.
    pub fn execution_available_for_runtime(&self, runtime: Option<&PackageRuntimeKind>) -> bool {
        !matches!(runtime, Some(PackageRuntimeKind::WasmComponent))
    }
}

/// Sanitized diagnostic emitted by Manifest v1 and Ability admission.
///
/// The diagnostic deliberately stores identifiers, kinds, and reason codes
/// only.  It must never embed raw manifest bodies, prompt content, secrets, env
/// values, API keys, or host payloads because admission reports are safe to log
/// and may later be shown in developer tooling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationAdmissionDiagnostic {
    pub code: String,
    pub subject: String,
    pub message: String,
}

impl ApplicationAdmissionDiagnostic {
    pub fn new(
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

/// Memento-style validation report for Application Platform admission.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ApplicationAdmissionReport {
    pub diagnostics: Vec<ApplicationAdmissionDiagnostic>,
}

impl ApplicationAdmissionReport {
    pub fn is_success(&self) -> bool {
        self.diagnostics.is_empty()
    }

    pub fn push(&mut self, diagnostic: ApplicationAdmissionDiagnostic) {
        self.diagnostics.push(diagnostic);
    }
}

/// Specification for Application Manifest v1 declarations.
#[derive(Debug, Default, Clone, Copy)]
pub struct ApplicationManifestV1Spec;

impl ApplicationManifestV1Spec {
    /// Validate the Manifest v1 shape without executing the application.
    pub fn validate(&self, manifest: &ApplicationManifestV1) -> ApplicationAdmissionReport {
        let mut report = ApplicationAdmissionReport::default();
        if manifest.name.trim().is_empty() {
            report.push(ApplicationAdmissionDiagnostic::new(
                "missing_application_name",
                manifest.package_id.as_str(),
                "Application Manifest v1 requires a non-empty application name",
            ));
        }
        if manifest.abilities.is_empty() {
            report.push(ApplicationAdmissionDiagnostic::new(
                "missing_ability",
                manifest.package_id.as_str(),
                "Application Manifest v1 requires at least one ability descriptor",
            ));
        }
        for ability in &manifest.abilities {
            merge_report(&mut report, ApplicationAbilitySpec.validate(ability));
        }
        if let Some(workbench) = &manifest.workbench {
            if let Err(error) =
                validate_workbench_declaration(workbench, manifest.package_id.as_str())
            {
                report.push(ApplicationAdmissionDiagnostic::new(
                    "invalid_workbench_declaration",
                    manifest.package_id.as_str(),
                    error.to_string(),
                ));
            }
        }
        if let Some(execution_profile) = &manifest.execution_profile {
            if let Err(error) = execution_profile.validate() {
                report.push(ApplicationAdmissionDiagnostic::new(
                    "invalid_application_execution_profile",
                    manifest.package_id.as_str(),
                    error.to_string(),
                ));
            }
        }
        tracing::info!(
            package_id = %manifest.package_id,
            ability_count = manifest.abilities.len(),
            execution_profile_declared = manifest.execution_profile.is_some(),
            workbench_declared = manifest.workbench.is_some(),
            diagnostic_count = report.diagnostics.len(),
            "application manifest v1 admission evaluated"
        );
        report
    }
}

/// Specification for individual ability descriptors.
#[derive(Debug, Default, Clone, Copy)]
pub struct ApplicationAbilitySpec;

impl ApplicationAbilitySpec {
    /// Validate ability declarations while preserving provider-neutrality.
    pub fn validate(&self, ability: &ApplicationAbilityDescriptor) -> ApplicationAdmissionReport {
        let mut report = ApplicationAdmissionReport::default();
        if ability.id.trim().is_empty() {
            report.push(ApplicationAdmissionDiagnostic::new(
                "missing_ability_id",
                format!("{:?}", ability.kind),
                "Ability descriptor requires a stable id",
            ));
        }
        for service in &ability.services {
            if service.reason.trim().is_empty() {
                report.push(ApplicationAdmissionDiagnostic::new(
                    "missing_service_reason",
                    service.service.as_str(),
                    "Ability service requirements must explain why the service is needed",
                ));
            }
        }
        for permission in &ability.permissions {
            if permission.reason.trim().is_empty() {
                report.push(ApplicationAdmissionDiagnostic::new(
                    "missing_permission_reason",
                    &permission.name,
                    "Ability permissions must explain why the permission is needed",
                ));
            }
        }
        report
    }
}

fn merge_report(target: &mut ApplicationAdmissionReport, source: ApplicationAdmissionReport) {
    target.diagnostics.extend(source.diagnostics);
}

/// Validate a workbench declaration with the Specification pattern.
///
/// The spec checks only provider-neutral identifiers and reasons. It does not
/// resolve concrete providers, inspect package bytes, read plugin/MCP/skill
/// payloads, or branch on application/workflow names; later services perform
/// those side-effecting checks through traceable command boundaries.
fn validate_workbench_declaration(
    declaration: &macaca_proto::ApplicationWorkbenchManifestDeclaration,
    subject: &str,
) -> MacacaResult<()> {
    for capability in &declaration.capabilities {
        require_text(&capability.family, "workbench capability family", subject)?;
        require_text(&capability.reason, "workbench capability reason", subject)?;
    }
    for profile in &declaration.permission_profiles {
        require_text(profile, "workbench permission profile", subject)?;
    }
    for family in &declaration.tool_families {
        require_text(family, "workbench tool family", subject)?;
    }
    for service in &declaration.service_dependencies {
        require_text(
            service.service.as_str(),
            "workbench service dependency",
            subject,
        )?;
        require_text(
            &service.reason,
            "workbench service dependency reason",
            subject,
        )?;
    }
    for provider in &declaration.optional_provider_requirements {
        require_text(
            &provider.provider_kind,
            "workbench optional provider kind",
            subject,
        )?;
        require_text(
            &provider.reason,
            "workbench optional provider reason",
            subject,
        )?;
    }
    for plugin in &declaration.plugin_dependencies {
        require_text(&plugin.plugin_id, "workbench plugin dependency", subject)?;
        require_text(
            &plugin.reason,
            "workbench plugin dependency reason",
            subject,
        )?;
    }
    for dependency in &declaration.mcp_dependencies {
        require_text(&dependency.server_id, "workbench mcp dependency", subject)?;
        require_text(
            &dependency.reason,
            "workbench mcp dependency reason",
            subject,
        )?;
    }
    for bundle in &declaration.skill_bundles {
        require_text(&bundle.bundle_id, "workbench skill bundle", subject)?;
        require_text(&bundle.reason, "workbench skill bundle reason", subject)?;
    }
    for subscription in &declaration.event_subscriptions {
        require_text(&subscription.topic, "workbench event subscription", subject)?;
        require_text(
            &subscription.reason,
            "workbench event subscription reason",
            subject,
        )?;
    }
    for surface in &declaration.ui_surfaces {
        require_text(&surface.surface_id, "workbench ui surface", subject)?;
        require_text(&surface.schema, "workbench ui surface schema", subject)?;
    }
    tracing::info!(
        subject,
        capability_count = declaration.capabilities.len(),
        service_dependency_count = declaration.service_dependencies.len(),
        event_subscription_count = declaration.event_subscriptions.len(),
        "application workbench manifest declaration admitted"
    );
    Ok(())
}

fn require_text(value: &str, field: &str, subject: &str) -> MacacaResult<()> {
    if value.trim().is_empty() {
        return Err(MacacaError::Config(format!(
            "{field} must not be empty for {subject}"
        )));
    }
    Ok(())
}

/// Project the legacy `AppStatus` view into the ABI lifecycle vocabulary.
pub fn lifecycle_from_app_status(status: AppStatus) -> ApplicationLifecycleState {
    match status {
        AppStatus::Loaded => ApplicationLifecycleState::Initialized,
        AppStatus::Running => ApplicationLifecycleState::Started,
        AppStatus::Stopped => ApplicationLifecycleState::Stopped,
        AppStatus::Failed => ApplicationLifecycleState::Failed {
            reason: "legacy app status error".into(),
        },
    }
}

/// Project the ABI lifecycle state back to the legacy route status view.
pub fn app_status_from_lifecycle(state: &ApplicationLifecycleState) -> AppStatus {
    match state {
        ApplicationLifecycleState::Declared | ApplicationLifecycleState::Initialized => {
            AppStatus::Loaded
        }
        ApplicationLifecycleState::Started | ApplicationLifecycleState::Resumed => {
            AppStatus::Running
        }
        ApplicationLifecycleState::Paused
        | ApplicationLifecycleState::ShuttingDown
        | ApplicationLifecycleState::Stopped => AppStatus::Stopped,
        ApplicationLifecycleState::Failed { .. } => AppStatus::Failed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service_capability::AppServiceContractConfig;

    #[test]
    fn lifecycle_projection_preserves_running_status() {
        let state = lifecycle_from_app_status(AppStatus::Running);
        assert_eq!(state, ApplicationLifecycleState::Started);
        assert_eq!(app_status_from_lifecycle(&state), AppStatus::Running);
    }

    #[test]
    fn trace_spec_rejects_blank_trace() {
        let error = ApplicationTraceSpec
            .validate(&TraceContext::new(" "))
            .expect_err("blank trace id must be rejected");
        assert!(error.to_string().contains("trace"));
    }

    #[test]
    fn manifest_v1_spec_rejects_missing_ability() {
        let manifest = ApplicationManifestV1::new(
            macaca_proto::PackageId::new("application.invalid"),
            macaca_proto::DeveloperId::new("developer.invalid"),
            "Invalid",
            "1.0.0",
            macaca_proto::ApplicationRuntimeProfile::new(
                macaca_proto::PackageRuntimeKind::Yaml,
                "1",
            ),
            macaca_proto::ApplicationCompatibilityDeclaration::new("0.1.0"),
        );

        let report = ApplicationManifestV1Spec.validate(&manifest);
        assert!(!report.is_success());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "missing_ability"));
    }

    #[test]
    fn manifest_spec_rejects_empty_declared_service_identifier() {
        let manifest = AppManifest {
            id: macaca_proto::ApplicationId::new(),
            name: "service-invalid".into(),
            description: None,
            version: "1.0.0".into(),
            layer: AppLayer::L2Wasm,
            ui_type: None,
            agents: vec![],
            llm_config: None,
            entry_agent: None,
            entrypoint: None,
            workflows: None,
            resources: None,
            context: None,
            service_contract: Some(AppServiceContractConfig {
                use_packs: vec![],
                required_services: vec!["".into()],
                optional_services: vec![],
                service_policy_overrides: Default::default(),
            }),
            execution_profile: None,
            workbench: None,
            autonomy: None,
            ui: None,
            execution_control: None,
        };
        let error = ApplicationManifestSpec.validate(&manifest).unwrap_err();
        assert!(error.to_string().contains("required_services"));
    }
}
