//! Canonical audit/trace JSON redaction for the Macaca OS platform.
//!
//! Every audit replay, telemetry snapshot, WASM host export, and application
//! execution event store MUST route sensitive payloads through this module so
//! redaction rules stay provider-neutral and application-agnostic.
//!
//! Design pattern: **Strategy** — key-classification predicates compose into
//! export vs inline-redaction policies without hardcoding application names.

use serde_json::{Map, Value};

/// Placeholder written when a scalar value is redacted for observability consumers.
pub const REDACTED_JSON_VALUE: &str = "[redacted]";

/// Substring markers that classify a lowercase JSON object key as unsafe for export.
///
/// Export paths **omit** matching keys entirely so replay consumers never see
/// even redacted placeholders for high-risk payload families (raw prompts, secrets).
const EXPORT_OMIT_KEY_MARKERS: &[&str] = &[
    "raw", "payload", "memory", "secret", "api_key", "apikey", "prompt", "env",
];

/// Substring markers that classify a lowercase JSON object key as sensitive for
/// inline observability storage (value replaced with [`REDACTED_JSON_VALUE`]).
const INLINE_SENSITIVE_KEY_MARKERS: &[&str] = &[
    "secret",
    "token",
    "password",
    "credential",
    "signature",
    "private_key",
    "prompt",
];

/// Substring markers that classify free-form string contents as unsafe for export.
const EXPORT_OMIT_STRING_MARKERS: &[&str] = &["secret", "prompt", "api_key"];

/// Markers replaced inside strings when harness-style partial redaction is required.
const STRING_PARTIAL_REDACT_MARKERS: &[&str] = &[
    "secret",
    "api_key",
    "apikey",
    "private key",
    "prompt",
    "env",
];

/// Return whether a JSON object key must be omitted from export/replay snapshots.
///
/// Runs in O(marker * key_len) with a tiny fixed marker table — acceptable for
/// audit-sized JSON blobs and keeps the rule set centralized for CI gates.
pub fn should_omit_json_key_from_export(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    EXPORT_OMIT_KEY_MARKERS
        .iter()
        .any(|marker| lower.contains(marker))
}

/// Return whether a JSON object key should be inline-redacted in observability stores.
///
/// Unlike [`should_omit_json_key_from_export`], matching keys remain present with a
/// redacted scalar so operators can see *that* a field existed without seeing content.
pub fn is_sensitive_json_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    if lower.ends_with("_prompt") {
        return true;
    }
    INLINE_SENSITIVE_KEY_MARKERS
        .iter()
        .any(|marker| lower.contains(marker))
}

/// Return whether a metadata map key is safe to forward to guest-visible surfaces.
///
/// Guest WASM imports and external replay consumers use this guard before copying
/// host metadata into bounded export maps.
pub fn is_safe_metadata_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    !(lower.contains("token")
        || lower.contains("secret")
        || lower.contains("password")
        || lower.contains("credential"))
}

/// Substring markers for supply-chain metadata that must never appear in audit exports.
const METADATA_MAP_OMIT_MARKERS: &[&str] = &[
    "manifest",
    "wasm",
    "package_bytes",
    "signature",
    "private_key",
];

/// Return whether a flat string metadata map key must be dropped before audit export.
///
/// Combines [`is_safe_metadata_key`], [`should_omit_json_key_from_export`], and
/// provider-neutral supply-chain markers so execution-control and replay surfaces
/// share one Strategy instead of duplicating local `is_sensitive_key` helpers.
pub fn should_omit_metadata_map_key(key: &str) -> bool {
    if !is_safe_metadata_key(key) {
        tracing::debug!(
            target = "macaca_proto::audit_redaction",
            event = "omit_metadata_key",
            reason_code = "unsafe_metadata_key"
        );
        return true;
    }
    if should_omit_json_key_from_export(key) {
        tracing::debug!(
            target = "macaca_proto::audit_redaction",
            event = "omit_metadata_key",
            reason_code = "export_key_marker"
        );
        return true;
    }
    let lower = key.to_ascii_lowercase();
    if METADATA_MAP_OMIT_MARKERS
        .iter()
        .any(|marker| lower.contains(marker))
    {
        tracing::debug!(
            target = "macaca_proto::audit_redaction",
            event = "omit_metadata_key",
            reason_code = "supply_chain_marker"
        );
        return true;
    }
    false
}

