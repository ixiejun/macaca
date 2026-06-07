//! Application service client adapter for chat orchestration.
//!
//! Resolves entry agents, session envelopes, and agent-scope metadata through
//! `application_client` with structured fallbacks for auditability.

use std::sync::Arc;

use macaca_proto::{
    ApplicationId, ApplicationMetadataQueryCommand, ApplicationServiceScope,
    ApplicationSessionStartCommand, ApplicationSessionStopCommand, ApplicationStatusCommand,
    TraceContext,
};

use crate::state::AppState;

pub(crate) async fn service_entry_agent_name(state: &Arc<AppState>, app_id: &ApplicationId) -> Option<String> {
    match ApplicationMetadataQueryCommand::application(
        TraceContext::new("web-chat-entry-agent-metadata"),
        *app_id,
    ) {
        Ok(command) => match state.application_client.metadata(command).await {
            Ok(view) => return view.entry.agent_name,
            Err(error) => tracing::warn!(
                app_id = %app_id,
                error = %error,
                "Application metadata query failed while resolving entry agent; using status fallback"
            ),
        },
        Err(error) => tracing::warn!(
            app_id = %app_id,
            error = %error,
            "Application metadata query rejected while resolving entry agent"
        ),
    }

    match state
        .application_client
        .status(ApplicationStatusCommand {
            trace: TraceContext::new("web-chat-entry-agent-status"),
            scope: ApplicationServiceScope::application(*app_id),
        })
        .await
    {
        Ok(views) => views
            .into_iter()
            .find(|view| view.id == *app_id)
            .and_then(|view| view.entry_agent),
        Err(error) => {
            tracing::warn!(
                app_id = %app_id,
                error = %error,
                "Application Service status failed while resolving entry agent"
            );
            None
        }
    }
}

pub(crate) async fn notify_application_session_start(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: &str,
    entry_agent_name: &str,
) {
    let scope = ApplicationServiceScope {
        application_id: Some(*app_id),
        application_name: None,
        session_id: Some(session_id.to_string()),
        agent_name: Some(entry_agent_name.to_string()),
    };
    let command = ApplicationSessionStartCommand {
        trace: TraceContext::new("web-chat-session-start"),
        scope,
    };
    match state.application_client.session_start(command).await {
        Ok(view) => tracing::info!(
            app_id = %view.application_id,
            session_id = %view.session_id,
            status = %view.status,
            "Application Service session envelope started"
        ),
        Err(error) => tracing::warn!(
            app_id = %app_id,
            session_id,
            error = %error,
            "Application Service session start failed; continuing coordinator path"
        ),
    }
}

pub(crate) async fn notify_application_session_stop(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: &str,
    entry_agent_name: &str,
    reason: &str,
) {
    let scope = ApplicationServiceScope {
        application_id: Some(*app_id),
        application_name: None,
        session_id: Some(session_id.to_string()),
        agent_name: Some(entry_agent_name.to_string()),
    };
    let command = ApplicationSessionStopCommand {
        trace: TraceContext::new("web-chat-session-stop"),
        scope,
        reason: Some(reason.to_string()),
    };
    if let Err(error) = state.application_client.session_stop(command).await {
        tracing::warn!(
            app_id = %app_id,
            session_id,
            error = %error,
            "Application Service session stop failed after coordinator completion"
        );
    }
}
