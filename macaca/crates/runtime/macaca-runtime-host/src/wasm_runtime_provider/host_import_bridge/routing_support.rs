//! Pure routing helpers for WASM host import adaptation.
//!
//! Functions in this module implement the Adapter pattern at the ABI boundary:
//! guests supply compact orchestration payloads while trusted host metadata and
//! trace context are hydrated into typed service commands. Sanitization helpers
//! enforce bounded, redacted outputs for audit replay and telemetry consumers.

use std::collections::{BTreeMap, BTreeSet};

use macaca_proto::{
    audit_redaction, ApplicationImport, KernelServiceId, ServiceCommandName, TraceContext,
    WasmHostImportCommand, APPLICATION_AGENT_DELEGATE_COMMAND, APPLICATION_SERVICE_ID,
    WASM_HOST_IMPORT_CAPABILITY, WASM_HOST_IMPORT_OPERATION, WASM_HOST_IMPORT_SERVICE_ID,
};
use serde_json::{Map, Value};

use super::constants::{TASK_CREATE_GOAL_OPERATION, TASK_QUERY_OPERATION, TASK_SERVICE_ID};

/// Hydrate host-owned scope fields on a declarative UI intent.
///
/// Guests often cannot know the concrete application id or session id before
/// the host creates an execution session. Web shells attach the user-visible
/// session id to the trace, while lower-level component sessions also attach an
/// internal runtime id to metadata. The bridge prefers the trace session for UI
/// surfaces so frontend queries and `ui.render` storage use the same key, then
/// falls back to metadata for non-web hosts. This keeps WASM artifacts
/// portable and prevents application code from hardcoding runtime identities.
pub(super) fn hydrate_ui_render_scope(
    intent: &mut macaca_proto::UiIntent,
    command: &WasmHostImportCommand,
    trace: &TraceContext,
) {
    if intent.app_id.trim().is_empty() || intent.app_id == "${app.id}" {
        if let Some(app_id) = command.metadata.get("app.id") {
            intent.app_id = app_id.clone();
        }
    }
    if intent.session_id.trim().is_empty() || intent.session_id == "${session.id}" {
        if let Some(session_id) = trace.session_id.as_ref() {
            intent.session_id = session_id.clone();
        } else if let Some(session_id) = command.metadata.get("session.id") {
            intent.session_id = session_id.clone();
        }
    }
    if intent.trace.is_none() {
        intent.trace = Some(trace.clone());
    }
}

/// Return whether an Application ABI import is admitted by this bridge portal.
pub(super) fn is_supported_portal_import(import: &ApplicationImport) -> bool {
    matches!(
        import,
        ApplicationImport::ServiceCall
            | ApplicationImport::TraceEmit
            | ApplicationImport::UiRender
            | ApplicationImport::TaskCreateGoal
            | ApplicationImport::TaskQuery
            | ApplicationImport::AgentDelegate
    )
}

/// Return whether an import name participates in orchestration scope checks.
pub(super) fn is_orchestration_import_name(import_name: &str) -> bool {
    import_name == ApplicationImport::TaskCreateGoal.as_name()
        || import_name == ApplicationImport::TaskQuery.as_name()
        || import_name == ApplicationImport::AgentDelegate.as_name()
}

/// Require non-empty `app.id` and `session.id` metadata for orchestration imports.
pub(super) fn has_non_empty_scope(metadata: &BTreeMap<String, String>) -> bool {
    metadata
        .get("app.id")
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
        && metadata
            .get("session.id")
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
}

/// Resolve the default target service for imports that omit explicit routing metadata.
pub(super) fn default_service_for_import(import: &ApplicationImport) -> Option<&'static str> {
    match import {
        ApplicationImport::TaskCreateGoal | ApplicationImport::TaskQuery => Some(TASK_SERVICE_ID),
        ApplicationImport::AgentDelegate => Some(APPLICATION_SERVICE_ID),
        _ => None,
    }
}

/// Resolve the default service operation for imports that omit explicit routing metadata.
pub(super) fn default_operation_for_import(import: &ApplicationImport) -> Option<&'static str> {
    match import {
        ApplicationImport::TaskCreateGoal => Some(TASK_CREATE_GOAL_OPERATION),
        ApplicationImport::TaskQuery => Some(TASK_QUERY_OPERATION),
        ApplicationImport::AgentDelegate => Some(APPLICATION_AGENT_DELEGATE_COMMAND),
        _ => None,
    }
}