/// Recursively sanitize JSON for audit export and host-import replay (immutable).
///
/// Operationally:
/// 1. Object keys matching [`should_omit_json_key_from_export`] are dropped.
/// 2. Nested structures are sanitized depth-first.
/// 3. String scalars containing export markers are replaced with [`REDACTED_JSON_VALUE`].
/// 4. Numbers/bools/null pass through unchanged.
///
/// Tracing: callers should log `event=audit_redaction_applied` at boundaries; this
/// pure function stays side-effect free for deterministic unit tests.
pub fn sanitize_json(value: Value) -> Value {
    match value {
        Value::Object(object) => {
            let mut sanitized = Map::new();
            for (key, child) in object {
                if should_omit_json_key_from_export(&key) {
                    tracing::debug!(
                        target = "macaca_proto::audit_redaction",
                        event = "omit_json_key",
                        reason_code = "export_key_marker"
                    );
                    continue;
                }
                sanitized.insert(key, sanitize_json(child));
            }
            Value::Object(sanitized)
        }
        Value::Array(items) => Value::Array(items.into_iter().map(sanitize_json).collect()),
        Value::String(text) => Value::String(sanitize_string_for_export(text)),
        other => other,
    }
}

/// Harness-oriented sanitizer: omit export keys but apply partial string redaction.
///
/// WASM guest harness fixtures use this variant so diagnostic strings remain
/// structurally similar while sensitive substrings are masked.
pub fn sanitize_json_for_harness(value: Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .filter_map(|(key, child)| {
                    if should_omit_json_key_from_export(&key) {
                        tracing::debug!(
                            target = "macaca_proto::audit_redaction",
                            event = "omit_json_key",
                            reason_code = "harness_key_marker"
                        );
                        return None;
                    }
                    Some((key, sanitize_json_for_harness(child)))
                })
                .collect::<Map<_, _>>(),
        ),
        Value::Array(values) => {
            Value::Array(values.into_iter().map(sanitize_json_for_harness).collect())
        }
        Value::String(value) => Value::String(sanitize_string_for_audit(value)),
        other => other,
    }
}

/// Redact export-unsafe substrings from a single string scalar.
pub fn sanitize_string_for_export(value: String) -> String {
    let lower = value.to_ascii_lowercase();
    if EXPORT_OMIT_STRING_MARKERS
        .iter()
        .any(|marker| lower.contains(marker))
    {
        REDACTED_JSON_VALUE.into()
    } else {
        value
    }
}

/// Apply partial in-string marker replacement for harness and diagnostic surfaces.
pub fn sanitize_string_for_audit(value: String) -> String {
    let mut sanitized = value;
    for marker in STRING_PARTIAL_REDACT_MARKERS {
        sanitized = sanitized.replace(marker, REDACTED_JSON_VALUE);
        sanitized = sanitized.replace(&marker.to_ascii_uppercase(), REDACTED_JSON_VALUE);
    }
    sanitized
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sanitize_json_omits_sensitive_object_keys() {
        let input = json!({
            "trace_id": "t-1",
            "raw_prompt": "do not leak",
            "nested": { "secret_key": "x", "ok": 1 }
        });
        let output = sanitize_json(input);
        assert_eq!(output["trace_id"], "t-1");
        assert!(output.get("raw_prompt").is_none());
        assert!(output["nested"].get("secret_key").is_none());
        assert_eq!(output["nested"]["ok"], 1);
    }

    #[test]
    fn sanitize_json_redacts_sensitive_strings() {
        let input = json!({ "message": "contains api_key=abc" });
        let output = sanitize_json(input);
        assert_eq!(output["message"], REDACTED_JSON_VALUE);
    }

    #[test]
    fn is_safe_metadata_key_rejects_credential_fields() {
        assert!(!is_safe_metadata_key("oauth_token"));
        assert!(is_safe_metadata_key("reason_code"));
    }

    #[test]
    fn should_omit_metadata_map_key_covers_supply_chain_and_credentials() {
        assert!(should_omit_metadata_map_key("oauth_token"));
        assert!(should_omit_metadata_map_key("wasm_package_bytes"));
        assert!(should_omit_metadata_map_key("raw_prompt"));
        assert!(!should_omit_metadata_map_key("reason_code"));
    }

    #[test]
    fn is_sensitive_json_key_matches_inline_store_rules() {
        assert!(is_sensitive_json_key("user_prompt"));
        assert!(is_sensitive_json_key("signing_token"));
        assert!(!is_sensitive_json_key("trace_id"));
    }

    #[test]
    fn sanitize_json_for_harness_partially_masks_strings() {
        let input = json!({ "note": "prefix secret suffix" });
        let output = sanitize_json_for_harness(input);
        assert_eq!(
            output["note"].as_str().unwrap(),
            format!("prefix {REDACTED_JSON_VALUE} suffix")
        );
    }
}
