//! `tool_schema!` macro contract tests.

#[test]
fn test_tool_schema_macro_basic() {
    let schema = crate::tool_schema!({
        "query" => { "type": "string", "description": "Search query" },
        "count" => { "type": "integer", "description": "Number of results" }
    });

    assert_eq!(schema["type"], "object");
    assert_eq!(schema["properties"]["query"]["type"], "string");
    assert_eq!(schema["properties"]["count"]["type"], "integer");

    let required = schema["required"].as_array().unwrap();
    assert!(required.contains(&serde_json::json!("query")));
    assert!(required.contains(&serde_json::json!("count")));
    assert_eq!(required.len(), 2);
}

#[test]
fn test_tool_schema_macro_with_defaults() {
    let schema = crate::tool_schema!({
        "query" => { "type": "string", "description": "Search query" },
        "limit" => { "type": "integer", "description": "Max results", "default": 5 }
    });

    let required = schema["required"].as_array().unwrap();
    assert!(required.contains(&serde_json::json!("query")));
    assert!(!required.contains(&serde_json::json!("limit")));
    assert_eq!(required.len(), 1);

    assert_eq!(schema["properties"]["limit"]["default"], 5);
}

#[test]
fn test_tool_schema_macro_empty() {
    let schema = crate::tool_schema!({});

    assert_eq!(schema["type"], "object");
    assert_eq!(schema["properties"], serde_json::json!({}));
    assert_eq!(schema["required"], serde_json::json!([]));
}
