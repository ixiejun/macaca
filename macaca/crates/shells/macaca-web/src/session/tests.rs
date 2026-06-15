//! Unit tests for session turn deduplication and trace serialization.

/// Provider-neutral fixture agent id for trace serialization tests (Object Mother).
const FIXTURE_TRACE_AGENT_ALPHA: &str = "trace-agent-alpha";

use super::trace_mapping::delegated_driver_trace_step;
use super::turn_model::ensure_running_assistant_turn;
use super::types::{AgentTrace, StoredTurn};

fn make_turn(role: &str, content: &str, status: Option<&str>) -> StoredTurn {
    StoredTurn {
        role: role.into(),
        content: content.into(),
        status: status.map(String::from),
        trace_steps: Vec::new(),
        meta: None,
        agent_traces: std::collections::HashMap::new(),
    }
}

fn make_turn_with_traces(
    role: &str,
    content: &str,
    status: Option<&str>,
    agents: Vec<&str>,
) -> StoredTurn {
    let mut agent_traces = std::collections::HashMap::new();
    for agent in agents {
        agent_traces.insert(
            agent.to_string(),
            vec![AgentTrace {
                task_id: format!("task-{agent}"),
                agent: agent.to_string(),
                status: "completed".to_string(),
                steps: vec![],
                output: Some("done".to_string()),
                error: None,
            }],
        );
    }
    StoredTurn {
        role: role.into(),
        content: content.into(),
        status: status.map(String::from),
        trace_steps: Vec::new(),
        meta: None,
        agent_traces,
    }
}

#[test]
fn test_dedup_removes_snapshot_running_turn() {
    // Simulate: snapshot saved [user, assistant(running)], then final save appends new pair
    let prompt = "hello".to_string();
    let mut turns = vec![
        make_turn("user", "hello", None),
        make_turn_with_traces(
            "assistant",
            "partial...",
            Some("running"),
            vec![FIXTURE_TRACE_AGENT_ALPHA],
        ),
    ];

    // Apply the dedup logic (same as in the success path)
    if let Some(pos) = turns.iter().rposition(|t| {
        t.role == "assistant" && matches!(t.status.as_deref(), Some("running") | Some("pending"))
    }) {
        turns.remove(pos);
        if pos > 0 && turns[pos - 1].role == "user" && turns[pos - 1].content == prompt {
            turns.remove(pos - 1);
        }
    }

    // After dedup, should be empty (both snapshot turns removed)
    assert!(
        turns.is_empty(),
        "snapshot running turn and its user turn should be removed"
    );

    // Now push final turns
    turns.push(make_turn("user", "hello", None));
    turns.push(make_turn_with_traces(
        "assistant",
        "final answer",
        Some("completed"),
        vec![FIXTURE_TRACE_AGENT_ALPHA],
    ));

    assert_eq!(turns.len(), 2);
    assert_eq!(turns[0].role, "user");
    assert_eq!(turns[1].role, "assistant");
    assert_eq!(turns[1].status.as_deref(), Some("completed"));
    assert!(!turns[1].agent_traces.is_empty());
}

#[test]
fn test_dedup_preserves_prior_conversation_turns() {
    // Simulate: prior completed turns + snapshot running turn
    let prompt = "second question".to_string();
    let mut turns = vec![
        make_turn("user", "first question", None),
        make_turn_with_traces(
            "assistant",
            "first answer",
            Some("completed"),
            vec![FIXTURE_TRACE_AGENT_ALPHA],
        ),
        make_turn("user", "second question", None),
        make_turn_with_traces("assistant", "partial...", Some("running"), vec!["tester"]),
    ];

    if let Some(pos) = turns.iter().rposition(|t| {
        t.role == "assistant" && matches!(t.status.as_deref(), Some("running") | Some("pending"))
    }) {
        turns.remove(pos);
        if pos > 0 && turns[pos - 1].role == "user" && turns[pos - 1].content == prompt {
            turns.remove(pos - 1);
        }
    }

    // Should keep the first completed pair, remove the snapshot pair
    assert_eq!(turns.len(), 2);
    assert_eq!(turns[0].content, "first question");
    assert_eq!(turns[1].content, "first answer");
    assert_eq!(turns[1].status.as_deref(), Some("completed"));
}

#[test]
fn test_dedup_noop_when_no_running_turn() {
    let prompt = "hello".to_string();
    let mut turns = vec![
        make_turn("user", "hello", None),
        make_turn_with_traces(
            "assistant",
            "done",
            Some("completed"),
            vec![FIXTURE_TRACE_AGENT_ALPHA],
        ),
    ];

    if let Some(pos) = turns.iter().rposition(|t| {
        t.role == "assistant" && matches!(t.status.as_deref(), Some("running") | Some("pending"))
    }) {
        turns.remove(pos);
        if pos > 0 && turns[pos - 1].role == "user" && turns[pos - 1].content == prompt {
            turns.remove(pos - 1);
        }
    }

    // No running turn, so nothing removed
    assert_eq!(turns.len(), 2);
    assert_eq!(turns[1].status.as_deref(), Some("completed"));
}

