//! Session lifecycle command handlers for Application Service.
//!
//! **Pattern:** Memento — session views are replayable provider-neutral records
//! stored in the host-owned session map and returned through typed commands.

use macaca_proto::{ApplicationSessionResumeCommand, ApplicationSessionStartCommand, ApplicationSessionStopCommand, ServiceCommand, ServiceResult};

use super::support::{decode, session_view, to_value};
use super::ApplicationSystemServiceProvider;

impl ApplicationSystemServiceProvider {
    pub(super) async fn handle_session_start(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<macaca_proto::ServiceCallResult> {
        let typed: ApplicationSessionStartCommand = decode(command.payload)?;
        let view = session_view(&typed.scope, "running")?;
        self.sync_wasm_service_policy_for_app(&view.application_id)
            .await?;
        self.sessions
            .write()
            .await
            .insert(view.session_id.clone(), view.clone());
        tracing::info!(
            trace_id = %typed.trace.trace_id,
            app_id = %view.application_id,
            session_id = %view.session_id,
            "application service session started"
        );
        Ok(Self::service_result(to_value(view)?, typed.trace))
    }

    pub(super) async fn handle_session_resume(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<macaca_proto::ServiceCallResult> {
        let typed: ApplicationSessionResumeCommand = decode(command.payload)?;
        let view = session_view(&typed.scope, "resumed")?;
        self.sessions
            .write()
            .await
            .insert(view.session_id.clone(), view.clone());
        Ok(Self::service_result(to_value(view)?, typed.trace))
    }

    pub(super) async fn handle_session_stop(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<macaca_proto::ServiceCallResult> {
        let typed: ApplicationSessionStopCommand = decode(command.payload)?;
        let view = session_view(&typed.scope, "stopped")?;
        self.sessions.write().await.remove(&view.session_id);
        Ok(Self::service_result(to_value(view)?, typed.trace))
    }
}
