//! Shared helpers for Skill CLI command assembly.
//!
//! Pure functions only: no I/O, no service calls.  Keeps handler modules thin
//! and testable.

use macaca_proto::{MacacaError, MacacaResult};

use super::live_client::LiveSkillOperatorPayload;
use super::types::SkillCliEvidenceRefs;

/// Convert an optional trimmed string into a bounded single-element vector.
pub(crate) fn optional_vec(value: Option<String>) -> Vec<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .into_iter()
        .collect()
}

/// Normalize and validate an HTTP API base URL for live Skill operations.
pub(crate) fn normalize_api_base(value: &str) -> MacacaResult<String> {
    let trimmed = value.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(MacacaError::Config(
            "skill live operations require a non-empty API base".into(),
        ));
    }
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return Err(MacacaError::Config(
            "skill live operations API base must start with http:// or https://".into(),
        ));
    }
    Ok(trimmed.to_string())
}

/// Percent-encode path segments for safe inclusion in REST URLs.
pub(crate) fn url_segment(value: &str) -> String {
    value.replace('%', "%25").replace('/', "%2F")
}

/// Map `reqwest` transport failures into structured CLI config errors.
pub(crate) fn http_error(error: reqwest::Error) -> MacacaError {
    MacacaError::Config(format!("skill live operations API request failed: {error}"))
}

/// Build the bounded operator payload shared by live HTTP mutation commands.
pub(crate) fn live_operator_payload(refs: SkillCliEvidenceRefs) -> LiveSkillOperatorPayload {
    let reason = refs
        .reason
        .unwrap_or_else(|| "cli_skill_operation".into())
        .trim()
        .to_string();
    let reason = if reason.is_empty() {
        "cli_skill_operation".into()
    } else {
        reason
    };
    LiveSkillOperatorPayload {
        rationale: reason.clone(),
        reason,
        evidence_ids: optional_vec(refs.evidence_ref),
        policy_decision_refs: optional_vec(refs.policy_ref),
        approval_refs: optional_vec(refs.approval_ref),
        rollback_ref: None,
        stale_after_days: None,
        narrow_use_threshold: None,
        source: "cli-skill-operations".into(),
        source_scope: "operator".into(),
    }
}
