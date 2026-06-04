//! User-facing display projection for mirrored agent execution events.
//!
//! The application-execution EventLog is a durable audit/replay surface.  The
//! mirror may include enough text for a human to understand progress, but the
//! payload must remain bounded and sanitized because it can outlive the browser
//! tab and be replayed after refresh.  This module applies the Presenter pattern
//! inside the Web shell layer: it converts already-observed generic agent events
//! into display titles and summaries without owning execution semantics.

pub(crate) const MAX_DISPLAY_CHARS: usize = 600;
pub(crate) const MAX_DISPLAY_LINES: usize = 12;

/// Merge display-oriented keys with audit details into one JSON object.
///
/// Keeping the construction centralized prevents individual event branches from
/// drifting into raw protocol dumps.  Display keys are intentionally ordinary
/// data fields so any shell, not only the Codex workbench, can render them.
pub(crate) fn merge_display(
    display: serde_json::Value,
    details: serde_json::Value,
) -> serde_json::Value {
    let mut merged = serde_json::Map::new();
    if let Some(values) = display.as_object() {
        merged.extend(values.clone());
    }
    if let Some(values) = details.as_object() {
        merged.extend(values.clone());
    }
    serde_json::Value::Object(merged)
}

/// Return a bounded, whitespace-normalized, secret-masked display excerpt.
///
/// This intentionally preserves short human-readable content such as the task
/// title or assistant progress while enforcing stable replay limits.  It is not
/// a security classifier; it is a last-mile observability guard that avoids
/// obvious credential-shaped tokens and unbounded output in EventLog rows.
pub(crate) fn display_excerpt(value: &str) -> String {
    let lines = value
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let line_truncated = lines.len() > MAX_DISPLAY_LINES;
    let normalized = lines
        .iter()
        .take(MAX_DISPLAY_LINES)
        .copied()
        .collect::<Vec<_>>()
        .join("\n");
    let char_truncated = normalized.chars().count() > MAX_DISPLAY_CHARS;
    let mut excerpt = normalized
        .chars()
        .take(MAX_DISPLAY_CHARS)
        .collect::<String>();
    if line_truncated || char_truncated {
        excerpt.push_str("...");
    }
    let masked = mask_secret_like_tokens(&excerpt);
    if masked.is_empty() {
        "No displayable content was provided.".into()
    } else {
        masked
    }
}

/// Return full user-visible display text after lightweight secret masking.
///
/// The application execution EventLog still enforces its own per-row payload
/// size limit and recursive secret-key redaction.  This helper simply avoids the
/// UI-specific short excerpt for content that the agent has already emitted as
/// user-facing assistant text, reasoning progress, or tool output.
pub(crate) fn display_full(value: &str) -> String {
    let masked = mask_secret_like_tokens(value);
    if masked.trim().is_empty() {
        "No displayable content was provided.".into()
    } else {
        masked
    }
}

/// Summarize tool input without persisting raw tool arguments.
///
/// File-oriented tools receive a special path summary because users need to see
/// which artifact changed.  Other tools fall back to a bounded JSON excerpt so
/// the UI can still communicate intent without becoming tool-specific code.
pub(crate) fn tool_input_summary(tool_name: &str, tool_input: &serde_json::Value) -> String {
    if let Some(path) = tool_file_path(tool_input) {
        let bytes = tool_input
            .get("content")
            .and_then(serde_json::Value::as_str)
            .map(str::len)
            .unwrap_or(0);
        if let Some(content) = tool_input
            .get("content")
            .and_then(serde_json::Value::as_str)
        {
            return format!(
                "Preparing `{tool_name}` for `{path}` ({bytes} content bytes).\n\n```text\n{}\n```",
                display_full(content)
            );
        }
        return format!("Preparing `{tool_name}` for `{path}` ({bytes} content bytes).");
    }
    format!(
        "Preparing `{tool_name}` with sanitized input {}.",
        display_excerpt(&tool_input.to_string())
    )
}

