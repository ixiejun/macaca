//! UI bridge command handler — capability admission and host dispatch.
//!
//! The host does not interpret application business intent. It validates the
//! bridge envelope, checks manifest-declared capabilities, maps `service.call`
//! to the stable Application ABI import, and lets the runtime/service router
//! make final provider and policy decisions.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

use axum::extract::{Path as AxumPath, State};
use axum::http::StatusCode;
use axum::Json;
use macaca_proto::{
    ApplicationHostCommand, ApplicationHostCommandResult, ApplicationHostDispatchServiceCommand,
    ApplicationId, ApplicationImport, ApplicationServiceScope, ApplicationUiBridgeRequest,
    ApplicationUiBridgeResponse, TraceContext, WASM_HOST_IMPORT_CAPABILITY,
    WASM_HOST_IMPORT_OPERATION, WASM_HOST_IMPORT_SERVICE_ID,
};

use crate::app_ui_llm_bridge::hydrate_llm_bridge_payload;
use crate::app_ui_routes::adapter::parse_app_id;
use crate::app_ui_routes::bridge_adapter::{
    bridge_response, bridge_trace, rejected_bridge_result, rejected_undeclared_service_result,
};
use crate::app_ui_routes::context::app_ui_context;
use crate::app_ui_routes::types::RouteError;
use crate::app_ui_session_projection::AppUiSessionProjection;
use crate::app_ui_workspace_scope::decorate_workspace_scoped_payload;
use crate::routes::{err, proto_err};
use crate::state::AppState;

/// Dispatch one generic UI bridge command through Application Service.
pub async fn post_app_ui_bridge(
    State(state): State<Arc<AppState>>,
    AxumPath(app_id): AxumPath<String>,
    Json(request): Json<ApplicationUiBridgeRequest>,
) -> Result<Json<ApplicationUiBridgeResponse>, RouteError> {
    let app_id = parse_app_id(&app_id)?;
    let context = app_ui_context(&state, app_id).await?;
    let trace = bridge_trace(&request);
    tracing::info!(
        app_id = %app_id,
        trace_id = %trace.trace_id,
        command_id = %request.command_id,
        capability = %request.capability,
        command = "app.ui.bridge.receive",
        "application-owned UI bridge command received"
    );

    if !context.ui.declares_bridge_capability(&request.capability) {
        tracing::warn!(
            app_id = %app_id,
            trace_id = %trace.trace_id,
            command_id = %request.command_id,
            capability = %request.capability,
            command = "app.ui.bridge.receive",
            "application-owned UI bridge rejected undeclared capability"
        );
        return Ok(Json(bridge_response(
            &request,
            false,
            rejected_bridge_result("ui bridge capability was not declared", trace),
        )));
    }

    // App-owned UI bundles can use the generic service bridge without the host
    // chat composer. Once the bridge capability is admitted, persist a small
    // shell-visible session memento so refresh and the left session log can
    // recover the run. The projection uses only generic bridge routing fields
    // and never inspects application-owned payload data.
    AppUiSessionProjection::new(Arc::clone(&state.persist.session_store))
        .record_bridge_received(&app_id, &request)
        .await
        .map_err(|error| {
            err(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to project application UI bridge session: {error}"),
            )
        })?;

    let result = match request.capability.as_str() {
        "service.call" => {
            dispatch_service_call(
                &state,
                app_id,
                &request,
                trace,
                &context.declared_services,
                context.workspace_root.as_deref(),
            )
            .await?
        }
        _ => rejected_bridge_result(
            "ui bridge capability is not implemented by this host",
            trace,
        ),
    };
    Ok(Json(bridge_response(&request, true, result)))
}

/// Forward an admitted `service.call` bridge command to Application host dispatch.
async fn dispatch_service_call(
    state: &Arc<AppState>,
    app_id: ApplicationId,
    request: &ApplicationUiBridgeRequest,
    trace: TraceContext,
    declared_services: &BTreeSet<String>,
    workspace_root: Option<&Path>,
) -> Result<ApplicationHostCommandResult, RouteError> {
    let Some(service_id) = request
        .service_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    else {
        return Ok(rejected_bridge_result(
            "service.call requires service_id",
            trace,
        ));
    };
    let Some(operation) = request
        .operation
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    else {
        return Ok(rejected_bridge_result(
            "service.call requires operation",
            trace,
        ));
    };
    if let Some(result) =
        rejected_undeclared_service_result(service_id.trim(), declared_services, trace.clone())
    {
        return Ok(result);
    }

    let payload = hydrate_llm_bridge_payload(
        app_id,
        request.session_id.as_deref(),
        service_id.trim(),
        operation.trim(),
        &request.payload,
        trace.clone(),
    )
    .map_err(|error| err(StatusCode::INTERNAL_SERVER_ERROR, error))?;
    let payload = decorate_workspace_scoped_payload(
        service_id.trim(),
        operation.trim(),
        payload,
        workspace_root,
    );

    let mut host_command =
        ApplicationHostCommand::with_trace(ApplicationImport::ServiceCall, payload, trace.clone());
    host_command.metadata.insert(
        WASM_HOST_IMPORT_SERVICE_ID.into(),
        service_id.trim().to_string(),
    );
    host_command.metadata.insert(
        WASM_HOST_IMPORT_OPERATION.into(),
        operation.trim().to_string(),
    );
    host_command.metadata.insert(
        WASM_HOST_IMPORT_CAPABILITY.into(),
        request.capability.trim().to_string(),
    );
    if let Some(session_id) = request
        .session_id
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        host_command
            .metadata
            .insert("session_id".into(), session_id.to_string());
    }
    host_command
        .metadata
        .insert("ui.bridge.command_id".into(), request.command_id.clone());
    if let Some(surface_id) = request
        .surface_id
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        host_command
            .metadata
            .insert("ui.surface_id".into(), surface_id.to_string());
    }

    let command = ApplicationHostDispatchServiceCommand {
        trace: trace.clone(),
        scope: ApplicationServiceScope::application(app_id),
        host_command,
    };
    let result = state
        .application_client
        .host_dispatch(command)
        .await
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;
    tracing::info!(
        app_id = %app_id,
        trace_id = %trace.trace_id,
        command_id = %request.command_id,
        service_id,
        operation,
        status = ?result.status,
        command = "app.ui.bridge.service.call",
        "application-owned UI bridge service.call dispatched"
    );
    Ok(result)
}
