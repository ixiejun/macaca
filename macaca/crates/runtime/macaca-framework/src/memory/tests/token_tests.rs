//! Contract tests for token estimation helpers.

use crate::message::Msg;

use super::super::tokens::{estimate_messages_tokens, estimate_tokens};

// Token estimation tests
    // =======================================================================

    // -----------------------------------------------------------------------
    // 26. test_estimate_tokens
    // -----------------------------------------------------------------------
#[test]
fn test_estimate_tokens() {
    // Empty string → 0 tokens
    assert_eq!(estimate_tokens(""), 0);
    // 8 chars → (8*3+7)/8 = 31/8 = 3
    assert_eq!(estimate_tokens("12345678"), 3);
    // 1 char → (1*3+7)/8 = 10/8 = 1
    assert_eq!(estimate_tokens("a"), 1);
    // 16 chars → (16*3+7)/8 = 55/8 = 6
    assert_eq!(estimate_tokens("1234567890123456"), 6);
}

// -----------------------------------------------------------------------
// 27. test_estimate_messages_tokens
// -----------------------------------------------------------------------
#[test]
fn test_estimate_messages_tokens() {
    let msgs = vec![
        Msg::user("alice", "hello"), // 5 chars → (15+7)/8 = 2
        Msg::user("bob", "world"),   // 5 chars → 2
    ];
    let total = estimate_messages_tokens(&msgs);
    assert_eq!(total, 4);

    // Empty list
    assert_eq!(estimate_messages_tokens(&[]), 0);
}

// =======================================================================