/// Extract a generic file path from common tool payload shapes.
///
/// The helper does not branch on application names or workflow names.  It only
/// recognizes conventional path fields used by many file-capable tools.
pub(crate) fn tool_file_path(tool_input: &serde_json::Value) -> Option<String> {
    tool_input
        .get("path")
        .or_else(|| tool_input.get("file_path"))
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            tool_input
                .pointer("/target/path")
                .and_then(serde_json::Value::as_str)
        })
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
}

/// Summarize tool output while keeping raw stdout/result bodies out of EventLog.
///
/// Structured outputs that report a path or summary are rendered directly after
/// bounding.  Arbitrary output is represented by a bounded excerpt plus hash and
/// length fields held by the caller for audit correlation.
pub(crate) fn tool_output_summary(tool_name: &str, output: &str, is_error: Option<bool>) -> String {
    let status = if is_error.unwrap_or(false) {
        "reported an error"
    } else {
        "completed successfully"
    };
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(output) {
        if let Some(path) = value
            .get("path")
            .or_else(|| value.get("file_path"))
            .and_then(serde_json::Value::as_str)
        {
            return format!("`{tool_name}` {status} for `{path}`.");
        }
        if let Some(message) = value
            .get("message")
            .or_else(|| value.get("summary"))
            .and_then(serde_json::Value::as_str)
        {
            return display_excerpt(message);
        }
    }
    format!("`{tool_name}` {status}: {}", display_excerpt(output))
}

/// Produce a short driver trace summary from provider-neutral trace fields.
pub(crate) fn driver_trace_summary(trace: &serde_json::Value) -> String {
    trace
        .get("summary")
        .or_else(|| trace.get("message"))
        .and_then(serde_json::Value::as_str)
        .map(display_excerpt)
        .unwrap_or_else(|| {
            let event_type = trace
                .get("event_type")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("driver event");
            format!("Observed {event_type}.")
        })
}

fn mask_secret_like_tokens(content: &str) -> String {
    content
        .split_whitespace()
        .map(|token| {
            if token.starts_with("sk-") && token.len() > 12 {
                "sk-...****".to_string()
            } else if token.starts_with("Bearer") {
                "Bearer...****".to_string()
            } else {
                token.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::{display_excerpt, display_full, MAX_DISPLAY_CHARS};

    #[test]
    fn display_excerpt_is_bounded_for_application_execution_ui_payloads() {
        let long = "line\n".repeat(200);
        let excerpt = display_excerpt(&long);

        assert!(excerpt.len() <= MAX_DISPLAY_CHARS + 3);
        assert!(excerpt.ends_with("..."));
    }

    #[test]
    fn display_excerpt_masks_obvious_secret_like_tokens() {
        let excerpt = display_excerpt("token sk-abcdefghijklmnopqrstuvwxyz Bearer abc123");

        assert!(excerpt.contains("sk-...****"));
        assert!(excerpt.contains("Bearer...****"));
        assert!(!excerpt.contains("abcdefghijklmnopqrstuvwxyz"));
    }

    #[test]
    fn display_full_preserves_user_visible_markdown_without_excerpt_truncation() {
        let markdown = format!(
            "# Result\n\n{}\n\n```rust\nfn main() {{ println!(\"hello\"); }}\n```",
            "line\n".repeat(120)
        );
        let full = display_full(&markdown);

        assert!(full.contains("```rust"));
        assert!(full.contains("fn main()"));
        assert!(
            full.len() > MAX_DISPLAY_CHARS,
            "full display content should not use the short excerpt limit"
        );
    }

    #[test]
    fn tool_input_summary_includes_complete_file_content_for_file_payloads() {
        let input = serde_json::json!({
            "target": { "path": "shared/src/main.rs" },
            "content": "fn main() {\n    println!(\"hello\");\n}"
        });
        let summary = super::tool_input_summary("macaca_file", &input);

        assert!(summary.contains("shared/src/main.rs"));
        assert!(summary.contains("fn main()"));
        assert!(summary.contains("println!"));
    }
}
