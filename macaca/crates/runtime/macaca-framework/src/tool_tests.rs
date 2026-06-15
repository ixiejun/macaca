mod tests {
    use super::super::*;
    use async_trait::async_trait;
    use serde_json::Value;

    use crate::message::{ContentBlock, TextBlock};

    // -----------------------------------------------------------------------
    // Mock tools
    // -----------------------------------------------------------------------

    /// EchoTool — returns its input arguments as JSON text.
    struct EchoTool;

    #[async_trait]
    impl ToolHandler for EchoTool {
        async fn execute(&self, args: Value) -> Result<ToolResponse, ToolError> {
            Ok(ToolResponse::json(args))
        }
        fn name(&self) -> &str {
            "echo"
        }
        fn description(&self) -> &str {
            "Echoes its input arguments as JSON."
        }
        fn schema(&self) -> Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "message": { "type": "string" }
                }
            })
        }
    }

    /// AddTool — expects `{"a": number, "b": number}`, returns their sum.
    struct AddTool;

    #[async_trait]
    impl ToolHandler for AddTool {
        async fn execute(&self, args: Value) -> Result<ToolResponse, ToolError> {
            let a = args["a"]
                .as_f64()
                .ok_or_else(|| ToolError::InvalidArgs("missing 'a'".into()))?;
            let b = args["b"]
                .as_f64()
                .ok_or_else(|| ToolError::InvalidArgs("missing 'b'".into()))?;
            Ok(ToolResponse::text(format!("{}", a + b)))
        }
        fn name(&self) -> &str {
            "add"
        }
        fn description(&self) -> &str {
            "Adds two numbers."
        }
        fn schema(&self) -> Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "a": { "type": "number" },
                    "b": { "type": "number" }
                },
                "required": ["a", "b"]
            })
        }
    }

    // -----------------------------------------------------------------------
    // Middleware mock
    // -----------------------------------------------------------------------

    use std::sync::{Arc, Mutex};

    struct RecordingMiddleware {
        before_calls: Arc<Mutex<Vec<String>>>,
        after_calls: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl ToolMiddleware for RecordingMiddleware {
        async fn before(&self, name: &str, _args: &mut Value) -> Result<(), ToolError> {
            self.before_calls.lock().unwrap().push(name.to_string());
            Ok(())
        }
        async fn after(&self, name: &str, _response: &mut ToolResponse) -> Result<(), ToolError> {
            self.after_calls.lock().unwrap().push(name.to_string());
            Ok(())
        }
    }

    // -----------------------------------------------------------------------
    // Tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_register_and_call() {
        let mut kit = Toolkit::new();
        kit.register(Box::new(AddTool), None);

        let resp = kit
            .call_tool("add", serde_json::json!({"a": 3.0, "b": 4.0}))
            .await
            .unwrap();

        assert_eq!(resp.content.len(), 1);
        if let ContentBlock::Text(tb) = &resp.content[0] {
            assert_eq!(tb.text, "7");
        } else {
            panic!("expected TextBlock");
        }
    }

    #[tokio::test]
    async fn test_tool_not_found() {
        let kit = Toolkit::new();
        let err = kit
            .call_tool("nonexistent", serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_preset_args() {
        // Register EchoTool, then manually set preset_args.
        let mut kit = Toolkit::new();
        kit.register(Box::new(EchoTool), None);

        // Inject a preset arg.
        kit.tools.get_mut("echo").unwrap().preset_args =
            serde_json::json!({"preset_key": "preset_val"});

        // Caller provides an additional key; caller wins on conflict.
        let resp = kit
            .call_tool("echo", serde_json::json!({"caller_key": "caller_val"}))
            .await
            .unwrap();

        if let ContentBlock::Text(tb) = &resp.content[0] {
            let v: Value = serde_json::from_str(&tb.text).unwrap();
            assert_eq!(v["preset_key"], "preset_val");
            assert_eq!(v["caller_key"], "caller_val");
        } else {
            panic!("expected TextBlock");
        }
    }

    #[tokio::test]
    async fn test_middleware_chain() {
        let before_calls = Arc::new(Mutex::new(Vec::<String>::new()));
        let after_calls = Arc::new(Mutex::new(Vec::<String>::new()));

        let mw = RecordingMiddleware {
            before_calls: before_calls.clone(),
            after_calls: after_calls.clone(),
        };

        let mut kit = Toolkit::new();
        kit.add_middleware(Box::new(mw));
        kit.register(Box::new(AddTool), None);

        kit.call_tool("add", serde_json::json!({"a": 1.0, "b": 2.0}))
            .await
            .unwrap();

        assert_eq!(*before_calls.lock().unwrap(), vec!["add"]);
        assert_eq!(*after_calls.lock().unwrap(), vec!["add"]);
    }

    #[tokio::test]
    async fn test_group_active() {
        let mut kit = Toolkit::new();
        kit.register(Box::new(EchoTool), Some("optional"));

        // Should work while group is active.
        kit.call_tool("echo", serde_json::json!({})).await.unwrap();

        // Deactivate the group.
        kit.set_group_active("optional", false);

        let err = kit
            .call_tool("echo", serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::PermissionDenied(_)));
    }

    #[tokio::test]
    async fn test_get_definitions() {
        let mut kit = Toolkit::new();
        kit.register(Box::new(EchoTool), Some("group_a"));
        kit.register(Box::new(AddTool), Some("group_b"));

        // Both groups active → both definitions present.
        let defs = kit.get_definitions();
        assert_eq!(defs.len(), 2);

        // Deactivate group_a → only AddTool definition.
        kit.set_group_active("group_a", false);
        let defs = kit.get_definitions();
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0]["name"], "add");
    }

    #[tokio::test]
    async fn test_unregister() {
        let mut kit = Toolkit::new();
        kit.register(Box::new(AddTool), None);
        assert_eq!(kit.tool_count(), 1);

        kit.unregister("add");
        assert_eq!(kit.tool_count(), 0);

        let err = kit
            .call_tool("add", serde_json::json!({"a": 1.0, "b": 2.0}))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_basic_group_always_active() {
        let mut kit = Toolkit::new();
        kit.register(Box::new(AddTool), None); // defaults to "basic"

        // Attempting to deactivate "basic" is a no-op.
        kit.set_group_active("basic", false);

        // Tool must still be callable.
        kit.call_tool("add", serde_json::json!({"a": 10.0, "b": 5.0}))
            .await
            .unwrap();

        // Definition must still appear.
        let defs = kit.get_definitions();
        assert_eq!(defs.len(), 1);
    }

    // -----------------------------------------------------------------------
    // Robustness tests
    // -----------------------------------------------------------------------

    /// A tool whose name is configurable, so we can register duplicates.
    struct NamedEchoTool {
        tool_name: String,
    }

    #[async_trait]
    impl ToolHandler for NamedEchoTool {
        async fn execute(&self, args: Value) -> Result<ToolResponse, ToolError> {
            Ok(ToolResponse::json(args))
        }
        fn name(&self) -> &str {
            &self.tool_name
        }
        fn description(&self) -> &str {
            "A named echo tool."
        }
        fn schema(&self) -> Value {
            serde_json::json!({"type": "object"})
        }
    }

    /// Middleware that always fails in `before`.
    struct FailBeforeMiddleware;

    #[async_trait]
    impl ToolMiddleware for FailBeforeMiddleware {
        async fn before(&self, _name: &str, _args: &mut Value) -> Result<(), ToolError> {
            Err(ToolError::ExecutionFailed("before blocked".into()))
        }
        async fn after(&self, _name: &str, _response: &mut ToolResponse) -> Result<(), ToolError> {
            Ok(())
        }
    }

    /// Middleware that modifies the response text in `after`.
    struct ModifyAfterMiddleware;

    #[async_trait]
    impl ToolMiddleware for ModifyAfterMiddleware {
        async fn before(&self, _name: &str, _args: &mut Value) -> Result<(), ToolError> {
            Ok(())
        }
        async fn after(&self, _name: &str, response: &mut ToolResponse) -> Result<(), ToolError> {
            response.content = vec![ContentBlock::Text(TextBlock {
                text: "modified_by_after".into(),
            })];
            Ok(())
        }
    }

    /// A labeled middleware that records its label into shared vecs.
    struct LabeledMiddleware {
        label: String,
        before_log: Arc<Mutex<Vec<String>>>,
        after_log: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl ToolMiddleware for LabeledMiddleware {
        async fn before(&self, _name: &str, _args: &mut Value) -> Result<(), ToolError> {
            self.before_log.lock().unwrap().push(self.label.clone());
            Ok(())
        }
        async fn after(&self, _name: &str, _response: &mut ToolResponse) -> Result<(), ToolError> {
            self.after_log.lock().unwrap().push(self.label.clone());
            Ok(())
        }
    }

    /// A tool that tracks whether execute was called.
    struct TrackingTool {
        called: Arc<Mutex<bool>>,
    }

    #[async_trait]
    impl ToolHandler for TrackingTool {
        async fn execute(&self, _args: Value) -> Result<ToolResponse, ToolError> {
            *self.called.lock().unwrap() = true;
            Ok(ToolResponse::text("ok"))
        }
        fn name(&self) -> &str {
            "tracking"
        }
        fn description(&self) -> &str {
            "Tracks calls."
        }
        fn schema(&self) -> Value {
            serde_json::json!({"type": "object"})
        }
    }

    #[tokio::test]
    async fn test_register_duplicate_tool_name() {
        let mut kit = Toolkit::new();
        kit.register(
            Box::new(NamedEchoTool {
                tool_name: "dup".into(),
            }),
            None,
        );
        kit.register(
            Box::new(NamedEchoTool {
                tool_name: "dup".into(),
            }),
            None,
        );
        // Second registration replaces the first; count is still 1.
        assert_eq!(kit.tool_count(), 1);
        // Tool is still callable.
        kit.call_tool("dup", serde_json::json!({})).await.unwrap();
    }

    #[tokio::test]
    async fn test_call_nonexistent_tool() {
        let kit = Toolkit::new();
        let err = kit
            .call_tool("does_not_exist", serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::NotFound(_)));
        assert!(err.to_string().contains("does_not_exist"));
    }

    #[tokio::test]
    async fn test_middleware_before_error_short_circuits() {
        let called = Arc::new(Mutex::new(false));
        let mut kit = Toolkit::new();
        kit.add_middleware(Box::new(FailBeforeMiddleware));
        kit.register(
            Box::new(TrackingTool {
                called: called.clone(),
            }),
            None,
        );

        let err = kit
            .call_tool("tracking", serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::ExecutionFailed(_)));
        // Handler must NOT have been called.
        assert!(!*called.lock().unwrap());
    }

    #[tokio::test]
    async fn test_middleware_after_modifies_response() {
        let mut kit = Toolkit::new();
        kit.register(Box::new(AddTool), None);
        kit.add_middleware(Box::new(ModifyAfterMiddleware));

        let resp = kit
            .call_tool("add", serde_json::json!({"a": 1.0, "b": 2.0}))
            .await
            .unwrap();

        if let ContentBlock::Text(tb) = &resp.content[0] {
            assert_eq!(tb.text, "modified_by_after");
        } else {
            panic!("expected TextBlock");
        }
    }

    #[tokio::test]
    async fn test_multiple_middlewares_chain() {
        let before_log = Arc::new(Mutex::new(Vec::<String>::new()));
        let after_log = Arc::new(Mutex::new(Vec::<String>::new()));

        let mut kit = Toolkit::new();
        for label in ["mw1", "mw2", "mw3"] {
            kit.add_middleware(Box::new(LabeledMiddleware {
                label: label.to_string(),
                before_log: before_log.clone(),
                after_log: after_log.clone(),
            }));
        }
        kit.register(Box::new(EchoTool), None);

        kit.call_tool("echo", serde_json::json!({})).await.unwrap();

        assert_eq!(*before_log.lock().unwrap(), vec!["mw1", "mw2", "mw3"]);
        assert_eq!(*after_log.lock().unwrap(), vec!["mw1", "mw2", "mw3"]);
    }

    #[tokio::test]
    async fn test_disabled_group_tool_rejected() {
        let mut kit = Toolkit::new();
        kit.register(Box::new(EchoTool), Some("optional"));

        // Deactivate the group.
        kit.set_group_active("optional", false);

        let err = kit
            .call_tool("echo", serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::PermissionDenied(_)));
        assert!(err.to_string().contains("inactive group"));
    }

    #[tokio::test]
    async fn test_basic_group_cannot_disable() {
        let mut kit = Toolkit::new();
        kit.register(Box::new(AddTool), None); // "basic" group

        // Attempt to disable basic group.
        kit.set_group_active("basic", false);

        // "basic" group must remain active.
        let group = kit.groups.get("basic").unwrap();
        assert!(group.active);

        // Tool still callable.
        kit.call_tool("add", serde_json::json!({"a": 1.0, "b": 1.0}))
            .await
            .unwrap();

        // Definition still present.
        assert_eq!(kit.get_definitions().len(), 1);
    }

    #[tokio::test]
    async fn test_empty_toolkit_definitions() {
        let kit = Toolkit::new();
        let defs = kit.get_definitions();
        assert!(defs.is_empty());
    }

    #[tokio::test]
    async fn test_preset_kwargs_merge() {
        let mut kit = Toolkit::new();
        kit.register(Box::new(EchoTool), None);

        // Set preset args.
        kit.tools.get_mut("echo").unwrap().preset_args =
            serde_json::json!({"preset_a": "from_preset", "shared": "preset_wins_not"});

        // Caller provides "shared" and an extra key — caller wins on conflict.
        let resp = kit
            .call_tool(
                "echo",
                serde_json::json!({"shared": "caller_wins", "caller_b": 42}),
            )
            .await
            .unwrap();

        if let ContentBlock::Text(tb) = &resp.content[0] {
            let v: Value = serde_json::from_str(&tb.text).unwrap();
            assert_eq!(v["preset_a"], "from_preset");
            assert_eq!(v["shared"], "caller_wins");
            assert_eq!(v["caller_b"], 42);
        } else {
            panic!("expected TextBlock");
        }
    }
}
