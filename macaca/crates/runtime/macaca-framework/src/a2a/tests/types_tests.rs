//! Serde round-trip contract tests for A2A wire-protocol value objects.

use crate::a2a::{
    A2AMessage, A2APart, A2ARole, A2ATask, A2ATaskState, A2ATaskStatus, AgentCapabilities,
    AgentCard, AgentSkillInfo, SendMessageRequest,
};

use super::fixtures::sample_card;

#[test]
fn test_agent_card_serialization() {
    let card = sample_card();
    let json = serde_json::to_string(&card).unwrap();
    let back: AgentCard = serde_json::from_str(&json).unwrap();
    assert_eq!(back.name, card.name);
    assert_eq!(back.url, card.url);
    assert_eq!(back.version, card.version);
    assert_eq!(back.description, card.description);
    assert_eq!(back.capabilities.streaming, true);
    assert_eq!(back.capabilities.push_notifications, false);
    assert_eq!(back.default_input_modes, card.default_input_modes);
    assert_eq!(back.skills.len(), 1);
    assert_eq!(back.skills[0].name, "echo");
}

#[test]
fn test_a2a_message_serialization() {
    let msg = A2AMessage {
        message_id: "msg-001".into(),
        role: A2ARole::User,
        parts: vec![
            A2APart::Text {
                text: "Hello!".into(),
            },
            A2APart::Data {
                data: serde_json::json!({"key": "value"}),
            },
        ],
        context_id: Some("ctx-1".into()),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let back: A2AMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(back.message_id, "msg-001");
    assert_eq!(back.role, A2ARole::User);
    assert_eq!(back.parts.len(), 2);
    assert_eq!(back.context_id, Some("ctx-1".into()));
}

#[test]
fn test_a2a_task_states() {
    let states = [
        A2ATaskState::Submitted,
        A2ATaskState::Working,
        A2ATaskState::Completed,
        A2ATaskState::Failed,
        A2ATaskState::Canceled,
    ];
    let expected_strs = ["submitted", "working", "completed", "failed", "canceled"];
    for (state, expected) in states.iter().zip(expected_strs.iter()) {
        let json = serde_json::to_string(state).unwrap();
        assert_eq!(json, format!("\"{}\"", expected));
        let back: A2ATaskState = serde_json::from_str(&json).unwrap();
        assert_eq!(back, *state);
    }

    let task = A2ATask {
        id: "task-1".into(),
        context_id: None,
        status: A2ATaskStatus {
            state: A2ATaskState::Working,
            message: None,
        },
        artifacts: vec![],
    };
    let json = serde_json::to_string(&task).unwrap();
    let back: A2ATask = serde_json::from_str(&json).unwrap();
    assert_eq!(back.id, "task-1");
    assert_eq!(back.status.state, A2ATaskState::Working);
}

#[test]
fn test_send_message_request() {
    let req = SendMessageRequest {
        id: "req-001".into(),
        message: A2AMessage {
            message_id: "msg-002".into(),
            role: A2ARole::User,
            parts: vec![A2APart::Text {
                text: "Do something".into(),
            }],
            context_id: None,
        },
    };
    let json = serde_json::to_string(&req).unwrap();
    let back: SendMessageRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(back.id, "req-001");
    assert_eq!(back.message.message_id, "msg-002");
    assert_eq!(back.message.role, A2ARole::User);
    match &back.message.parts[0] {
        A2APart::Text { text } => assert_eq!(text, "Do something"),
        other => panic!("Expected text, got {:?}", other),
    }
}

#[test]
fn test_agent_card_serde() {
    let card = AgentCard {
        name: "multi-skill".into(),
        url: "https://example.com/a2a".into(),
        version: "2.1.0".into(),
        description: Some("An agent with many skills".into()),
        capabilities: AgentCapabilities {
            streaming: true,
            push_notifications: true,
            state_transition_history: true,
        },
        default_input_modes: vec!["text/plain".into(), "application/json".into()],
        default_output_modes: vec!["text/plain".into()],
        skills: vec![
            AgentSkillInfo {
                name: "search".into(),
                description: Some("Search the web".into()),
            },
            AgentSkillInfo {
                name: "compute".into(),
                description: None,
            },
        ],
    };
    let json = serde_json::to_string(&card).unwrap();
    let back: AgentCard = serde_json::from_str(&json).unwrap();

    assert_eq!(back.name, "multi-skill");
    assert_eq!(back.version, "2.1.0");
    assert_eq!(back.description, Some("An agent with many skills".into()));
    assert!(back.capabilities.push_notifications);
    assert!(back.capabilities.state_transition_history);
    assert_eq!(back.default_input_modes.len(), 2);
    assert_eq!(back.skills.len(), 2);
    assert_eq!(back.skills[1].name, "compute");
    assert!(back.skills[1].description.is_none());
}

#[test]
fn test_a2a_task_state_variants() {
    let all_states = vec![
        A2ATaskState::Submitted,
        A2ATaskState::Working,
        A2ATaskState::Completed,
        A2ATaskState::Failed,
        A2ATaskState::Canceled,
    ];
    for state in &all_states {
        let json = serde_json::to_value(state).unwrap();
        let back: A2ATaskState = serde_json::from_value(json).unwrap();
        assert_eq!(&back, state);
    }
}
