//! Serde contract tests for MCP wire types.

use super::super::types::McpToolDef;

#[test]
fn test_mcp_tool_def_serde() {
    let def = McpToolDef {
        name: "read_file".to_string(),
        description: "Reads a file".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" }
            },
            "required": ["path"]
        }),
    };

    let json = serde_json::to_string(&def).unwrap();
    let deser: McpToolDef = serde_json::from_str(&json).unwrap();
    assert_eq!(deser.name, "read_file");
    assert_eq!(deser.description, "Reads a file");
    assert!(deser.input_schema["properties"]["path"]["type"] == "string");
}

#[test]
fn test_mcp_tool_def_accepts_camel_case_schema() {
    let json = r#"{
        "name": "browser_navigate",
        "description": "Navigate",
        "inputSchema": {
            "type": "object",
            "properties": {
                "url": { "type": "string" }
            },
            "required": ["url"]
        }
    }"#;
    let def: McpToolDef = serde_json::from_str(json).unwrap();
    assert_eq!(def.name, "browser_navigate");
    assert_eq!(def.input_schema["properties"]["url"]["type"], "string");
}
