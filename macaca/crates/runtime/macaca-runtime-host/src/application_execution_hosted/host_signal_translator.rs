//! Observer bridge translating ABI host-command rows into durable execution signals.
//!
//! Component-model WASM artifacts may declare a sequence of host commands routed through
//! ServiceRuntime or Application Service orchestration. This module records bounded,
//! protocol-shaped facts without persisting raw host output or application-specific fields.

use macaca_proto::ApplicationExecutionEventType;

use super::types::HostedApplicationExecutionSignal;

/// Translate ABI host-command results into durable execution-protocol signals.
///
/// Under the unified call-path model every host command row is equally authoritative
/// for terminal-state determination: any failure fails the run, and completion requires
/// every non-queued row to report a terminal success.
pub(crate) fn hosted_signals_from_host_result(
    result: &macaca_proto::ApplicationHostCommandResult,
) -> Vec<HostedApplicationExecutionSignal> {
    let Some(results) = result
        .output
        .get("host_command_results")
        .and_then(serde_json::Value::as_array)
    else {
        return Vec::new();
    };
    let mut signals = Vec::new();
    let mut completed = 0usize;
    let mut failed = 0usize;
    let mut queued = 0usize;
    for (index, row) in results.iter().enumerate() {
        let status = host_command_status_label(row);
        let metadata = row
            .get("metadata")
            .and_then(serde_json::Value::as_object)
            .cloned()
            .unwrap_or_default();
        let service_id = metadata
            .get("service_id")
            .or_else(|| metadata.get("service.id"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown");
        let reason_code = metadata
            .get("reason_code")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown");
        let task_id = row
            .pointer("/output/output/task_id")
            .and_then(serde_json::Value::as_str)
            .or_else(|| {
                row.pointer("/output/task_id")
                    .and_then(serde_json::Value::as_str)
            });
        let delegated_status = row
            .pointer("/output/output/status")
            .and_then(serde_json::Value::as_str)
            .or_else(|| {
                row.pointer("/output/status")
                    .and_then(serde_json::Value::as_str)
            });

        let effective_status = delegated_status.unwrap_or(status);
        let normalized_status = effective_status.to_ascii_lowercase();
        match normalized_status.as_str() {
            // Host imports may use either `queued` or `pending` for async continuation.
            "queued" | "pending" => {
                queued += 1;
            }
            "completed" | "ok" => {
                completed += 1;
            }
            _ => {
                failed += 1;
            }
        }
        signals.push(HostedApplicationExecutionSignal::new(
            ApplicationExecutionEventType::ToolCallCompleted,
            "hosted application declared host command completed",
            Some(serde_json::json!({
                "index": index,
                "status": effective_status,
                "service_id": service_id,
                "reason_code": reason_code,
                "task_id": task_id,
            })),
            format!("host-command-{index}-completed"),
        ));
    }
    if !results.is_empty() {
        let terminal_count = completed + failed + queued;
        let terminal = if failed > 0 {
            Some((
                ApplicationExecutionEventType::ExecutionFailed,
                "hosted application host commands failed",
                "host-commands-failed",
            ))
        } else if terminal_count > 0 && queued == 0 && completed == terminal_count {
            Some((
                ApplicationExecutionEventType::ExecutionCompleted,
                "hosted application host commands completed",
                "host-commands-completed",
            ))
        } else {
            None
        };
        if let Some((event_type, summary, suffix)) = terminal {
            signals.push(HostedApplicationExecutionSignal::new(
                event_type,
                summary,
                Some(serde_json::json!({
                    "host_command_count": results.len(),
                    "completed": completed,
                    "failed": failed,
                    "queued": queued,
                })),
                suffix,
            ));
        }
    }
    signals
}

fn host_command_status_label(row: &serde_json::Value) -> &str {
    row.get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown")
}
