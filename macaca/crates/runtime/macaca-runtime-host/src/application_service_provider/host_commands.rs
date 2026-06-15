//! Host dispatch, orchestration delegate, and GenUI query command handlers.
//!
//! **Pattern:** Bridge + Strategy — host dispatch lazily resolves WASM sessions,
//! agent delegation routes through an injected `ApplicationOrchestrationBackend`,
//! and GenUI queries read from the host-owned surface repository.

use std::collections::BTreeMap;

use macaca_app::{ApplicationHost, UnavailableApplicationHostBackend};
use macaca_proto::{
    ApplicationAgentDelegateCommand, ApplicationAgentDelegateResult,
    ApplicationGenUiSurfaceCommand, ApplicationHostDispatchResult,
    ApplicationHostDispatchServiceCommand, ServiceCommand, ServiceError, ServiceResult,
};

use super::support::{decode, service_adapter_error, to_value};
use super::ApplicationSystemServiceProvider;

impl ApplicationSystemServiceProvider {
    pub(super) async fn handle_host_dispatch(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<macaca_proto::ServiceCallResult> {
        let typed: ApplicationHostDispatchServiceCommand = decode(command.payload)?;
        let app_id = typed.scope.application_id.ok_or_else(|| {
            ServiceError::AdapterFailure("application.host.dispatch requires application_id".into())
        })?;
        let result: ApplicationHostDispatchResult = if let Some(session) = self
            .ensure_wasm_session(app_id, typed.trace.clone())
            .await?
        {
            session
                .dispatch(typed.host_command)
                .await
                .map_err(service_adapter_error)?
        } else {
            let host = ApplicationHost::new(UnavailableApplicationHostBackend);
            host.dispatch(typed.host_command)
                .await
                .map_err(service_adapter_error)?
        };
        Ok(Self::service_result(to_value(result)?, typed.trace))
    }

    pub(super) async fn handle_agent_delegate(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<macaca_proto::ServiceCallResult> {
        let typed: ApplicationAgentDelegateCommand = decode(command.payload)?;
        let result_trace = typed.trace.clone();
        let app_id = typed.scope.application_id.ok_or_else(|| {
            ServiceError::AdapterFailure(
                "application.agent.delegate requires application_id".into(),
            )
        })?;
        let session_id = typed
            .scope
            .session_id
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                ServiceError::AdapterFailure(
                    "application.agent.delegate requires session_id".into(),
                )
            })?;
        if typed.target_agent.trim().is_empty() {
            return Err(ServiceError::AdapterFailure(
                "application.agent.delegate requires target_agent".into(),
            ));
        }
        if typed.prompt.trim().is_empty() {
            return Err(ServiceError::AdapterFailure(
                "application.agent.delegate requires prompt".into(),
            ));
        }
        tracing::info!(
            trace_id = %typed.trace.trace_id,
            app_id = %app_id,
            session_id,
            target_agent = %typed.target_agent,
            "application service accepted app-scoped agent delegation"
        );
        let result = if let Some(backend) = &self.orchestration_backend {
            backend.delegate_agent(typed).await?
        } else {
            ApplicationAgentDelegateResult {
                application_id: app_id,
                session_id: session_id.to_string(),
                target_agent: typed.target_agent,
                task_id: None,
                success: false,
                output: serde_json::json!({
                    "reason": "application orchestration backend is not configured"
                }),
                status: "unavailable".into(),
                metadata: BTreeMap::from([(
                    "reason_code".into(),
                    "orchestration_backend_unavailable".into(),
                )]),
            }
        };
        Ok(Self::service_result(to_value(result)?, result_trace))
    }

    pub(super) async fn handle_genui_surface(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<macaca_proto::ServiceCallResult> {
        let typed: ApplicationGenUiSurfaceCommand = decode(command.payload)?;
        let surface = self.get_genui_surface(&typed).await?;
        Ok(Self::service_result(to_value(surface)?, typed.trace))
    }
}
