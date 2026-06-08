//! Contract tests for [`LongTermMemory`] and [`InMemoryLongTermMemory`].

use crate::message::Msg;
use crate::state::StateModule;

use super::super::long_term::{InMemoryLongTermMemory, LongTermMemory};
use super::helpers::user_msg;

// LongTermMemory tests
    // =======================================================================

    // -----------------------------------------------------------------------
    // 20. test_ltm_record_and_retrieve
    // -----------------------------------------------------------------------
#[tokio::test]
async fn test_ltm_record_and_retrieve() {
    let mut ltm = InMemoryLongTermMemory::new();
    let msgs = vec![
        Msg::user("alice", "Rust programming language"),
        Msg::user("bob", "Python data science"),
    ];
    ltm.record(&msgs).await.unwrap();

    let results = ltm.retrieve("Rust", 10).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].get_text(), "Rust programming language");
}

// -----------------------------------------------------------------------
// 21. test_ltm_retrieve_empty
// -----------------------------------------------------------------------
#[tokio::test]
async fn test_ltm_retrieve_empty() {
    let ltm = InMemoryLongTermMemory::new();
    let results = ltm.retrieve("anything", 10).await.unwrap();
    assert!(results.is_empty());
}

// -----------------------------------------------------------------------
// 22. test_ltm_retrieve_limit
// -----------------------------------------------------------------------
#[tokio::test]
async fn test_ltm_retrieve_limit() {
    let mut ltm = InMemoryLongTermMemory::new();
    let msgs: Vec<Msg> = (0..10)
        .map(|i| Msg::user("user", format!("keyword msg-{}", i)))
        .collect();
    ltm.record(&msgs).await.unwrap();

    let results = ltm.retrieve("keyword", 3).await.unwrap();
    assert_eq!(results.len(), 3);
}

// -----------------------------------------------------------------------
// 23. test_ltm_state_roundtrip
// -----------------------------------------------------------------------
#[tokio::test]
async fn test_ltm_state_roundtrip() {
    let mut ltm = InMemoryLongTermMemory::new();
    let msgs = vec![
        Msg::user("alice", "remember this"),
        Msg::user("bob", "and this too"),
    ];
    ltm.record(&msgs).await.unwrap();

    let state = ltm.state_dict();

    let mut restored = InMemoryLongTermMemory::new();
    restored.load_state_dict(state).unwrap();

    let results = restored.retrieve("remember", 10).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].get_text(), "remember this");
}

// -----------------------------------------------------------------------
// 24. test_ltm_record_to_memory
// -----------------------------------------------------------------------
#[tokio::test]
async fn test_ltm_record_to_memory() {
    let mut ltm = InMemoryLongTermMemory::new();
    let content = vec![
        "First memory entry".to_string(),
        "Second memory entry".to_string(),
    ];
    ltm.record_to_memory(&content).await.unwrap();

    let results = ltm.retrieve("memory", 10).await.unwrap();
    assert_eq!(results.len(), 2);
}

// -----------------------------------------------------------------------
// 25. test_ltm_retrieve_from_memory
// -----------------------------------------------------------------------
#[tokio::test]
async fn test_ltm_retrieve_from_memory() {
    let mut ltm = InMemoryLongTermMemory::new();
    let msgs = vec![
        Msg::user("alice", "apple orange banana"),
        Msg::user("bob", "grape lemon"),
        Msg::user("carol", "apple grape"),
    ];
    ltm.record(&msgs).await.unwrap();

    let keywords = vec!["apple".to_string(), "grape".to_string()];
    let results = ltm.retrieve_from_memory(&keywords, 10).await.unwrap();
    // "apple grape" matches 2 keywords, "apple orange banana" matches 1, "grape lemon" matches 1
    assert_eq!(results.len(), 3);
    // First result should be the one with highest score (matches both keywords)
    assert_eq!(results[0].get_text(), "apple grape");
}

// =======================================================================
