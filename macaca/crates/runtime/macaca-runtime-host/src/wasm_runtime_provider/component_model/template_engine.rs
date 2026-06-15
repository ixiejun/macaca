//! Host-command template resolution for component-model sessions.
//!
//! **Pattern:** Template Method — placeholder substitution over declared
//! host-command templates using export payload context and prior host results.
//!
//! Exact placeholders resolve to native JSON values; mixed text strings use
//! interpolation so GenUI labels and display text can embed scalar fields.

use serde_json::Value;

/// Recursively applies host-command template placeholders to a JSON value tree.
///
/// String leaves attempt exact placeholder resolution first (`${chat.*}`,
/// `${host.results.*}`), then fall back to embedded interpolation for mixed
/// display text such as `"${chat.symbol} Signal Analysis"`.
pub(super) fn apply_host_command_template(
    value: Value,
    export_payload: &Value,
    host_results: &[Value],
) -> Value {
    match value {
        Value::String(text) => resolve_chat_payload_template(&text, export_payload)
            .or_else(|| resolve_host_result_template(&text, host_results))
            .unwrap_or_else(|| {
                Value::String(interpolate_host_command_template(
                    &text,
                    export_payload,
                    host_results,
                ))
            }),
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .map(|item| apply_host_command_template(item, export_payload, host_results))
                .collect(),
        ),
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, item)| {
                    (
                        key,
                        apply_host_command_template(item, export_payload, host_results),
                    )
                })
                .collect(),
        ),
        other => other,
    }
}

/// Interpolate embedded placeholders inside human-facing strings.
///
/// Exact placeholders are still resolved by `resolve_*_template` first so JSON
/// payload fields keep their native value type.  This helper is only for mixed
/// text such as `"${chat.symbol} Signal Analysis"`, where a string result is
/// already the correct output type for GenUI labels, Markdown, or display text.
fn interpolate_host_command_template(
    text: &str,
    export_payload: &Value,
    host_results: &[Value],
) -> String {
    let mut rendered = String::with_capacity(text.len());
    let mut remainder = text;
    while let Some(start) = remainder.find("${") {
        rendered.push_str(&remainder[..start]);
        let after_start = &remainder[start..];
        let Some(end) = after_start.find('}') else {
            rendered.push_str(after_start);
            return rendered;
        };
        let placeholder = &after_start[..=end];
        if let Some(value) = resolve_chat_payload_template(placeholder, export_payload)
            .or_else(|| resolve_host_result_template(placeholder, host_results))
        {
            rendered.push_str(&template_value_to_display_text(&value));
        } else {
            rendered.push_str(placeholder);
        }
        remainder = &after_start[end + 1..];
    }
    rendered.push_str(remainder);
    rendered
}

/// Convert an interpolated JSON value into stable display text.
///
/// Scalar values render without JSON quotes so labels stay clean; structured
/// values use compact JSON so audit text remains deterministic if an app embeds
/// a non-scalar placeholder in a display string.
fn template_value_to_display_text(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(value) => value.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Array(_) | Value::Object(_) => {
            serde_json::to_string(value).unwrap_or_else(|_| value.to_string())
        }
    }
}

/// Resolve an exact chat-payload placeholder without interpreting its content.
///
/// This is intentionally a structural lookup only.  `${chat.symbol}` means the
/// app/coordinator already supplied a typed `symbol` field in the export
/// payload; the runtime must never parse `${chat.input}` or infer domain data
/// from free-form prose.
pub(super) fn resolve_chat_payload_template(text: &str, export_payload: &Value) -> Option<Value> {
    let path = text.strip_prefix("${chat.")?.strip_suffix('}')?;
    // Newer hosted-application execution envelopes place user input under a
    // typed `chat` object so runtime metadata can carry session/run/workspace
    // fields without colliding with application-owned payload keys.  Earlier
    // declarative WASM tests and packages used top-level fields such as
    // `${chat.input}` -> `payload.input`.  Prefer the explicit `chat` object and
    // fall back to the top-level shape for already-authored packages.
    let mut current = export_payload
        .get("chat")
        .cloned()
        .unwrap_or_else(|| export_payload.clone());
    for part in path.split('.') {
        current = match current {
            Value::Object(map) => map.get(part)?.clone(),
            Value::Array(items) => {
                let item_index = part.parse::<usize>().ok()?;
                items.get(item_index)?.clone()
            }
            _ => return None,
        };
    }
    Some(current)
}

/// Resolve a full-value placeholder against prior declared host-command results.
///
/// The portable Component Model adapter cannot yet execute guest bytecode, so
/// it models guest orchestration through declarative host-command metadata.  A
/// later command may reference a prior command result with a bounded JSON path
/// such as `${host.results.0.output}` or `${host.results.2.output.analysis}`.
/// The runtime resolves only exact full-value placeholders; it deliberately
/// avoids string interpolation so payload types remain JSON-native and audit
/// output stays predictable.
pub(super) fn resolve_host_result_template(text: &str, host_results: &[Value]) -> Option<Value> {
    let path = text.strip_prefix("${host.results.")?.strip_suffix('}')?;
    let mut parts = path.split('.');
    let index = parts.next()?.parse::<usize>().ok()?;
    let mut current = host_results.get(index)?.clone();
    for part in parts {
        current = match current {
            Value::Object(map) => map.get(part)?.clone(),
            Value::Array(items) => {
                let item_index = part.parse::<usize>().ok()?;
                items.get(item_index)?.clone()
            }
            _ => return None,
        };
    }
    Some(current)
}
