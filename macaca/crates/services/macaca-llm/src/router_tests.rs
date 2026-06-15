use super::*;
use async_trait::async_trait;
use macaca_proto::types::TokenUsage;

struct EchoProvider {
    name: String,
}

#[async_trait]
impl LlmProvider for EchoProvider {
    fn name(&self) -> &str {
        &self.name
    }

    async fn chat(
        &self,
        messages: Vec<LlmMessage>,
        options: &LlmOptions,
    ) -> MacacaResult<LlmResponse> {
        let content = messages
            .last()
            .map(|m| m.content.clone())
            .unwrap_or_default();
        Ok(LlmResponse {
            content,
            reasoning_content: None,
            model: options.model.clone(),
            usage: TokenUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            },
            finish_reason: "stop".into(),
            tool_calls: None,
        })
    }
}

#[tokio::test]
async fn router_dispatches_to_registered_provider() {
    let mut router = LlmRouter::new();
    router.register(
        "openai",
        Arc::new(EchoProvider {
            name: "openai".into(),
        }),
    );

    let messages = vec![LlmMessage::user("hello")];
    let options = LlmOptions {
        model: "gpt-4o".into(),
        ..Default::default()
    };

    let resp = router.chat(messages, &options).await.unwrap();
    assert_eq!(resp.content, "hello");
}

#[tokio::test]
async fn router_returns_error_for_missing_provider() {
    let router = LlmRouter::new();
    let messages = vec![LlmMessage::user("hi")];
    let options = LlmOptions {
        model: "gpt-4".into(),
        ..Default::default()
    };
    assert!(router.chat(messages, &options).await.is_err());
}

#[test]
fn resolve_selection_prefers_agent_over_app_and_system() {
    let mut router = LlmRouter::new();
    router.set_default_provider("openai");
    router.set_provider_default_model("openai", "gpt-4o");
    router.register(
        "openai",
        Arc::new(EchoProvider {
            name: "openai".into(),
        }),
    );
    router.register(
        "anthropic",
        Arc::new(EchoProvider {
            name: "anthropic".into(),
        }),
    );

    let selection = router
        .resolve_selection(&ModelSelectionRequest {
            agent_model: Some("anthropic:claude-sonnet-4".into()),
            app_model: Some("gpt-4.1".into()),
            app_provider: Some("openai".into()),
            system_model: Some("openai:gpt-4o".into()),
            ..Default::default()
        })
        .unwrap();

    assert_eq!(
        selection.primary,
        ModelTarget {
            provider: "anthropic".into(),
            model: "claude-sonnet-4".into(),
        }
    );
    assert_eq!(selection.source, "agent");
}

#[test]
fn resolve_selection_uses_app_provider_hint() {
    let mut router = LlmRouter::new();
    router.set_default_provider("openai");
    router.set_provider_default_model("openai", "gpt-4o");
    router.set_provider_default_model("anthropic", "claude-sonnet-4");
    router.register(
        "openai",
        Arc::new(EchoProvider {
            name: "openai".into(),
        }),
    );
    router.register(
        "anthropic",
        Arc::new(EchoProvider {
            name: "anthropic".into(),
        }),
    );

    let selection = router
        .resolve_selection(&ModelSelectionRequest {
            app_model: Some("claude-3-5-sonnet-20241022".into()),
            app_provider: Some("anthropic".into()),
            ..Default::default()
        })
        .unwrap();

    assert_eq!(selection.primary.provider, "anthropic");
    assert_eq!(selection.primary.model, "claude-3-5-sonnet-20241022");
}

#[tokio::test]
async fn chat_with_selection_uses_fallback_target() {
    struct FailingProvider;

    #[async_trait]
    impl LlmProvider for FailingProvider {
        fn name(&self) -> &str {
            "fail"
        }

        async fn chat(
            &self,
            _messages: Vec<LlmMessage>,
            _options: &LlmOptions,
        ) -> MacacaResult<LlmResponse> {
            Err(MacacaError::Llm("boom".into()))
        }
    }

    let mut router = LlmRouter::new();
    router.register("openai", Arc::new(FailingProvider));
    router.register(
        "anthropic",
        Arc::new(EchoProvider {
            name: "anthropic".into(),
        }),
    );

    let selection = ModelSelection {
        primary: ModelTarget {
            provider: "openai".into(),
            model: "gpt-4o".into(),
        },
        fallbacks: vec![ModelTarget {
            provider: "anthropic".into(),
            model: "claude-sonnet-4".into(),
        }],
        source: "test",
    };

    let response = router
        .chat_with_selection(
            vec![LlmMessage::user("hello")],
            &LlmOptions {
                model: "gpt-4o".into(),
                ..Default::default()
            },
            &selection,
        )
        .await
        .unwrap();

    assert_eq!(response.model, "claude-sonnet-4");
    assert_eq!(response.content, "hello");
}
