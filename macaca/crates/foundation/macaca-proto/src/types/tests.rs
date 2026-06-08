//! Unit tests for domain type invariants, serde round-trips, and visitor dispatch.

use super::*;
use chrono::Utc;
    use serde_json::json;

    struct RecordingVisitor;

    impl AgentExecutionEventVisitor<String> for RecordingVisitor {
        fn thinking(&mut self, iteration: usize, content: Option<&str>) -> String {
            format!("thinking:{iteration}:{}", content.unwrap_or(""))
        }

        fn tool_call(
            &mut self,
            tool_name: &str,
            tool_input: &serde_json::Value,
            call_id: Option<&str>,
        ) -> String {
            format!(
                "tool_call:{tool_name}:{tool_input}:{}",
                call_id.unwrap_or("")
            )
        }

        fn tool_result(&mut self, tool_name: &str, output: &str, is_error: Option<bool>) -> String {
            format!(
                "tool_result:{tool_name}:{output}:{}",
                is_error.unwrap_or(false)
            )
        }

        fn assistant(&mut self, content: &str) -> String {
            format!("assistant:{content}")
        }

        fn driver_trace(&mut self, driver_name: &str, trace: &serde_json::Value) -> String {
            format!("driver_trace:{driver_name}:{trace}")
        }

        fn completed(&mut self, success: bool, error: Option<&str>) -> String {
            format!("completed:{success}:{}", error.unwrap_or(""))
        }
    }

    #[test]
    fn agent_id_is_unique() {
        let a = AgentId::new();
        let b = AgentId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn application_id_is_unique() {
        let a = ApplicationId::new();
        let b = ApplicationId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn driver_id_is_unique() {
        let a = DriverId::new();
        let b = DriverId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn application_id_display() {
        let id = ApplicationId::new();
        let s = format!("{}", id);
        assert!(!s.is_empty());
    }

    #[test]
    fn task_request_new() {
        let req = TaskRequest::new("build a web app");
        assert_eq!(req.description, "build a web app");
        assert_eq!(req.priority, TaskPriority::Normal);
    }

    #[test]
    fn agent_execution_event_visitor_dispatches_correctly() {
        let event = AgentExecutionEvent::ToolCall {
            tool_name: "file_read".into(),
            tool_input: json!({"path":"/tmp/a.txt"}),
            call_id: Some("call-1".into()),
        };
        let mut visitor = RecordingVisitor;
        let result = event.accept(&mut visitor);
        assert_eq!(
            result,
            r#"tool_call:file_read:{"path":"/tmp/a.txt"}:call-1"#
        );
    }

    #[test]
    fn agent_execution_event_serde_shape_is_unchanged() {
        let event = AgentExecutionEvent::Thinking {
            iteration: 2,
            content: Some("planning".into()),
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "thinking");
        assert_eq!(json["iteration"], 2);
        assert_eq!(json["content"], "planning");
    }

    #[test]
    fn types_serialize_roundtrip() {
        let entry = MemoryEntry {
            id: MemoryId::new(),
            layer: MemoryLayer::Vector,
            content: "test memory".into(),
            metadata: serde_json::json!({"key": "value"}),
            agent_id: Some(AgentId::new()),
            created_at: Utc::now(),
            expires_at: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: MemoryEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.content, "test memory");
        assert_eq!(parsed.layer, MemoryLayer::Vector);
    }

    #[test]
    fn gateway_event_serialize() {
        let event = GatewayEvent::TaskRequest {
            user_id: "u123".into(),
            channel_id: "c456".into(),
            content: "build me an app".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("TaskRequest"));
    }

    #[test]
    fn task_priority_ordering() {
        assert!(TaskPriority::Critical > TaskPriority::High);
        assert!(TaskPriority::High > TaskPriority::Normal);
        assert!(TaskPriority::Normal > TaskPriority::Low);
    }

    // ── LLM Message constructors ──

    #[test]
    fn llm_message_user_constructor() {
        let msg = LlmMessage::user("hello");
        assert_eq!(msg.role, LlmRole::User);
        assert_eq!(msg.content, "hello");
        assert!(msg.tool_calls.is_none());
        assert!(msg.tool_call_id.is_none());
    }

    #[test]
    fn llm_message_system_constructor() {
        let msg = LlmMessage::system("you are helpful");
        assert_eq!(msg.role, LlmRole::System);
        assert_eq!(msg.content, "you are helpful");
    }

    #[test]
    fn llm_message_tool_result_constructor() {
        let msg = LlmMessage::tool_result("call_123", r#"{"result": "ok"}"#);
        assert_eq!(msg.role, LlmRole::Tool);
        assert_eq!(msg.tool_call_id.as_deref(), Some("call_123"));
    }

    #[test]
    fn llm_message_assistant_with_tool_calls() {
        let calls = vec![ToolCall {
            id: "call_1".into(),
            name: "file_read".into(),
            arguments: serde_json::json!({"path": "/tmp/test.txt"}),
        }];
        let msg = LlmMessage::assistant_with_tool_calls("", calls);
        assert_eq!(msg.role, LlmRole::Assistant);
        assert_eq!(msg.tool_calls.as_ref().unwrap().len(), 1);
        assert_eq!(msg.tool_calls.as_ref().unwrap()[0].name, "file_read");
    }

    // ── Tool calling types serialization ──

    #[test]
    fn tool_call_serialize_roundtrip() {
        let tc = ToolCall {
            id: "call_abc".into(),
            name: "shell".into(),
            arguments: serde_json::json!({"command": "ls"}),
        };
        let json = serde_json::to_string(&tc).unwrap();
        let parsed: ToolCall = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "call_abc");
        assert_eq!(parsed.name, "shell");
    }

    #[test]
    fn tool_definition_serialize_roundtrip() {
        let td = ToolDefinition {
            name: "file_read".into(),
            description: "Read a file".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"}
                },
                "required": ["path"]
            }),
        };
        let json = serde_json::to_string(&td).unwrap();
        let parsed: ToolDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "file_read");
    }

    #[test]
    fn llm_message_backward_compatible_deserialization() {
        // Old format without tool_calls/tool_call_id should still parse
        let json = r#"{"role":"User","content":"hello"}"#;
        let msg: LlmMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.role, LlmRole::User);
        assert_eq!(msg.content, "hello");
        assert!(msg.tool_calls.is_none());
        assert!(msg.tool_call_id.is_none());
    }

    #[test]
    fn llm_response_with_tool_calls() {
        let resp = LlmResponse {
            content: String::new(),
            reasoning_content: None,
            model: "model-alpha".into(),
            usage: TokenUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            },
            finish_reason: "tool_calls".into(),
            tool_calls: Some(vec![ToolCall {
                id: "call_1".into(),
                name: "shell".into(),
                arguments: serde_json::json!({"command": "echo hi"}),
            }]),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: LlmResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.finish_reason, "tool_calls");
        assert_eq!(parsed.tool_calls.unwrap().len(), 1);
    }

    #[test]
    fn llm_response_backward_compatible_deserialization() {
        // Old format without tool_calls should still parse
        let json = r#"{"content":"hi","model":"model-alpha","usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2},"finish_reason":"stop"}"#;
        let resp: LlmResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.content, "hi");
        assert!(resp.tool_calls.is_none());
    }

    #[test]
    fn todo_item_sequence_number_default() {
        let item = TodoItem::new(
            ApplicationId::new(),
            None,
            "agent1",
            "plan",
            "Test task",
            "desc",
            5,
        );
        assert_eq!(item.sequence_number, 0);
    }

    #[test]
    fn todo_item_serialize_with_sequence_number() {
        let mut item = TodoItem::new(
            ApplicationId::new(),
            None,
            "agent1",
            "plan",
            "Test",
            "desc",
            5,
        );
        item.sequence_number = 3;
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"sequence_number\":3"));
        let parsed: TodoItem = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.sequence_number, 3);
    }

    #[test]
    fn todo_item_deserialize_without_sequence_number() {
        // Simulate legacy data without sequence_number field
        let json = serde_json::json!({
            "id": TaskId::new(),
            "application_id": ApplicationId::new(),
            "assigned_agent": "agent1",
            "created_by": "plan",
            "title": "Legacy task",
            "description": "desc",
            "acceptance_criteria": [],
            "status": "Pending",
            "priority": 5,
            "created_at": "2025-01-01T00:00:00Z",
            "updated_at": "2025-01-01T00:00:00Z",
            "depends_on": [],
            "progress_notes": [],
            "attempt_count": 0,
            "max_attempts": 3
        });
        let item: TodoItem = serde_json::from_value(json).unwrap();
        assert_eq!(item.sequence_number, 0); // default
    }

    #[test]
    fn agent_task_ref_serialize_roundtrip() {
        let all = AgentTaskRef::AllTasks {
            agent: "agent-alpha".into(),
        };
        let json = serde_json::to_string(&all).unwrap();
        let parsed: AgentTaskRef = serde_json::from_str(&json).unwrap();
        match parsed {
            AgentTaskRef::AllTasks { agent } => assert_eq!(agent, "agent-alpha"),
            _ => panic!("Expected AllTasks"),
        }

        let specific = AgentTaskRef::SpecificTask {
            agent: "agent-beta".into(),
            title: "Sample Task Title".into(),
        };
        let json = serde_json::to_string(&specific).unwrap();
        let parsed: AgentTaskRef = serde_json::from_str(&json).unwrap();
        match parsed {
            AgentTaskRef::SpecificTask { agent, title } => {
                assert_eq!(agent, "agent-beta");
                assert_eq!(title, "Sample Task Title");
            }
            _ => panic!("Expected SpecificTask"),
        }
    }

    #[test]
    fn llm_options_with_tools() {
        let opts = LlmOptions {
            tools: Some(vec![ToolDefinition {
                name: "test".into(),
                description: "test tool".into(),
                parameters: serde_json::json!({}),
            }]),
            ..Default::default()
        };
        assert_eq!(opts.tools.as_ref().unwrap().len(), 1);
    }
