//! Cross-module integration tests for macaca-framework.
//!
//! These tests exercise the public API across multiple modules:
//! ReActAgent + Toolkit + Memory, Session persistence, HookedAgent + Pipeline,
//! MsgHub multi-agent conversations, and PlanNotebook lifecycle.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use macaca_framework::agent::{Agent, AgentResult, Hook, HookRegistry, HookedAgent};
use macaca_framework::formatter::OpenAiFormatter;
use macaca_framework::memory::{InMemoryWorkingMemory, WorkingMemory};
use macaca_framework::message::{ContentBlock, Msg, Role, TextBlock, ToolUseBlock};
use macaca_framework::model::{ChatModel, ChatOptions, ChatResponse, ChatUsage, ModelError};
use macaca_framework::pipeline::{MsgHub, Pipeline, SequentialPipeline};
use macaca_framework::plan::{PlanNotebook, PlanState, SubTaskState};
use macaca_framework::react_agent::ReActAgent;
use macaca_framework::session::{self, InMemorySessionStore, SessionStore};
use macaca_framework::tool::{ToolError, ToolHandler, ToolResponse, Toolkit};
use macaca_proto::AgentId;
use tokio::sync::Mutex;

// ===========================================================================
// Mock components (re-defined here because unit-test mocks are private)
// ===========================================================================

/// A mock model that returns pre-configured responses in order.
struct MockChatModel {
    responses: Arc<Mutex<Vec<ChatResponse>>>,
    call_count: Arc<AtomicUsize>,
}

impl MockChatModel {
    fn new(responses: Vec<ChatResponse>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(responses)),
            call_count: Arc::new(AtomicUsize::new(0)),
        }
    }
}

#[async_trait]
impl ChatModel for MockChatModel {
    async fn chat(
        &self,
        _messages: Vec<serde_json::Value>,
        _options: &ChatOptions,
    ) -> Result<ChatResponse, ModelError> {
        let idx = self.call_count.fetch_add(1, Ordering::SeqCst);
        let responses = self.responses.lock().await;
        responses
            .get(idx)
            .cloned()
            .ok_or_else(|| ModelError::Other("no more responses".into()))
    }

    fn name(&self) -> &str {
        "mock"
    }
}

// ---------------------------------------------------------------------------
// Helper: build ChatResponse values
// ---------------------------------------------------------------------------

fn text_response(text: &str) -> ChatResponse {
    ChatResponse {
        content: vec![ContentBlock::Text(TextBlock { text: text.into() })],
        id: "r".into(),
        created_at: String::new(),
        usage: ChatUsage::default(),
        metadata: None,
    }
}

fn tool_call_response(tool_id: &str, tool_name: &str, input: serde_json::Value) -> ChatResponse {
    ChatResponse {
        content: vec![ContentBlock::ToolUse(ToolUseBlock {
            id: tool_id.into(),
            name: tool_name.into(),
            input,
            raw_input: None,
        })],
        id: "r".into(),
        created_at: String::new(),
        usage: ChatUsage::default(),
        metadata: None,
    }
}

// ---------------------------------------------------------------------------
// AddTool — real tool for integration testing
// ---------------------------------------------------------------------------

struct AddTool;

