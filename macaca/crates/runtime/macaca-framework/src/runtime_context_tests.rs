// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

use super::*;
use crate::message::Msg;

#[test]
fn runtime_context_keeps_transient_extras_separate() {
    let context = RuntimeContext::new("trace-1")
        .with_tenant_id("tenant")
        .with_application_id("app")
        .with_session_id("session")
        .with_extra("request", serde_json::json!({"transient": true}))
        .unwrap();

    let state = AgentState::empty();

    assert_eq!(context.trace_id, "trace-1");
    assert_eq!(context.extras.len(), 1);
    assert!(state.custom_namespaces.is_empty());
    assert!(state.conversation.is_empty());
}

#[test]
fn session_key_rejects_empty_segments() {
    let error = SessionKey::new("tenant", "", "session", "agent").unwrap_err();
    assert!(matches!(error, AgentSessionError::InvalidKey { .. }));
}

#[tokio::test]
async fn session_store_preserves_tenant_application_isolation() {
    let store = InMemoryAgentSessionStore::new();
    let key_a = SessionKey::new("tenant-a", "app", "session", "agent").unwrap();
    let key_b = SessionKey::new("tenant-b", "app", "session", "agent").unwrap();
    let mut state = AgentState::empty();
    state.push_message(Msg::user("user", "hello"));

    store.save_state(&key_a, state.clone()).await.unwrap();

    assert!(store.load_state(&key_a).await.unwrap().is_some());
    assert!(store.load_state(&key_b).await.unwrap().is_none());
    assert_eq!(store.snapshot().stored_keys, vec![key_a.storage_key()]);
}

#[tokio::test]
async fn durable_state_roundtrips_permission_and_custom_namespace() {
    let store = InMemoryAgentSessionStore::new();
    let key = SessionKey::new("tenant", "app", "session", "agent").unwrap();
    let mut state = AgentState::empty();
    state.permission = PermissionState::WaitingForUserConfirm {
        request_id: "confirm-1".to_string(),
        tool_call_id: Some("tool-1".to_string()),
        prompt: "approve operation".to_string(),
    };
    state
        .put_custom_namespace("provider-neutral", serde_json::json!({"value": 1}))
        .unwrap();

    store.save_state(&key, state).await.unwrap();
    let restored = store.load_state(&key).await.unwrap().unwrap();

    assert!(matches!(
        restored.permission,
        PermissionState::WaitingForUserConfirm { .. }
    ));
    assert_eq!(
        restored.custom_namespaces["provider-neutral"],
        serde_json::json!({"value": 1})
    );
}
