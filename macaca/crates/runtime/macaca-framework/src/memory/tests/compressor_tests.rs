//! Contract tests for [`MemoryCompressor`] summarization pipeline.

use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::formatter::OpenAiFormatter;
use crate::message::{ContentBlock, Msg, TextBlock};
use crate::model::{ChatModel, ChatOptions, ChatResponse, ChatUsage, ModelError};

use super::super::compressor::MemoryCompressor;
use super::super::config::CompressionConfig;
use super::super::working::{InMemoryWorkingMemory, WorkingMemory};
use super::helpers::user_msg;

/// A mock model that returns pre-configured responses in order.
    struct MockChatModel {
        responses: tokio::sync::Mutex<Vec<ChatResponse>>,
        call_count: AtomicUsize,
    }

    impl MockChatModel {
        fn new(responses: Vec<ChatResponse>) -> Self {
            Self {
                responses: tokio::sync::Mutex::new(responses),
                call_count: AtomicUsize::new(0),
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

    fn text_response(text: &str) -> ChatResponse {
        ChatResponse {
            content: vec![ContentBlock::Text(TextBlock { text: text.into() })],
            id: "r".into(),
            created_at: String::new(),
            usage: ChatUsage::default(),
            metadata: None,
        }
    }

    // -----------------------------------------------------------------------
    // 28. test_compressor_below_threshold
    // -----------------------------------------------------------------------
    #[tokio::test]
    async fn test_compressor_below_threshold() {
        let config = CompressionConfig {
            trigger_threshold: 10000,
            target_tokens: 5000,
            keep_recent: 2,
            summary_model: None,
        };
        let compressor = MemoryCompressor::new(config);

        let mut mem = InMemoryWorkingMemory::new();
        // Add a few short messages — well below threshold
        mem.add(user_msg("hello"), vec![]).await;
        mem.add(user_msg("world"), vec![]).await;
        mem.add(user_msg("test"), vec![]).await;

        let model = MockChatModel::new(vec![]);
        let formatter = OpenAiFormatter;

        let result = compressor
            .compress_if_needed(&mut mem, &model, &formatter, "sys")
            .await
            .unwrap();

        assert!(!result); // No compression needed
        assert_eq!(mem.size().await, 3); // All messages still there
    }

    // -----------------------------------------------------------------------
    // 29. test_compressor_above_threshold
    // -----------------------------------------------------------------------
    #[tokio::test]
    async fn test_compressor_above_threshold() {
        let config = CompressionConfig {
            trigger_threshold: 10, // Very low threshold to trigger compression
            target_tokens: 5,
            keep_recent: 2,
            summary_model: None,
        };
        let compressor = MemoryCompressor::new(config);

        let mut mem = InMemoryWorkingMemory::new();
        // Add enough messages to trigger compression
        for i in 0..10 {
            mem.add(
                user_msg(&format!("This is a long message number {} with lots of text content to exceed threshold", i)),
                vec![],
            ).await;
        }

        let model = MockChatModel::new(vec![text_response("Summary of conversation.")]);
        let formatter = OpenAiFormatter;

        let result = compressor
            .compress_if_needed(&mut mem, &model, &formatter, "sys")
            .await
            .unwrap();

        assert!(result); // Compression was performed
                         // The compressed messages should be deleted, keeping only keep_recent=2
        assert_eq!(mem.size().await, 2);
        // Summary should be set
        let with_summary = mem.get_with_summary().await;
        assert_eq!(with_summary.len(), 3); // summary + 2 kept messages
        assert!(with_summary[0]
            .get_text()
            .contains("Summary of conversation."));
    }

    // -----------------------------------------------------------------------
    // 30. test_compressor_keep_recent
    // -----------------------------------------------------------------------
    #[tokio::test]
    async fn test_compressor_keep_recent() {
        let config = CompressionConfig {
            trigger_threshold: 5, // Very low
            target_tokens: 3,
            keep_recent: 3,
            summary_model: None,
        };
        let compressor = MemoryCompressor::new(config);

        let mut mem = InMemoryWorkingMemory::new();
        let mut kept_ids = Vec::new();
        for i in 0..8 {
            let m = user_msg(&format!(
                "Message {} with enough content to pass threshold easily",
                i
            ));
            if i >= 5 {
                kept_ids.push(m.id.clone());
            }
            mem.add(m, vec![]).await;
        }

        let model = MockChatModel::new(vec![text_response("Compressed summary.")]);
        let formatter = OpenAiFormatter;

        let result = compressor
            .compress_if_needed(&mut mem, &model, &formatter, "sys")
            .await
            .unwrap();

        assert!(result);
        // Should keep exactly keep_recent=3 messages
        assert_eq!(mem.size().await, 3);
        let remaining = mem.get_memory(None, None).await;
        for (i, msg) in remaining.iter().enumerate() {
            assert_eq!(msg.id, kept_ids[i]);
        }
    }
