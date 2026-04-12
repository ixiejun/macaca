//! Serialize tool call `arguments` for OpenAI-compatible chat APIs.
//!
//! Some providers (e.g. MiniMax) reject the next request if `tool_calls[].function.arguments`
//! is not a valid JSON **object** string. We store arguments as [`serde_json::Value`]; when the
//! upstream model returned malformed JSON, parse falls back to [`Value::String`]. Using
//! [`Value::to_string()`] on that variant can produce a quoted JSON string (wrong type for
//! strict APIs) or invalid fragments — normalize here.

use serde_json::Value;

/// Produce the `function.arguments` string field for chat/completions requests.
pub fn tool_arguments_for_chat_api(args: &Value) -> String {
    match args {
        Value::String(s) => normalize_arguments_str(s),
        Value::Null => "{}".to_string(),
        other => serde_json::to_string(other).unwrap_or_else(|_| "{}".to_string()),
    }
}

fn normalize_arguments_str(s: &str) -> String {
    let t = s.trim();
    if t.is_empty() {
        return "{}".to_string();
    }

    // 1. 标准解析
    if let Ok(v) = serde_json::from_str::<Value>(t) {
        return serde_json::to_string(&v).unwrap_or_else(|_| "{}".to_string());
    }

    // 2. 尝试修复常见 JSON 格式问题
    if let Some(repaired) = try_repair_json(t) {
        tracing::warn!(
            original_len = t.len(),
            "Repaired malformed tool arguments JSON"
        );
        return repaired;
    }

    // 3. 降级: 记录警告，返回包含原始字符串的对象
    tracing::warn!(
        len = t.len(),
        preview = %t.chars().take(120).collect::<String>(),
        "tool arguments string was not valid JSON; using fallback"
    );
    // 不再返回空对象，而是包装原始输入
    serde_json::to_string(&serde_json::json!({"raw_input": t})).unwrap_or_else(|_| "{}".to_string())
}

/// 尝试修复常见的 JSON 格式错误
fn try_repair_json(raw: &str) -> Option<String> {
    let mut s: String = raw
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t' || *c == '\r')
        .collect();

    // 移除尾部逗号: `,}` / `,]`（逗号与闭合符之间可能有空白）
    loop {
        let before = s.clone();
        s = remove_trailing_commas(&s);
        if s == before {
            break;
        }
    }

    // 尝试解析修复后的字符串
    if let Ok(v) = serde_json::from_str::<Value>(&s) {
        return Some(serde_json::to_string(&v).unwrap_or_else(|_| "{}".to_string()));
    }

    None
}

/// 移除 JSON 中 `,}` 和 `,]` 模式（含中间空白）
fn remove_trailing_commas(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut result = Vec::with_capacity(bytes.len());
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b',' {
            // 向前扫描跳过空白，看下一个非空白字符是否为 } 或 ]
            let mut j = i + 1;
            while j < len
                && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n' || bytes[j] == b'\r')
            {
                j += 1;
            }
            if j < len && (bytes[j] == b'}' || bytes[j] == b']') {
                // 跳过逗号，保留空白和闭合符由后续迭代处理
                i += 1;
                continue;
            }
        }
        result.push(bytes[i]);
        i += 1;
    }

    // SAFETY: 我们只移除了 ASCII 逗号，不会破坏 UTF-8
    unsafe { String::from_utf8_unchecked(result) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_roundtrip() {
        let v = serde_json::json!({"path": "/tmp/a", "content": "x"});
        let s = tool_arguments_for_chat_api(&v);
        let m: serde_json::Map<String, Value> = serde_json::from_str(&s).unwrap();
        assert_eq!(m.get("path").and_then(|x| x.as_str()), Some("/tmp/a"));
        assert_eq!(m.get("content").and_then(|x| x.as_str()), Some("x"));
    }

    #[test]
    fn string_containing_object() {
        let v = Value::String(r#"{"path":"/b"}"#.to_string());
        let s = tool_arguments_for_chat_api(&v);
        assert_eq!(s, r#"{"path":"/b"}"#);
    }

    #[test]
    fn invalid_string_becomes_fallback() {
        let v = Value::String("not json {".to_string());
        let result = tool_arguments_for_chat_api(&v);
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert!(parsed.get("raw_input").is_some());
    }

    #[test]
    fn test_repair_trailing_comma() {
        let result = normalize_arguments_str(r#"{"key": "value",}"#);
        assert_eq!(result, r#"{"key":"value"}"#);
    }

    #[test]
    fn test_repair_trailing_comma_array() {
        let result = normalize_arguments_str(r#"{"items": [1, 2, 3,]}"#);
        assert_eq!(result, r#"{"items":[1,2,3]}"#);
    }

    #[test]
    fn test_non_json_input_wrapped() {
        let result = normalize_arguments_str("just a string");
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert!(parsed.get("raw_input").is_some());
    }

    #[test]
    fn test_repair_control_characters() {
        let input = "{\"key\": \"val\x01ue\"}";
        let result = normalize_arguments_str(input);
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed.get("key").and_then(|v| v.as_str()), Some("value"));
    }

    #[test]
    fn test_repair_nested_trailing_commas() {
        let result = normalize_arguments_str(r#"{"a": {"b": 1,}, "c": [1,],}"#);
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["a"]["b"], 1);
        assert_eq!(parsed["c"][0], 1);
    }
}
