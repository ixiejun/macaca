//! Policy and resource Specifications for local-file service admission.
//!
//! The policy layer consumes only bounded, provider-neutral facts. It never
//! opens a host path, reads file bytes, or exposes a file name in diagnostics.

/// Validate picker, transfer, traversal, retention, and approval facts before
/// the local-files provider allocates a handle or transfer ledger entry.
pub fn local_files_policy_denial(
    operation: &str,
    payload: &serde_json::Value,
) -> Option<&'static str> {
    let denied = |key: &str, reason: &'static str| {
        (payload.get(key).and_then(serde_json::Value::as_bool) == Some(true)).then_some(reason)
    };
    ((payload
        .get("permission_granted")
        .and_then(serde_json::Value::as_bool)
        == Some(false))
    .then_some("permission_denied"))
    .or_else(|| denied("picker_unavailable", "picker_unavailable"))
    .or_else(|| denied("foreground_required", "foreground_required"))
    .or_else(|| denied("mime_filter_invalid", "mime_filter_invalid"))
    .or_else(|| denied("grant_persistence_denied", "grant_persistence_denied"))
    .or_else(|| denied("directory_traversal", "directory_traversal_denied"))
    .or_else(|| denied("retention_denied", "retention_policy_denied"))
    .or_else(|| denied("content_scan_blocked", "content_scan_blocked"))
    .or_else(|| denied("quota_exceeded", "quota_exceeded"))
    .or_else(|| denied("cancelled", "transfer_cancelled"))
    .or_else(|| denied("raw_path_requested", "raw_path_redacted"))
    .or_else(|| {
        bounded_u64(
            payload,
            "transfer_size_bytes",
            10_000_000,
            "transfer_size_exceeded",
        )
    })
    .or_else(|| bounded_u64(payload, "directory_depth", 16, "directory_traversal_denied"))
    .or_else(|| {
        bounded_u64(
            payload,
            "directory_entry_count",
            1_000,
            "directory_entries_exceeded",
        )
    })
    .or_else(|| bounded_u64(payload, "chunk_count", 1_024, "chunk_quota_exceeded"))
    .or_else(|| {
        bounded_u64(
            payload,
            "memory_bytes",
            64 * 1024 * 1024,
            "memory_quota_exceeded",
        )
    })
    .or_else(|| {
        bounded_u64(
            payload,
            "storage_bytes",
            100 * 1024 * 1024,
            "storage_quota_exceeded",
        )
    })
    .or_else(|| {
        bounded_u64(
            payload,
            "retained_snapshot_count",
            100,
            "snapshot_quota_exceeded",
        )
    })
    .or_else(|| {
        bounded_u64(
            payload,
            "replay_metadata_bytes",
            65_536,
            "replay_metadata_exceeded",
        )
    })
    .or_else(|| {
        (payload
            .get("timeout_ms")
            .and_then(serde_json::Value::as_u64)
            == Some(0))
        .then_some("timeout")
    })
    .or_else(|| {
        (operation == "local_files.request_directory_handle"
            && payload
                .get("approval_required")
                .and_then(serde_json::Value::as_bool)
                == Some(true)
            && payload.get("approved").and_then(serde_json::Value::as_bool) != Some(true))
        .then_some("directory_grant_approval_required")
    })
    .or_else(|| {
        (payload
            .get("destructive")
            .and_then(serde_json::Value::as_bool)
            == Some(true)
            && payload.get("approved").and_then(serde_json::Value::as_bool) != Some(true))
        .then_some("destructive_approval_required")
    })
    .or_else(|| {
        (payload
            .get("large_transfer")
            .and_then(serde_json::Value::as_bool)
            == Some(true)
            && payload.get("approved").and_then(serde_json::Value::as_bool) != Some(true))
        .then_some("large_transfer_approval_required")
    })
    .or_else(|| {
        (payload
            .get("remote_host_access")
            .and_then(serde_json::Value::as_bool)
            == Some(true)
            && payload.get("approved").and_then(serde_json::Value::as_bool) != Some(true))
        .then_some("remote_host_approval_required")
    })
    .or_else(|| {
        (payload
            .get("sensitive_category")
            .and_then(serde_json::Value::as_bool)
            == Some(true)
            && payload.get("approved").and_then(serde_json::Value::as_bool) != Some(true))
        .then_some("sensitive_category_approval_required")
    })
    .or_else(|| {
        (payload
            .get("grant_state")
            .and_then(serde_json::Value::as_str)
            == Some("expired"))
        .then_some("grant_expired")
    })
    .or_else(|| {
        (payload
            .get("grant_state")
            .and_then(serde_json::Value::as_str)
            == Some("revoked"))
        .then_some("handle_revoked")
    })
}

fn bounded_u64(
    payload: &serde_json::Value,
    key: &str,
    maximum: u64,
    reason: &'static str,
) -> Option<&'static str> {
    payload
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .filter(|value| *value > maximum)
        .map(|_| reason)
}

#[cfg(test)]
mod tests {
    use super::local_files_policy_denial;

    #[test]
    fn policy_rejects_unbounded_resources_and_missing_approvals() {
        assert_eq!(
            local_files_policy_denial(
                "local_files.export_file",
                &serde_json::json!({"transfer_size_bytes": 10_000_001})
            ),
            Some("transfer_size_exceeded")
        );
        assert_eq!(
            local_files_policy_denial(
                "local_files.write",
                &serde_json::json!({"destructive": true, "approved": false})
            ),
            Some("destructive_approval_required")
        );
        assert_eq!(
            local_files_policy_denial(
                "local_files.request_directory_handle",
                &serde_json::json!({"directory_depth": 17})
            ),
            Some("directory_traversal_denied")
        );
    }
}
