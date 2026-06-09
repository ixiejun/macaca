mod tests {
    use super::super::*;
    use std::collections::HashMap;

    // -----------------------------------------------------------------------
    // MockMcpClient
    // -----------------------------------------------------------------------

    struct MockMcpClient {
        tools: Vec<McpToolDef>,
        connected: bool,
        call_responses: HashMap<String, McpCallResult>,
    }

    struct LocalEchoTool {
        name: String,
    }

    impl LocalEchoTool {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
            }
        }
    }

    #[async_trait]
    impl ToolHandler for LocalEchoTool {
        async fn execute(&self, _args: Value) -> Result<ToolResponse, ToolError> {
            Ok(ToolResponse::text("local"))
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            "local echo"
        }

        fn schema(&self) -> Value {
            serde_json::json!({"type": "object"})
        }
    }

    impl MockMcpClient {
        fn new() -> Self {
            Self {
                tools: Vec::new(),
                connected: false,
                call_responses: HashMap::new(),
            }
        }

        fn with_tool(mut self, name: &str, description: &str) -> Self {
            self.tools.push(McpToolDef {
                name: name.to_string(),
                description: description.to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "input": { "type": "string" }
                    }
                }),
            });
            self
        }

        fn with_response(mut self, tool_name: &str, result: McpCallResult) -> Self {
            self.call_responses.insert(tool_name.to_string(), result);
            self
        }
    }

    #[async_trait]
    impl McpClient for MockMcpClient {
        async fn connect(&mut self) -> Result<(), McpError> {
            if self.connected {
                return Err(McpError::AlreadyConnected);
            }
            self.connected = true;
            Ok(())
        }

        async fn list_tools(&mut self) -> Result<Vec<McpToolDef>, McpError> {
            if !self.connected {
                return Err(McpError::NotConnected);
            }
            Ok(self.tools.clone())
        }

        async fn call_tool(&mut self, name: &str, _args: Value) -> Result<McpCallResult, McpError> {
            if !self.connected {
                return Err(McpError::NotConnected);
            }
            self.call_responses
                .get(name)
                .cloned()
                .ok_or_else(|| McpError::ToolNotFound(name.to_string()))
        }

        async fn close(&mut self) -> Result<(), McpError> {
            self.connected = false;
            Ok(())
        }

        fn is_connected(&self) -> bool {
            self.connected
        }
    }

    // -----------------------------------------------------------------------
    // Tests
    // -----------------------------------------------------------------------

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

    #[tokio::test]
    async fn test_mock_client_connect_and_list() {
        let mut client = MockMcpClient::new()
            .with_tool("tool_a", "Tool A description")
            .with_tool("tool_b", "Tool B description");

        // Not connected yet.
        assert!(!client.is_connected());

        // Connect.
        client.connect().await.unwrap();
        assert!(client.is_connected());

        // List tools.
        let tools = client.list_tools().await.unwrap();
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].name, "tool_a");
        assert_eq!(tools[1].name, "tool_b");
    }

    #[tokio::test]
    async fn test_mock_client_call_tool() {
        let result = McpCallResult {
            content: vec![ContentBlock::Text(TextBlock {
                text: "hello from mcp".to_string(),
            })],
            is_error: false,
            metadata: None,
        };

        let mut client = MockMcpClient::new()
            .with_tool("greet", "Greets the user")
            .with_response("greet", result);

        client.connect().await.unwrap();

        let call_result = client
            .call_tool("greet", serde_json::json!({"name": "world"}))
            .await
            .unwrap();

        assert!(!call_result.is_error);
        assert_eq!(call_result.content.len(), 1);
        if let ContentBlock::Text(tb) = &call_result.content[0] {
            assert_eq!(tb.text, "hello from mcp");
        } else {
            panic!("expected TextBlock");
        }
    }

    #[tokio::test]
    async fn test_mock_client_not_connected() {
        let mut client = MockMcpClient::new().with_tool("t", "desc");

        // list_tools without connect should fail.
        let err = client
            .call_tool("t", serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(matches!(err, McpError::NotConnected));
    }

    #[tokio::test]
    async fn test_mcp_tool_handler_as_tool() {
        let result = McpCallResult {
            content: vec![ContentBlock::Text(TextBlock {
                text: "result data".to_string(),
            })],
            is_error: false,
            metadata: Some(serde_json::json!({"tokens": 42})),
        };

        let mut mock = MockMcpClient::new()
            .with_tool("my_tool", "Does something")
            .with_response("my_tool", result);
        mock.connect().await.unwrap();

        let client: Arc<tokio::sync::RwLock<dyn McpClient>> =
            Arc::new(tokio::sync::RwLock::new(mock));

        let tool_def = McpToolDef {
            name: "my_tool".to_string(),
            description: "Does something".to_string(),
            input_schema: serde_json::json!({"type": "object"}),
        };

        let handler = McpToolHandler::new(client, tool_def);

        // Verify ToolHandler trait methods.
        assert_eq!(handler.name(), "my_tool");
        assert_eq!(handler.description(), "Does something");
        assert_eq!(handler.schema(), serde_json::json!({"type": "object"}));

        // Execute.
        let resp = handler.execute(serde_json::json!({})).await.unwrap();
        assert_eq!(resp.content.len(), 1);
        if let ContentBlock::Text(tb) = &resp.content[0] {
            assert_eq!(tb.text, "result data");
        } else {
            panic!("expected TextBlock");
        }
        assert_eq!(resp.metadata, Some(serde_json::json!({"tokens": 42})));
    }

    #[tokio::test]
    async fn test_register_mcp_tools() {
        let result_a = McpCallResult {
            content: vec![ContentBlock::Text(TextBlock {
                text: "a_result".to_string(),
            })],
            is_error: false,
            metadata: None,
        };
        let result_b = McpCallResult {
            content: vec![ContentBlock::Text(TextBlock {
                text: "b_result".to_string(),
            })],
            is_error: false,
            metadata: None,
        };

        let mut mock = MockMcpClient::new()
            .with_tool("mcp_a", "Tool A")
            .with_tool("mcp_b", "Tool B")
            .with_response("mcp_a", result_a)
            .with_response("mcp_b", result_b);
        mock.connect().await.unwrap();

        let client: Arc<tokio::sync::RwLock<dyn McpClient>> =
            Arc::new(tokio::sync::RwLock::new(mock));

        let mut toolkit = Toolkit::new();
        register_mcp_tools(&mut toolkit, client, "mcp_group")
            .await
            .unwrap();

        // Both tools should be registered.
        assert_eq!(toolkit.tool_count(), 2);
        assert!(toolkit.get_tool("mcp_a").is_some());
        assert!(toolkit.get_tool("mcp_b").is_some());

        // Call one of them.
        let resp = toolkit
            .call_tool("mcp_a", serde_json::json!({}))
            .await
            .unwrap();
        if let ContentBlock::Text(tb) = &resp.content[0] {
            assert_eq!(tb.text, "a_result");
        } else {
            panic!("expected TextBlock");
        }
    }

    #[tokio::test]
    async fn test_call_nonexistent_mcp_tool() {
        let mut mock = MockMcpClient::new().with_tool("exists", "Exists");
        mock.connect().await.unwrap();

        let client: Arc<tokio::sync::RwLock<dyn McpClient>> =
            Arc::new(tokio::sync::RwLock::new(mock));

        let mut toolkit = Toolkit::new();
        register_mcp_tools(&mut toolkit, client, "mcp")
            .await
            .unwrap();

        // Calling a tool that was never registered should fail.
        let err = toolkit
            .call_tool("does_not_exist", serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::NotFound(_)));

        // Calling a registered MCP tool whose backend doesn't have a response
        // should also fail (ToolNotFound from mock → ExecutionFailed).
        let err = toolkit
            .call_tool("exists", serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::ExecutionFailed(_)));
    }

    #[test]
    fn test_parse_call_result_basic() {
        let result = serde_json::json!({
            "content": [
                {"type": "text", "text": "hello"},
                {"type": "text", "text": " world"}
            ],
            "isError": false
        });
        let parsed = parse_call_result(&result).unwrap();
        assert!(!parsed.is_error);
        assert_eq!(parsed.content.len(), 2);
    }

    #[test]
    fn test_parse_call_result_error() {
        let result = serde_json::json!({
            "content": [{"type": "text", "text": "something went wrong"}],
            "isError": true
        });
        let parsed = parse_call_result(&result).unwrap();
        assert!(parsed.is_error);
    }

    #[test]
    fn test_parse_call_result_multimodal_and_resource_fallback() {
        let result = serde_json::json!({
            "content": [
                {"type": "image", "data": "abc", "mimeType": "image/png"},
                {"type": "audio", "data": "def", "mimeType": "audio/wav"},
                {"type": "resource", "resource": {"uri": "file://tmp.txt", "text": "resource text"}},
                {"type": "unknown", "value": 1}
            ],
            "isError": false,
            "_meta": {"server": "test"}
        });
        let parsed = parse_call_result(&result).unwrap();
        assert!(!parsed.is_error);
        assert_eq!(parsed.content.len(), 4);
        assert!(matches!(parsed.content[0], ContentBlock::Image(_)));
        assert!(matches!(parsed.content[1], ContentBlock::Audio(_)));
        match &parsed.content[2] {
            ContentBlock::Text(text) => assert_eq!(text.text, "resource text"),
            _ => panic!("expected text resource fallback"),
        }
        match &parsed.content[3] {
            ContentBlock::Text(text) => assert!(text.text.contains("\"unknown\"")),
            _ => panic!("expected json text fallback"),
        }
        assert_eq!(parsed.metadata, Some(serde_json::json!({"server": "test"})));
    }

    #[tokio::test]
    async fn test_register_mcp_tools_raises_on_collision() {
        let mut mock = MockMcpClient::new().with_tool("exists", "MCP Exists");
        mock.connect().await.unwrap();
        let client: Arc<tokio::sync::RwLock<dyn McpClient>> =
            Arc::new(tokio::sync::RwLock::new(mock));

        let mut toolkit = Toolkit::new();
        toolkit.register(Box::new(LocalEchoTool::new("exists")), None);

        let err = register_mcp_tools_with_options(
            &mut toolkit,
            client,
            McpToolRegistrationOptions::new("mcp"),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, McpError::ToolNameCollision(_)));
    }

    #[tokio::test]
    async fn test_register_mcp_tools_prefixes_collision() {
        let result = McpCallResult {
            content: vec![ContentBlock::Text(TextBlock {
                text: "mcp result".to_string(),
            })],
            is_error: false,
            metadata: None,
        };
        let mut mock = MockMcpClient::new()
            .with_tool("exists", "MCP Exists")
            .with_response("exists", result);
        mock.connect().await.unwrap();
        let client: Arc<tokio::sync::RwLock<dyn McpClient>> =
            Arc::new(tokio::sync::RwLock::new(mock));

        let mut toolkit = Toolkit::new();
        toolkit.register(Box::new(LocalEchoTool::new("exists")), None);

        register_mcp_tools_with_options(
            &mut toolkit,
            client,
            McpToolRegistrationOptions {
                group_name: "mcp".to_string(),
                conflict_policy: McpToolNameConflictPolicy::Prefix("mcp_".to_string()),
                disabled_tools: HashSet::new(),
                on_close: None,
            },
        )
        .await
        .unwrap();

        assert!(toolkit.get_tool("exists").is_some());
        assert!(toolkit.get_tool("mcp_exists").is_some());
        let resp = toolkit
            .call_tool("mcp_exists", serde_json::json!({}))
            .await
            .unwrap();
        match &resp.content[0] {
            ContentBlock::Text(text) => assert_eq!(text.text, "mcp result"),
            _ => panic!("expected text"),
        }
    }

    #[tokio::test]
    async fn test_register_mcp_tools_skips_disabled_tools() {
        let mut mock = MockMcpClient::new()
            .with_tool("allowed", "Allowed")
            .with_tool("blocked", "Blocked");
        mock.connect().await.unwrap();
        let client: Arc<tokio::sync::RwLock<dyn McpClient>> =
            Arc::new(tokio::sync::RwLock::new(mock));

        let mut disabled_tools = HashSet::new();
        disabled_tools.insert("blocked".to_string());
        let mut toolkit = Toolkit::new();
        let registered = register_mcp_tools_with_options(
            &mut toolkit,
            client,
            McpToolRegistrationOptions {
                group_name: "mcp".to_string(),
                conflict_policy: McpToolNameConflictPolicy::Raise,
                disabled_tools,
                on_close: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(registered, vec!["allowed"]);
        assert!(toolkit.get_tool("allowed").is_some());
        assert!(toolkit.get_tool("blocked").is_none());
    }
}
