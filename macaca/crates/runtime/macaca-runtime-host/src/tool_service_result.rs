//! Result normalization for `service.tool` invocation.
//!
//! The normalizer is a Strategy boundary: this slice keeps a small in-memory
//! artifact strategy, while later runtime-environment and artifact-service
//! proposals can swap in durable providers.  The public result shape stays the
//! same and logs never duplicate raw output.

use macaca_proto::{
    CapabilityToolInvocationResult, ToolArtifactRef, ToolAuditRef, ToolCommandResult,
    ToolInvocationRef, ToolResultClass, TraceContext,
};
use serde_json::{json, Value};

/// Bounded output returned by a tool invocation after budget enforcement.
pub struct NormalizedToolResult {
    pub command_result: ToolCommandResult,
    pub artifact_payload: Option<(ToolArtifactRef, Value)>,
}

/// Normalize an owning-service result into the provider-neutral `service.tool`
/// result contract.
pub fn normalize_invocation_result(
    invocation_ref: ToolInvocationRef,
    result: CapabilityToolInvocationResult,
    inline_budget_bytes: usize,
) -> NormalizedToolResult {
    let trace = result.trace.clone();
    if result.status != "ok" {
        return NormalizedToolResult {
            command_result: command_result(
                trace,
                result.status,
                ToolResultClass::Failure,
                None,
                Vec::new(),
                Some(invocation_ref),
                Some(bounded_summary(result.error_summary.unwrap_or_else(|| {
                    "owning service returned a failed tool result".into()
                }))),
            ),
            artifact_payload: None,
        };
    }

    let output = result.output.unwrap_or(Value::Null);
    let encoded_len = serde_json::to_vec(&output)
        .map(|bytes| bytes.len())
        .unwrap_or(0);
    if encoded_len > inline_budget_bytes {
        let artifact_ref = ToolArtifactRef(format!("tool-artifact-{}", invocation_ref.0));
        let inline = json!({
            "artifact_ref": artifact_ref.0,
            "summary": "tool output exceeded inline result budget",
            "bytes": encoded_len,
        });
        return NormalizedToolResult {
            command_result: command_result(
                trace,
                "ok",
                ToolResultClass::BinaryArtifact,
                Some(inline),
                vec![artifact_ref.clone()],
                Some(invocation_ref),
                None,
            ),
            artifact_payload: Some((artifact_ref, output)),
        };
    }

    NormalizedToolResult {
        command_result: command_result(
            trace,
            "ok",
            ToolResultClass::StructuredJson,
            Some(output),
            Vec::new(),
            Some(invocation_ref),
            None,
        ),
        artifact_payload: None,
    }
}

pub fn command_result(
    trace: TraceContext,
    status: impl Into<String>,
    result_class: ToolResultClass,
    inline_output: Option<Value>,
    artifact_refs: Vec<ToolArtifactRef>,
    invocation_ref: Option<ToolInvocationRef>,
    error_summary: Option<String>,
) -> ToolCommandResult {
    ToolCommandResult {
        trace,
        status: status.into(),
        result_class,
        inline_output,
        artifact_refs,
        invocation_ref,
        audit_refs: vec![ToolAuditRef::new("tool.invoke.local")],
        error_summary,
        captured_at: chrono::Utc::now(),
        metadata: Default::default(),
    }
}

pub fn bounded_summary(mut value: String) -> String {
    value = value.replace('\n', " ").replace('\r', " ");
    value.truncate(240);
    value
}