/// Convert compact guest orchestration payloads into typed service commands.
///
/// Declarative WASM metadata should not need to know host-owned app/session
/// identity or trace shapes. The bridge acts as an Adapter at the ABI boundary:
/// guests declare provider-neutral work intent, while trusted host metadata
/// supplies application/session identity, trace, and command envelopes required
/// by Macaca services. This keeps demo and third-party WASM applications honest:
/// they can use `macaca:task/*` and `macaca:agent/delegate` without bypassing the
/// OS task and agent execution services or hardcoding runtime internals.
pub(super) fn hydrate_orchestration_service_payload(
    command: &WasmHostImportCommand,
    trace: &TraceContext,
) -> Value {
    let app_id = command.metadata.get("app.id").cloned().unwrap_or_default();
    let session_id = command
        .metadata
        .get("session.id")
        .cloned()
        .or_else(|| trace.session_id.clone())
        .unwrap_or_default();
    let agent_name = command
        .metadata
        .get("agent.name")
        .cloned()
        .unwrap_or_else(|| "wasm-guest".into());
    if command.import_name == ApplicationImport::TaskCreateGoal.as_name() {
        if command.payload.get("app_id").is_some() && command.payload.get("description").is_some() {
            return command.payload.clone();
        }
        return serde_json::json!({
            "app_id": app_id,
            "session_id": session_id,
            "description": command.payload.get("description")
                .or_else(|| command.payload.get("goal"))
                .or_else(|| command.payload.get("task"))
                .cloned()
                .unwrap_or(Value::Null),
            "trace": trace,
        });
    }
    if command.import_name == ApplicationImport::TaskQuery.as_name() {
        if command.payload.get("app_id").is_some() && command.payload.get("session_id").is_some() {
            return command.payload.clone();
        }
        return serde_json::json!({
            "app_id": app_id,
            "session_id": session_id,
            "trace": trace,
        });
    }
    if command.import_name != ApplicationImport::AgentDelegate.as_name() {
        return command.payload.clone();
    }
    if command.payload.get("trace").is_some() && command.payload.get("scope").is_some() {
        return command.payload.clone();
    }
    serde_json::json!({
        "trace": trace,
        "scope": {
            "application_id": app_id,
            "application_name": null,
            "session_id": session_id,
            "agent_name": agent_name
        },
        "target_agent": command.payload.get("target_agent").cloned().unwrap_or(Value::Null),
        "prompt": command.payload.get("prompt").cloned().unwrap_or(Value::Null),
        "context": command.payload.get("context").cloned().unwrap_or_else(|| serde_json::json!({})),
        "metadata": command.metadata
    })
}

/// Build a Task Service lifecycle payload envelope for one assignment id.
pub(super) fn task_lifecycle_payload(
    app_id: &str,
    session_id: &str,
    agent_name: &str,
    task_id: &str,
    trace: &TraceContext,
) -> Value {
    serde_json::json!({
        "app_id": app_id,
        "session_id": session_id,
        "agent_name": agent_name,
        "task_id": task_id,
        "trace": trace,
    })
}

/// Extract a task id from a Task Service assignment response object.
pub(super) fn extract_task_id_from_assignment(output: Value) -> Option<String> {
    output
        .get("task")
        .and_then(|task| task.get("id"))
        .and_then(extract_task_id_value)
}

/// Normalize task id representations that may be plain strings or tagged values.
pub(super) fn extract_task_id_value(value: &Value) -> Option<String> {
    if let Some(text) = value.as_str() {
        return Some(text.to_string());
    }
    value
        .as_object()
        .and_then(|object| object.get("value").or_else(|| object.get("0")))
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// Parse comma-separated service id lists from trusted host metadata.
pub(super) fn parse_csv_services(value: Option<&str>) -> BTreeSet<String> {
    value
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(str::to_string)
        .collect()
}

/// Bound serialized JSON output to the configured bridge maximum byte size.
pub(super) fn bound_json(value: Value, max_bytes: u64) -> Value {
    let size = serde_json::to_vec(&value)
        .map(|payload| payload.len() as u64)
        .unwrap_or(u64::MAX);
    if size <= max_bytes {
        value
    } else {
        serde_json::json!({
            "truncated": true,
            "reason": "host_import_output_too_large",
            "bytes": size
        })
    }
}

/// Normalize labels for safe inclusion in guest-visible metadata maps.
pub(super) fn sanitize_label(value: impl AsRef<str>) -> String {
    value
        .as_ref()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':' | '.') {
                ch
            } else if ch == '/' {
                ':'
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

/// Build the normalized guest command shell from one Application ABI request.
pub(super) fn command_shell(
    command: macaca_proto::ApplicationHostCommand,
    trace: TraceContext,
    category: macaca_proto::WasmHostImportCategory,
    import_name: String,
    payload_bytes: u64,
) -> WasmHostImportCommand {
    let default_target_service = default_service_for_import(&command.import);
    let default_operation = default_operation_for_import(&command.import);
    let target_service = command
        .metadata
        .get(WASM_HOST_IMPORT_SERVICE_ID)
        .filter(|value| !value.trim().is_empty())
        .map(KernelServiceId::new)
        .or_else(|| default_target_service.map(KernelServiceId::new));
    let operation = command
        .metadata
        .get(WASM_HOST_IMPORT_OPERATION)
        .filter(|value| !value.trim().is_empty())
        .map(ServiceCommandName::new)
        .or_else(|| default_operation.map(ServiceCommandName::new));
    let capability = command
        .metadata
        .get(WASM_HOST_IMPORT_CAPABILITY)
        .filter(|value| !value.trim().is_empty())
        .cloned();
    WasmHostImportCommand {
        category,
        import_name,
        target_service,
        operation,
        capability,
        payload: command.payload,
        payload_bytes,
        trace,
        metadata: command.metadata,
    }
}

/// Re-export canonical audit redaction helpers for sibling bridge modules.
pub(super) use audit_redaction::{is_safe_metadata_key, sanitize_json};
