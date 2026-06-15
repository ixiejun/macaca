use std::sync::Arc;

use macaca_proto::config::TelegramConfig;
use macaca_proto::types::GatewayEvent;

use crate::adapter::{EventHandler, ImAdapter};
use crate::telegram::TelegramAdapter;
use crate::telegram_format::split_message;

fn test_config() -> TelegramConfig {
    TelegramConfig {
        enabled: true,
        bot_token_env: "TEST_TELEGRAM_TOKEN".into(),
        allowed_user_ids: vec!["user1".into(), "user2".into()],
    }
}

#[test]
fn telegram_adapter_name() {
    let adapter = TelegramAdapter::new(test_config());
    assert_eq!(ImAdapter::name(&adapter), "telegram");
}

#[test]
fn telegram_adapter_config_access() {
    let cfg = test_config();
    let adapter = TelegramAdapter::new(cfg.clone());
    assert_eq!(adapter.config().bot_token_env, "TEST_TELEGRAM_TOKEN");
    assert_eq!(adapter.config().allowed_user_ids.len(), 2);
}

#[test]
fn test_parse_task_request() {
    let event = TelegramAdapter::parse_message("build me a web app", "42", "100");
    match event {
        GatewayEvent::TaskRequest {
            user_id,
            channel_id,
            content,
        } => {
            assert_eq!(user_id, "42");
            assert_eq!(channel_id, "100");
            assert_eq!(content, "build me a web app");
        }
        other => panic!("expected TaskRequest, got {other:?}"),
    }
}

#[test]
fn test_parse_status_command_no_id() {
    let event = TelegramAdapter::parse_message("/status", "1", "2");
    match event {
        GatewayEvent::StatusQuery {
            user_id,
            channel_id,
            task_id,
        } => {
            assert_eq!(user_id, "1");
            assert_eq!(channel_id, "2");
            assert!(task_id.is_none());
        }
        other => panic!("expected StatusQuery, got {other:?}"),
    }
}

#[test]
fn test_parse_status_command_with_valid_uuid() {
    let id = "550e8400-e29b-41d4-a716-446655440000";
    let msg = format!("/status {id}");
    let event = TelegramAdapter::parse_message(&msg, "1", "2");
    match event {
        GatewayEvent::StatusQuery { task_id, .. } => {
            assert!(task_id.is_some());
        }
        other => panic!("expected StatusQuery, got {other:?}"),
    }
}

#[test]
fn test_parse_status_command_with_invalid_id() {
    let event = TelegramAdapter::parse_message("/status abc123", "1", "2");
    match event {
        GatewayEvent::StatusQuery { task_id, .. } => {
            assert!(task_id.is_none());
        }
        other => panic!("expected StatusQuery, got {other:?}"),
    }
}

#[test]
fn test_parse_generic_command() {
    let event = TelegramAdapter::parse_message("/agents list all", "7", "9");
    match event {
        GatewayEvent::Command {
            user_id,
            channel_id,
            command,
            args,
        } => {
            assert_eq!(user_id, "7");
            assert_eq!(channel_id, "9");
            assert_eq!(command, "agents");
            assert_eq!(args, vec!["list", "all"]);
        }
        other => panic!("expected Command, got {other:?}"),
    }
}

#[test]
fn test_parse_command_with_bot_suffix() {
    let event = TelegramAdapter::parse_message("/help@MyBot", "1", "2");
    match event {
        GatewayEvent::Command { command, args, .. } => {
            assert_eq!(command, "help");
            assert!(args.is_empty());
        }
        other => panic!("expected Command, got {other:?}"),
    }
}

#[test]
fn test_parse_trims_whitespace() {
    let event = TelegramAdapter::parse_message("  hello world  ", "1", "2");
    match event {
        GatewayEvent::TaskRequest { content, .. } => {
            assert_eq!(content, "hello world");
        }
        other => panic!("expected TaskRequest, got {other:?}"),
    }
}

#[test]
fn test_split_message_short() {
    let chunks = split_message("hello", 100);
    assert_eq!(chunks, vec!["hello"]);
}

#[test]
fn test_split_message_exact_limit() {
    let text = "a".repeat(100);
    let chunks = split_message(&text, 100);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].len(), 100);
}

#[test]
fn test_split_message_long_no_newline() {
    let text = "x".repeat(200);
    let chunks = split_message(&text, 100);
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].len(), 100);
    assert_eq!(chunks[1].len(), 100);
}

#[test]
fn test_split_message_splits_at_newline() {
    let text = format!("{}\n{}", "a".repeat(60), "b".repeat(60));
    let chunks = split_message(&text, 100);
    assert_eq!(chunks.len(), 2);
    assert!(chunks[0].ends_with('\n'));
    assert!(chunks[1].starts_with('b'));
}

#[test]
fn test_split_message_empty() {
    let chunks = split_message("", 100);
    assert_eq!(chunks, vec![""]);
}

#[test]
fn test_split_message_three_chunks() {
    let text = "z".repeat(300);
    let chunks = split_message(&text, 100);
    assert_eq!(chunks.len(), 3);
    for c in &chunks {
        assert_eq!(c.len(), 100);
    }
}

#[tokio::test]
async fn telegram_adapter_start_without_token_is_ok() {
    std::env::remove_var("TEST_TELEGRAM_TOKEN");
    let adapter = TelegramAdapter::new(test_config());
    let handler: Arc<dyn EventHandler> = Arc::new(crate::gateway::DefaultEventHandler);
    adapter.start(handler).await.unwrap();
}

#[tokio::test]
async fn telegram_adapter_stop_is_ok() {
    let adapter = TelegramAdapter::new(test_config());
    ImAdapter::stop(&adapter).await.unwrap();
}

#[tokio::test]
async fn telegram_adapter_send_without_token_is_ok() {
    std::env::remove_var("TEST_TELEGRAM_TOKEN");
    let adapter = TelegramAdapter::new(test_config());
    adapter
        .send_message("chat_123", "Hello from test")
        .await
        .unwrap();
}
