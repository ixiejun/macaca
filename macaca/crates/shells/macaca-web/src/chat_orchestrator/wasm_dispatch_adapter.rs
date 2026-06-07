//! WASM host-dispatch routing adapter and DTO mapping.
//!
//! Detects WASM-first applications and maps chat prompts into neutral host
//! command payloads without application-specific branching.

use std::sync::Arc;

use macaca_proto::{
    ApplicationHostCommand, ApplicationHostDispatchServiceCommand, ApplicationId,
    ApplicationImport, ApplicationMetadataQueryCommand, ApplicationServiceScope,
    PackageRuntimeKind, TraceContext,
};

use crate::state::AppState;

/// Build a generic WASM host-dispatch command when the application is WASM-first.
///
/// This helper keeps routing app-agnostic:
/// - it relies only on sanitized metadata (`runtime_kind`, `abilities`, `agents`),
/// - it never checks app name or business identifiers,
/// - it maps chat input into neutral payload metadata for the guest export.
pub(crate) async fn wasm_chat_dispatch_command(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    trace: TraceContext,
    prompt: &str,
) -> Option<ApplicationHostDispatchServiceCommand> {
    let command = ApplicationMetadataQueryCommand::application(trace.clone(), *app_id).ok()?;
    let metadata = state.application_client.metadata(command).await.ok()?;
    let is_registry_wasm_runtime =
        crate::application_shell_adapter::is_registry_wasm_layer_app(state, app_id).await;
    let is_wasm_runtime = is_registry_wasm_runtime
        || metadata.application.runtime.runtime_kind == Some(PackageRuntimeKind::WasmComponent);
    if !is_wasm_runtime {
        return None;
    }
    let has_wasm_ability = metadata.abilities.iter().any(|ability| {
        ability.id == "ability.runtime.wasm" || ability.implementation.contains("wasm")
    });
    if !has_wasm_ability && !is_registry_wasm_runtime {
        return None;
    }

    let mut host_command = ApplicationHostCommand::with_trace(
        ApplicationImport::TraceEmit,
        wasm_chat_export_payload(prompt),
        trace.clone(),
    );
    host_command
        .metadata
        .insert("wasm.export".into(), "app:start".into());
    Some(ApplicationHostDispatchServiceCommand {
        trace,
        scope: ApplicationServiceScope::application(*app_id),
        host_command,
    })
}

/// Build the payload passed to a WASM application export from chat text.
///
/// This is intentionally schema-generic. If an app-owned coordinator supplies a
/// JSON object, the runtime preserves those typed fields for declarative
/// placeholders such as `${chat.symbol}`. Plain chat text remains plain
/// `input`; Macaca does not infer domain fields from prose.
pub(crate) fn wasm_chat_export_payload(prompt: &str) -> serde_json::Value {
    match serde_json::from_str::<serde_json::Value>(prompt) {
        Ok(serde_json::Value::Object(mut object)) => {
            object
                .entry("channel")
                .or_insert_with(|| serde_json::Value::String("chat".into()));
            serde_json::Value::Object(object)
        }
        _ => serde_json::json!({
            "input": prompt,
            "channel": "chat",
        }),
    }
}

/// Return whether the application declares app-scoped agents.
///
/// WASM applications with declared agents still execute through the WASM
/// runtime.  The agents are prepared as OS capabilities for task/delegation
/// imports, not as a replacement coordinator path.
pub(crate) async fn application_declares_agents(state: &Arc<AppState>, app_id: &ApplicationId) -> bool {
    let Some(command) = ApplicationMetadataQueryCommand::application(
        TraceContext::new("web-chat-agent-scope"),
        *app_id,
    )
    .ok() else {
        return false;
    };
    state
        .application_client
        .metadata(command)
        .await
        .map(|metadata| !metadata.application.agents.is_empty())
        .unwrap_or(false)
}

/// New-session preparation branch selected before chat execution starts.
///
/// The value is intentionally small and pure so Web tests can lock the routing
/// contract without constructing an entire `AppState`.  WASM applications are
/// always executed by the WASM host-dispatch path; declared agents only decide
/// whether OS orchestration services must be prepared for guest imports such as
/// `macaca:agent/delegate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NewSessionPreparation {
    WasmOrchestrationExecutor,
    FrameworkExecutor,
    WasmHostDispatchOnly,
}

pub(crate) fn new_session_preparation_for_chat(
    has_wasm_dispatch: bool,
    declares_agents: bool,
) -> NewSessionPreparation {
    match (has_wasm_dispatch, declares_agents) {
        (true, true) => NewSessionPreparation::WasmOrchestrationExecutor,
        (true, false) => NewSessionPreparation::WasmHostDispatchOnly,
        (false, _) => NewSessionPreparation::FrameworkExecutor,
    }
}
