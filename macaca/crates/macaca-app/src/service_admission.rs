//! Admission specifications for the Route C Application Service.
//!
//! The module applies the Specification pattern to service inputs.  Keeping
//! trace, scope, manifest, and runtime-kind checks in small reusable specs
//! prevents Web, runtime-host, and `AppRuntime` from copying ad-hoc validation
//! branches as the Application Service grows.

use macaca_proto::{
    ApplicationLifecycleState, MacacaError, MacacaResult, PackageRuntimeKind, TraceContext,
};

use crate::model::{AppLayer, AppManifest, AppStatus};

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
        tracing::info!(
            app_id = %manifest.id,
            app_name = %manifest.name,
            layer = ?manifest.layer,
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
}
