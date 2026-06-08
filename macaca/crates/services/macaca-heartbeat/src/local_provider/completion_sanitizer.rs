//! Completion metadata sanitizer for Heartbeat run terminalization.
//!
//! Runtime Host reports delegated-dispatch outcomes through
//! `HeartbeatCompleteRunCommand`. This module applies a **Specification**
//! over inbound metadata keys/values so audit snapshots never retain prompts,
//! secrets, raw provider payloads, or other application-specific blobs.
//!
//! The allowlist is platform-neutral: every application shares the same audit
//! surface regardless of persona, workflow, or driver configuration.

use std::collections::BTreeMap;

use macaca_proto::{MacacaError, MacacaResult};

/// Normalizes a provider-neutral reason/label string for audit storage.
///
/// Empty or whitespace-only labels are rejected because terminal Heartbeat
/// transitions must always carry a replayable, non-empty reason code.
pub(super) fn normalize_label(value: String, message: &'static str) -> MacacaResult<String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(MacacaError::Config(message.into()));
    }
    Ok(value)
}

/// Filters completion metadata to a bounded, prefix-allowlisted audit view.
///
/// Runs in O(n) over inbound entries with early exit once the cap is reached.
/// Rejected keys are dropped silently so hostile or accidental oversized payloads
/// cannot expand memento history or leak sensitive fragments into snapshots.
pub(super) fn sanitized_completion_metadata(
    metadata: BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    const MAX_KEY_LEN: usize = 96;
    const MAX_VALUE_LEN: usize = 256;
    const MAX_ENTRIES: usize = 24;
    const ALLOWED_PREFIXES: &[&str] = &[
        "agent_execution.",
        "execution_envelope.",
        "dispatch.",
        "evidence.",
        "heartbeat.",
    ];
    const REJECTED_FRAGMENTS: &[&str] = &[
        "prompt",
        "raw",
        "secret",
        "credential",
        "private",
        "manifest",
        "package_bytes",
        "provider_output",
    ];

    let mut sanitized = BTreeMap::new();
    for (key, value) in metadata {
        if sanitized.len() >= MAX_ENTRIES {
            break;
        }
        let key = key.trim();
        let value = value.trim();
        if key.is_empty()
            || value.is_empty()
            || key.len() > MAX_KEY_LEN
            || !ALLOWED_PREFIXES
                .iter()
                .any(|prefix| key.starts_with(prefix))
            || REJECTED_FRAGMENTS
                .iter()
                .any(|fragment| key.to_ascii_lowercase().contains(fragment))
        {
            continue;
        }
        sanitized.insert(
            key.to_string(),
            value.chars().take(MAX_VALUE_LEN).collect::<String>(),
        );
    }
    sanitized
}
