use std::sync::Arc;

use tempfile::tempdir;

use crate::{AppendEventCommand, EventLog, EventLogQuery, RedbStore};

/// Provider-neutral event-log source label for fixture tests (not an agent role name).
const FIXTURE_EVENT_SOURCE_ENTRY: &str = "fixture-entry-source";
/// Provider-neutral indexed-query agent dimension for fixture tests.
const FIXTURE_AGENT_ALPHA: &str = "fixture-agent-alpha";
const FIXTURE_AGENT_BETA: &str = "fixture-agent-beta";

async fn test_log() -> EventLog {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.redb");
    let _dir = Box::leak(Box::new(dir));
    let store = Arc::new(RedbStore::open(db_path).unwrap());
    EventLog::new(store)
}

#[tokio::test]
async fn append_and_query() {
    let log = test_log().await;
    let s1 = log
        .append(
            "sess1",
            "thinking",
            FIXTURE_EVENT_SOURCE_ENTRY,
            serde_json::json!({"iteration": 1}),
        )
        .await;
    let s2 = log
        .append(
            "sess1",
            "tool_call",
            FIXTURE_EVENT_SOURCE_ENTRY,
            serde_json::json!({"tool": "list_agents"}),
        )
        .await;
    assert_eq!(s1, 1);
    assert_eq!(s2, 2);

    let events = log.query("sess1", 0, 100).await;
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event_type, "thinking");
    assert_eq!(events[1].event_type, "tool_call");
}

#[tokio::test]
async fn query_since_seq() {
    let log = test_log().await;
    log.append("sess1", "a", "src", serde_json::json!({})).await;
    log.append("sess1", "b", "src", serde_json::json!({})).await;
    log.append("sess1", "c", "src", serde_json::json!({})).await;

    let events = log.query("sess1", 1, 100).await;
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event_type, "b");
    assert_eq!(events[1].event_type, "c");
}

#[tokio::test]
async fn query_with_limit() {
    let log = test_log().await;
    for i in 0..10 {
        log.append("sess1", &format!("e{}", i), "src", serde_json::json!({}))
            .await;
    }
    let events = log.query("sess1", 0, 3).await;
    assert_eq!(events.len(), 3);
}

#[tokio::test]
async fn sessions_are_isolated() {
    let log = test_log().await;
    log.append("sess1", "a", "src", serde_json::json!({})).await;
    log.append("sess2", "b", "src", serde_json::json!({})).await;

    assert_eq!(log.query("sess1", 0, 100).await.len(), 1);
    assert_eq!(log.query("sess2", 0, 100).await.len(), 1);
}

#[tokio::test]
async fn latest_seq() {
    let log = test_log().await;
    assert_eq!(log.latest_seq("sess1").await, 0);
    log.append("sess1", "a", "src", serde_json::json!({})).await;
    log.append("sess1", "b", "src", serde_json::json!({})).await;
    assert_eq!(log.latest_seq("sess1").await, 2);
}

#[tokio::test]
async fn notify_on_append() {
    let log = test_log().await;
    let mut rx = log.subscribe();
    log.append("sess1", "a", "src", serde_json::json!({})).await;
    let (sid, seq) = rx.recv().await.unwrap();
    assert_eq!(sid, "sess1");
    assert_eq!(seq, 1);
}

#[tokio::test]
async fn replay_iterator_preserves_order_and_cursor() {
    let log = test_log().await;
    log.append("sess1", "a", "src", serde_json::json!({})).await;
    log.append("sess1", "b", "src", serde_json::json!({})).await;
    log.append("sess1", "c", "src", serde_json::json!({})).await;

    let events: Vec<_> = log.replay("sess1", 1, 10).await.collect();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].seq, 2);
    assert_eq!(events[0].event_type, "b");
    assert_eq!(events[1].seq, 3);
}

#[tokio::test]
async fn append_command_matches_stable_append_behavior() {
    let log = test_log().await;
    let seq = log
        .append_command(AppendEventCommand::new(
            "sess1",
            "thinking",
            FIXTURE_EVENT_SOURCE_ENTRY,
            serde_json::json!({"iteration": 1}),
        ))
        .await;
    assert_eq!(seq, 1);

    let events = log.query("sess1", 0, 10).await;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "thinking");
    assert_eq!(events[0].source, FIXTURE_EVENT_SOURCE_ENTRY);
    assert_eq!(events[0].payload["iteration"], 1);
}

#[tokio::test]
async fn indexed_query_filters_by_agent() {
    let log = test_log().await;
    log.append_command(AppendEventCommand::new(
        "sess1",
        "tool_call",
        "executor",
        serde_json::json!({"agent": FIXTURE_AGENT_ALPHA, "tool": "cargo"}),
    ))
    .await;
    log.append_command(AppendEventCommand::new(
        "sess1",
        "tool_call",
        "executor",
        serde_json::json!({"agent": FIXTURE_AGENT_BETA, "tool": "npm"}),
    ))
    .await;

    let events = log
        .query_indexed(
            EventLogQuery::new("sess1")
                .agent(Some(FIXTURE_AGENT_ALPHA.to_string()))
                .limit(10),
        )
        .await;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].payload["agent"], FIXTURE_AGENT_ALPHA);
}

#[tokio::test]
async fn indexed_query_filters_by_source_and_type() {
    let log = test_log().await;
    log.append(
        "sess1",
        "thinking",
        FIXTURE_EVENT_SOURCE_ENTRY,
        serde_json::json!({}),
    )
    .await;
    log.append("sess1", "run_trace", "plan_loop", serde_json::json!({}))
        .await;
    log.append("sess1", "run_trace", "executor_loop", serde_json::json!({}))
        .await;

    let source_events = log
        .query_indexed(
            EventLogQuery::new("sess1")
                .source(Some(FIXTURE_EVENT_SOURCE_ENTRY.to_string()))
                .limit(10),
        )
        .await;
    assert_eq!(source_events.len(), 1);
    assert_eq!(source_events[0].source, FIXTURE_EVENT_SOURCE_ENTRY);

    let trace_events = log
        .query_indexed(
            EventLogQuery::new("sess1")
                .event_type(Some("run_trace".to_string()))
                .limit(10),
        )
        .await;
    assert_eq!(trace_events.len(), 2);
    assert!(trace_events
        .iter()
        .all(|event| event.event_type == "run_trace"));
}

#[tokio::test]
async fn indexed_query_honors_since_and_limit() {
    let log = test_log().await;
    for i in 0..5 {
        log.append_command(AppendEventCommand::new(
            "sess1",
            "tool_call",
            "executor",
            serde_json::json!({"agent": FIXTURE_AGENT_ALPHA, "index": i}),
        ))
        .await;
    }

    let events = log
        .query_indexed(
            EventLogQuery::new("sess1")
                .since(2)
                .limit(2)
                .agent(Some(FIXTURE_AGENT_ALPHA.to_string())),
        )
        .await;
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].seq, 3);
    assert_eq!(events[1].seq, 4);
}