#[async_trait]
impl ToolHandler for AddTool {
    async fn execute(&self, args: serde_json::Value) -> Result<ToolResponse, ToolError> {
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
    fn schema(&self) -> serde_json::Value {
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

// ---------------------------------------------------------------------------
// MockAgent — simple agent for pipeline / hub tests
// ---------------------------------------------------------------------------

struct MockAgent {
    agent_name: String,
    agent_id: AgentId,
    prefix: String,
    observed: Arc<Mutex<Vec<Msg>>>,
}

impl MockAgent {
    fn new(name: &str, prefix: &str) -> Self {
        Self {
            agent_name: name.to_string(),
            agent_id: AgentId::new(),
            prefix: prefix.to_string(),
            observed: Arc::new(Mutex::new(Vec::new())),
        }
    }

    async fn observed_msgs(&self) -> Vec<Msg> {
        self.observed.lock().await.clone()
    }
}

#[async_trait]
impl Agent for MockAgent {
    async fn reply(&self, msg: Msg) -> AgentResult<Msg> {
        Ok(Msg::assistant(
            &self.agent_name,
            format!("{}{}", self.prefix, msg.get_text()),
        ))
    }

    async fn observe(&self, msg: Msg) -> AgentResult<()> {
        self.observed.lock().await.push(msg);
        Ok(())
    }

    fn name(&self) -> &str {
        &self.agent_name
    }

    fn id(&self) -> &AgentId {
        &self.agent_id
    }
}

// ===========================================================================
// Test 1: Full ReAct loop with tool call
// ===========================================================================

#[tokio::test]
async fn test_full_react_loop_with_tool_call() {
    // MockChatModel: 1st call → ToolUse(add, {a:3, b:4}), 2nd call → text "The sum is 7"
    let model = MockChatModel::new(vec![
        tool_call_response("call_add", "add", serde_json::json!({"a": 3.0, "b": 4.0})),
        text_response("The sum is 7"),
    ]);
    let call_count = model.call_count.clone();

    let mut toolkit = Toolkit::new();
    toolkit.register(Box::new(AddTool), None);

    let agent = ReActAgent::new(
        "calculator-bot",
        "You are a calculator assistant.",
        Arc::new(model),
        Arc::new(OpenAiFormatter),
    )
    .with_toolkit(toolkit)
    .with_max_iters(5);

    let reply = agent
        .reply(Msg::user("alice", "What is 3 + 4?"))
        .await
        .unwrap();

    // Final reply text
    assert_eq!(reply.get_text(), "The sum is 7");
    assert_eq!(reply.name, "calculator-bot");
    assert_eq!(reply.role, Role::Assistant);

    // LLM was called exactly twice
    assert_eq!(call_count.load(Ordering::SeqCst), 2);
}

// ===========================================================================
// Test 2: Session full lifecycle (save + restore working memory)
// ===========================================================================

#[tokio::test]
async fn test_session_full_lifecycle() {
    // 1) Create agent and have a conversation
    let model = MockChatModel::new(vec![text_response("Hello, I remember you!")]);

    let agent = ReActAgent::new(
        "session-bot",
        "You are helpful.",
        Arc::new(model),
        Arc::new(OpenAiFormatter),
    );

    let reply = agent.reply(Msg::user("alice", "Hi there")).await.unwrap();
    assert_eq!(reply.get_text(), "Hello, I remember you!");

    // 2) Create a standalone InMemoryWorkingMemory, populate, and save
    let mut memory = InMemoryWorkingMemory::new();
    memory.add(Msg::user("alice", "Hi there"), vec![]).await;
    memory
        .add(
            Msg::assistant("session-bot", "Hello, I remember you!"),
            vec![],
        )
        .await;

    let store = InMemorySessionStore::new();
    session::save_module_state(&store, "sess-123", &memory)
        .await
        .unwrap();

    // 3) Restore into a fresh memory instance
    let mut restored_memory = InMemoryWorkingMemory::new();
    session::load_module_state(&store, "sess-123", &mut restored_memory)
        .await
        .unwrap();

    // 4) Verify loaded memory contains the conversation
    let msgs = restored_memory.get_memory(None, None).await;
    assert_eq!(msgs.len(), 2);
    assert_eq!(msgs[0].get_text(), "Hi there");
    assert_eq!(msgs[0].role, Role::User);
    assert_eq!(msgs[1].get_text(), "Hello, I remember you!");
    assert_eq!(msgs[1].role, Role::Assistant);

    // 5) Verify session listing
    let sessions = store.list_sessions().await.unwrap();
    assert_eq!(sessions, vec!["sess-123"]);
}

// ===========================================================================
// Test 3: HookedAgent + SequentialPipeline
// ===========================================================================

/// Hook that appends " [hooked]" to the reply text.
struct AppendHookedHook;

#[async_trait]
impl Hook for AppendHookedHook {
    async fn post_reply(&self, msg: Msg) -> AgentResult<Msg> {
        let new_text = format!("{} [hooked]", msg.get_text());
        Ok(Msg::assistant(&msg.name, new_text))
    }
}

#[tokio::test]
async fn test_hooked_agent_in_pipeline() {
    // Agent A: prepends "A:"
    let agent_a: Arc<dyn Agent> = Arc::new(MockAgent::new("AgentA", "A:"));

    // Agent B: prepends "B:" — wrapped with a hook that appends " [hooked]"
    let inner_b = MockAgent::new("AgentB", "B:");
    let mut hooks = HookRegistry::new();
    hooks.register_instance_hook(Box::new(AppendHookedHook));
    let hooked_b: Arc<dyn Agent> = Arc::new(HookedAgent::new(inner_b, hooks));

    // Agent C: prepends "C:"
    let agent_c: Arc<dyn Agent> = Arc::new(MockAgent::new("AgentC", "C:"));

    // Sequential: A → B(hooked) → C
    let pipeline = SequentialPipeline::new(vec![agent_a, hooked_b, agent_c]);
    let result = pipeline.run(Msg::user("user", "start")).await.unwrap();

    // A receives "start" → outputs "A:start"
    // B receives "A:start" → outputs "B:A:start" → hook appends → "B:A:start [hooked]"
    // C receives "B:A:start [hooked]" → outputs "C:B:A:start [hooked]"
    assert_eq!(result.get_text(), "C:B:A:start [hooked]");
    assert_eq!(result.name, "AgentC");
}

// ===========================================================================
// Test 4: MsgHub multi-agent conversation
// ===========================================================================

#[tokio::test]
async fn test_msghub_multi_agent_conversation() {
    let alice = Arc::new(MockAgent::new("Alice", "[Alice says] "));
    let bob = Arc::new(MockAgent::new("Bob", "[Bob says] "));
    let charlie = Arc::new(MockAgent::new("Charlie", "[Charlie says] "));

    let hub = MsgHub::new(vec![
        Arc::clone(&alice) as Arc<dyn Agent>,
        Arc::clone(&bob) as Arc<dyn Agent>,
        Arc::clone(&charlie) as Arc<dyn Agent>,
    ]);

    let replies = hub
        .run_round(Msg::user("moderator", "Discuss topic X"))
        .await
        .unwrap();

    // 3 replies, one per agent
    assert_eq!(replies.len(), 3);
    assert_eq!(replies[0].name, "Alice");
    assert_eq!(replies[1].name, "Bob");
    assert_eq!(replies[2].name, "Charlie");

    // Each reply echoes the initial message with their prefix
    assert!(replies[0].get_text().contains("[Alice says]"));
    assert!(replies[1].get_text().contains("[Bob says]"));
    assert!(replies[2].get_text().contains("[Charlie says]"));

    // Bob observed: initial message + Alice's reply
    let bob_obs = bob.observed_msgs().await;
    let bob_texts: Vec<String> = bob_obs.iter().map(|m| m.get_text()).collect();
    assert!(bob_texts.contains(&"Discuss topic X".to_string()));
    assert!(bob_texts.iter().any(|t| t.contains("[Alice says]")));

    // Charlie observed: initial + Alice's reply + Bob's reply
    let charlie_obs = charlie.observed_msgs().await;
    let charlie_texts: Vec<String> = charlie_obs.iter().map(|m| m.get_text()).collect();
    assert!(charlie_texts.contains(&"Discuss topic X".to_string()));
    assert!(charlie_texts.iter().any(|t| t.contains("[Alice says]")));
    assert!(charlie_texts.iter().any(|t| t.contains("[Bob says]")));

    // Alice should NOT have observed her own reply
    let alice_obs = alice.observed_msgs().await;
    let alice_texts: Vec<String> = alice_obs.iter().map(|m| m.get_text()).collect();
    assert!(!alice_texts.iter().any(|t| t.contains("[Alice says]")));
    // But Alice DID observe Bob's and Charlie's replies
    assert!(alice_texts.iter().any(|t| t.contains("[Bob says]")));
    assert!(alice_texts.iter().any(|t| t.contains("[Charlie says]")));
}

// ===========================================================================
// Test 5: PlanNotebook lifecycle + SessionStore persistence
// ===========================================================================

#[tokio::test]
async fn test_plan_notebook_with_agent() {
    let mut notebook = PlanNotebook::new();

    // 1) Create a plan with 3 subtasks
    notebook.create_plan(
        "Build feature",
        "Implement the new search feature",
        "Search feature works end-to-end",
    );
    let plan = notebook.current_plan_mut().unwrap();
    plan.add_subtask("Design API", "Define REST endpoints", "API spec ready");
    plan.add_subtask(
        "Implement",
        "Write the code",
        "Code compiles and passes tests",
    );
    plan.add_subtask("Deploy", "Deploy to staging", "Service running on staging");

    assert_eq!(plan.state, PlanState::Todo);
    assert_eq!(plan.subtasks.len(), 3);

    // 2) Start first subtask → plan transitions to InProgress
    plan.start_subtask(0).unwrap();
    assert_eq!(plan.state, PlanState::InProgress);
    assert_eq!(plan.subtasks[0].state, SubTaskState::InProgress);

    // 3) Finish first subtask → auto-starts second
    plan.finish_subtask(0, "API spec created").unwrap();
    assert_eq!(plan.subtasks[0].state, SubTaskState::Done);
    assert_eq!(
        plan.subtasks[0].outcome.as_deref(),
        Some("API spec created")
    );
    assert_eq!(plan.subtasks[1].state, SubTaskState::InProgress); // auto-started

    // 4) Finish second → auto-starts third
    plan.finish_subtask(1, "Code complete").unwrap();
    assert_eq!(plan.subtasks[1].state, SubTaskState::Done);
    assert_eq!(plan.subtasks[2].state, SubTaskState::InProgress);

    // 5) Finish third → all done
    plan.finish_subtask(2, "Deployed to staging").unwrap();
    assert_eq!(plan.subtasks[2].state, SubTaskState::Done);
    assert!(plan.all_subtasks_finished());

    // 6) Finish the plan
    notebook.finish_plan("Feature shipped!").unwrap();
    assert!(notebook.current_plan().is_none());
    assert_eq!(notebook.historical_plans().len(), 1);
    assert_eq!(notebook.historical_plans()[0].state, PlanState::Done);
    assert_eq!(
        notebook.historical_plans()[0].outcome.as_deref(),
        Some("Feature shipped!")
    );

    // 7) Save to SessionStore and restore
    let store = InMemorySessionStore::new();
    session::save_module_state(&store, "plan-sess", &notebook)
        .await
        .unwrap();

    let mut restored = PlanNotebook::new();
    session::load_module_state(&store, "plan-sess", &mut restored)
        .await
        .unwrap();

    // Verify restored state
    assert!(restored.current_plan().is_none());
    assert_eq!(restored.historical_plans().len(), 1);
    let hist = &restored.historical_plans()[0];
    assert_eq!(hist.name, "Build feature");
    assert_eq!(hist.state, PlanState::Done);
    assert_eq!(hist.subtasks.len(), 3);
    assert!(hist.subtasks.iter().all(|s| s.state == SubTaskState::Done));
    assert_eq!(hist.outcome.as_deref(), Some("Feature shipped!"));
}