#[test]
fn test_dedup_handles_pending_status() {
    let prompt = "test".to_string();
    let mut turns = vec![
        make_turn("user", "test", None),
        make_turn("assistant", "thinking...", Some("pending")),
    ];

    if let Some(pos) = turns.iter().rposition(|t| {
        t.role == "assistant" && matches!(t.status.as_deref(), Some("running") | Some("pending"))
    }) {
        turns.remove(pos);
        if pos > 0 && turns[pos - 1].role == "user" && turns[pos - 1].content == prompt {
            turns.remove(pos - 1);
        }
    }

    assert!(turns.is_empty(), "pending turn should also be removed");
}

#[test]
fn test_agent_trace_serialization_roundtrip() {
    let turn = make_turn_with_traces(
        "assistant",
        "answer",
        Some("completed"),
        vec![FIXTURE_TRACE_AGENT_ALPHA, "tester"],
    );
    let json = serde_json::to_string(&turn).unwrap();
    let deserialized: StoredTurn = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.agent_traces.len(), 2);
    assert!(deserialized
        .agent_traces
        .contains_key(FIXTURE_TRACE_AGENT_ALPHA));
    assert!(deserialized.agent_traces.contains_key("tester"));
    assert_eq!(
        deserialized.agent_traces[FIXTURE_TRACE_AGENT_ALPHA][0].task_id,
        format!("task-{FIXTURE_TRACE_AGENT_ALPHA}")
    );
    assert_eq!(deserialized.agent_traces["tester"][0].status, "completed");
}

#[test]
fn test_agent_trace_empty_skipped_in_json() {
    let turn = make_turn("assistant", "answer", Some("completed"));
    let json = serde_json::to_string(&turn).unwrap();

    // agent_traces with skip_serializing_if should not appear in JSON when empty
    assert!(
        !json.contains("agent_traces"),
        "empty agent_traces should be skipped in JSON"
    );
}

#[test]
fn test_delegated_driver_trace_step_handles_direct_trace_payload() {
    let payload = serde_json::json!({
        "driver_name": "opencode",
        "event": {
            "type": "bash",
            "driver_id": "opencode",
            "tool_name": "bash",
            "tool_input": { "cmd": "ls -la" },
            "tool_output": "ok",
            "title": "Bash"
        }
    });

    let step = delegated_driver_trace_step(&payload);

    assert_eq!(step.step_type, "driver_trace");
    assert_eq!(step.event_type.as_deref(), Some("bash"));
    assert_eq!(step.driver_name.as_deref(), Some("opencode"));
    assert_eq!(step.driver_id.as_deref(), Some("opencode"));
    assert_eq!(step.tool_name.as_deref(), Some("bash"));
    assert_eq!(step.tool_output.as_deref(), Some("ok"));
    assert_eq!(step.title.as_deref(), Some("Bash"));
}

#[test]
fn test_delegated_driver_trace_step_unwraps_nested_driver_trace_payload() {
    // Neutral fixture ids — driver names are application config, not OS literals.
    const DRIVER_FIXTURE_ID: &str = "delegated-external-driver";
    let payload = serde_json::json!({
        "event": {
            "type": "driver_trace",
            "driver_name": DRIVER_FIXTURE_ID,
            "trace": {
                "type": "thinking",
                "driver_id": DRIVER_FIXTURE_ID,
                "content": "planning next action"
            }
        }
    });

    let step = delegated_driver_trace_step(&payload);

    assert_eq!(step.step_type, "driver_trace");
    assert_eq!(step.event_type.as_deref(), Some("thinking"));
    assert_eq!(step.driver_name.as_deref(), Some(DRIVER_FIXTURE_ID));
    assert_eq!(step.driver_id.as_deref(), Some(DRIVER_FIXTURE_ID));
    assert_eq!(step.content.as_deref(), Some("planning next action"));
}

#[test]
fn test_ensure_running_assistant_turn_creates_new() {
    let mut turns = vec![make_turn("user", "hi", None)];
    let turn = ensure_running_assistant_turn(&mut turns);
    assert_eq!(turn.role, "assistant");
    assert_eq!(turn.status.as_deref(), Some("running"));
    assert_eq!(turns.len(), 2);
}

#[test]
fn test_ensure_running_assistant_turn_reuses_existing() {
    let mut turns = vec![
        make_turn("user", "hi", None),
        make_turn("assistant", "partial", Some("running")),
    ];
    let turn = ensure_running_assistant_turn(&mut turns);
    assert_eq!(turn.content, "partial");
    assert_eq!(turns.len(), 2); // no new turn added
}
