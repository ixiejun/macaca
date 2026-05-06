//! **Anti-corruption** helpers for **opaque** external context payloads.
//!
//! Remote transports (MCP stdio, HTTP bridges, WASM shims) stay **outside** this module.  Here we
//! only enforce **structural** limits so hostile or buggy peers cannot allocate unbounded memory.

use crate::report::{ContextDecisionReport, ContextDecisionSeverity};

/// Transport-agnostic opaque blob handed to Macaca **before** it becomes a [`crate::composer::ContextCandidate`].
///
/// `transport_class` is a **hint** for logging (e.g. `"mcp"`, `"http"`) — it MUST NOT influence
/// security decisions; only `max_body_bytes` and canonical fields below matter.
#[derive(Debug, Clone)]
pub struct OpaqueExternalPayload {
    /// Free-form label for diagnostics (e.g. the adapter kind).
    pub transport_class: String,
    /// Monotonic version chosen by the adapter author — Macaca does **not** interpret payload
    /// schema beyond enforcing numeric sanity.
    pub schema_version: u32,
    /// Candidate `source_id` after adapter routing (must be non-empty when valid).
    pub source_id: String,
    /// UTF-8 body destined for downstream [`crate::estimate::estimate_text_tokens`].
    pub body: String,
}

/// Hard limits for [`validate_opaque_external_payload`].
#[derive(Debug, Clone, Copy)]
pub struct ExternalCandidateLimits {
    pub max_source_id_bytes: usize,
    pub max_body_bytes: usize,
}

impl Default for ExternalCandidateLimits {
    fn default() -> Self {
        Self {
            max_source_id_bytes: 2 * 1024,
            max_body_bytes: 1024 * 1024,
        }
    }
}

/// Validates only **sizes** and presence — never interprets MCP/HTTP framing.
pub fn validate_opaque_external_payload(
    payload: &OpaqueExternalPayload,
    limits: &ExternalCandidateLimits,
) -> Result<(), ContextDecisionReport> {
    if payload.source_id.trim().is_empty() {
        return Err(ContextDecisionReport {
            code: "external_context_missing_source_id".into(),
            severity: ContextDecisionSeverity::Error,
            message: "Opaque external payload must carry a non-empty source_id.".into(),
        });
    }
    if payload.source_id.len() > limits.max_source_id_bytes {
        return Err(ContextDecisionReport {
            code: "external_context_source_id_too_large".into(),
            severity: ContextDecisionSeverity::Error,
            message: format!(
                "source_id exceeds max_source_id_bytes={}",
                limits.max_source_id_bytes
            ),
        });
    }
    if payload.body.len() > limits.max_body_bytes {
        return Err(ContextDecisionReport {
            code: "external_context_body_too_large".into(),
            severity: ContextDecisionSeverity::Error,
            message: format!("body exceeds max_body_bytes={}", limits.max_body_bytes),
        });
    }
    Ok(())
}
