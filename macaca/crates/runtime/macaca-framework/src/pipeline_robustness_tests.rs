mod tests {
    use super::super::*;
    use crate::message::{ContentBlock, MsgContent, ThinkingBlock};
    use macaca_proto::AgentId;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    /// Test double that appends a deterministic suffix to each reply.
    ///
    /// This local double keeps the robustness tests self-contained after the
    /// file split. It mirrors the generic Agent contract only and does not
    /// introduce provider, model, or application-specific behavior.
    struct AppendAgent {
        name: String,
        id: AgentId,
        suffix: String,
    }

    impl AppendAgent {
        fn new(name: &str, suffix: &str) -> Self {
            Self {
                name: name.to_string(),
                id: AgentId::new(),
                suffix: suffix.to_string(),
            }
        }
    }

    #[async_trait]
    impl Agent for AppendAgent {
        async fn reply(&self, msg: Msg) -> AgentResult<Msg> {
            Ok(Msg::assistant(
                &self.name,
                format!("{}{}", msg.get_text(), self.suffix),
            ))
        }

        async fn observe(&self, _msg: Msg) -> AgentResult<()> {
            Ok(())
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn id(&self) -> &AgentId {
            &self.id
        }
    }

    // ------------------------------------------------------------------
    // ThinkingAgent & ObserverAgent for broadcast-strip tests
    // ------------------------------------------------------------------

    struct ThinkingAgent {
        name: String,
        id: AgentId,
        observed: Arc<Mutex<Vec<Msg>>>,
    }

    #[async_trait]
    impl Agent for ThinkingAgent {
        async fn reply(&self, _msg: Msg) -> AgentResult<Msg> {
            let blocks = vec![
                ContentBlock::Thinking(ThinkingBlock {
                    thinking: "secret thought".into(),
                }),
                ContentBlock::Text(crate::message::TextBlock {
                    text: "visible reply".into(),
                }),
            ];
            Ok(Msg::assistant(&self.name, MsgContent::Blocks(blocks)))
        }

        async fn observe(&self, msg: Msg) -> AgentResult<()> {
            self.observed.lock().await.push(msg);
            Ok(())
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn id(&self) -> &AgentId {
            &self.id
        }
    }

    struct ObserverAgent {
        name: String,
        id: AgentId,
        observed: Arc<Mutex<Vec<Msg>>>,
    }

    #[async_trait]
    impl Agent for ObserverAgent {
        async fn reply(&self, msg: Msg) -> AgentResult<Msg> {
            Ok(Msg::assistant(&self.name, msg.get_text()))
        }

        async fn observe(&self, msg: Msg) -> AgentResult<()> {
            self.observed.lock().await.push(msg);
            Ok(())
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn id(&self) -> &AgentId {
            &self.id
        }
    }

    #[tokio::test]
    async fn test_msghub_strips_thinking() {
        let thinker = Arc::new(ThinkingAgent {
            name: "thinker".into(),
            id: AgentId::new(),
            observed: Arc::new(Mutex::new(Vec::new())),
        });

        let observed_msgs: Arc<Mutex<Vec<Msg>>> = Arc::new(Mutex::new(Vec::new()));

        let observer = Arc::new(ObserverAgent {
            name: "observer".into(),
            id: AgentId::new(),
            observed: Arc::clone(&observed_msgs),
        });

        let hub = MsgHub::new(vec![
            Arc::clone(&thinker) as Arc<dyn Agent>,
            Arc::clone(&observer) as Arc<dyn Agent>,
        ]);

        hub.run_round(Msg::user("user", "go")).await.unwrap();

        // Observer should have received the thinker's reply with thinking stripped.
        let msgs = observed_msgs.lock().await;
        // Filter to msgs from "thinker"
        let thinker_broadcasts: Vec<&Msg> = msgs.iter().filter(|m| m.name == "thinker").collect();
        assert!(
            !thinker_broadcasts.is_empty(),
            "observer should see thinker's broadcast"
        );
        for broadcast in thinker_broadcasts {
            // Should not contain any ThinkingBlock.
            if let MsgContent::Blocks(blocks) = &broadcast.content {
                for block in blocks {
                    assert!(
                        !matches!(block, ContentBlock::Thinking(_)),
                        "ThinkingBlock should be stripped from broadcast"
                    );
                }
            }
            // Text content should still be visible.
            assert_eq!(broadcast.get_text(), "visible reply");
        }
    }

    // ------------------------------------------------------------------
    // Robustness tests
    // ------------------------------------------------------------------

    /// An agent that always fails.
    struct FailAgent {
        name: String,
        id: AgentId,
    }

    impl FailAgent {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                id: AgentId::new(),
            }
        }
    }

    #[async_trait]
    impl Agent for FailAgent {
        async fn reply(&self, _msg: Msg) -> AgentResult<Msg> {
            Err(crate::agent::AgentError::Other(format!(
                "{} failed",
                self.name
            )))
        }

        async fn observe(&self, _msg: Msg) -> AgentResult<()> {
            Ok(())
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn id(&self) -> &AgentId {
            &self.id
        }
    }

    /// An agent that can be configured to fail or succeed.
    struct ConditionalAgent {
        name: String,
        id: AgentId,
        should_fail: bool,
    }

    impl ConditionalAgent {
        fn new(name: &str, should_fail: bool) -> Self {
            Self {
                name: name.to_string(),
                id: AgentId::new(),
                should_fail,
            }
        }
    }

    #[async_trait]
    impl Agent for ConditionalAgent {
        async fn reply(&self, msg: Msg) -> AgentResult<Msg> {
            if self.should_fail {
                Err(crate::agent::AgentError::Other(format!(
                    "{} failed",
                    self.name
                )))
            } else {
                Ok(Msg::assistant(
                    &self.name,
                    format!("{}-{}", msg.get_text(), self.name),
                ))
            }
        }

        async fn observe(&self, _msg: Msg) -> AgentResult<()> {
            Ok(())
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn id(&self) -> &AgentId {
            &self.id
        }
    }

    /// Agent that fails on observe but succeeds on reply (for MsgHub error test).
    struct ObserveFailAgent {
        name: String,
        id: AgentId,
    }

    #[async_trait]
    impl Agent for ObserveFailAgent {
        async fn reply(&self, _msg: Msg) -> AgentResult<Msg> {
            Err(crate::agent::AgentError::Other(format!(
                "{} reply error",
                self.name
            )))
        }

        async fn observe(&self, _msg: Msg) -> AgentResult<()> {
            Ok(())
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn id(&self) -> &AgentId {
            &self.id
        }
    }

    #[tokio::test]
    async fn test_sequential_empty_pipeline() {
        let pipeline = SequentialPipeline::new(vec![]);
        let msg = Msg::user("user", "passthrough");
        let result = pipeline.run(msg).await.unwrap();
        assert_eq!(result.get_text(), "passthrough");
    }

    #[tokio::test]
    async fn test_sequential_single_agent() {
        let agent: Arc<dyn Agent> = Arc::new(AppendAgent::new("A", "-A"));

        // Direct call
        let direct = agent.reply(Msg::user("user", "hello")).await.unwrap();

        // Pipeline with single agent
        let pipeline = SequentialPipeline::new(vec![Arc::clone(&agent)]);
        let piped = pipeline.run(Msg::user("user", "hello")).await.unwrap();

        assert_eq!(direct.get_text(), piped.get_text());
    }

    #[tokio::test]
    async fn test_fanout_all_fail() {
        let agents: Vec<Arc<dyn Agent>> = vec![
            Arc::new(FailAgent::new("F1")),
            Arc::new(FailAgent::new("F2")),
            Arc::new(FailAgent::new("F3")),
        ];
        let pipeline = FanoutPipeline::new(agents, false);
        let result = pipeline.run(Msg::user("user", "test")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fanout_run_first_success() {
        // Agent 1 fails, agent 2 succeeds, agent 3 succeeds.
        let agents: Vec<Arc<dyn Agent>> = vec![
            Arc::new(ConditionalAgent::new("A1", true)),  // fail
            Arc::new(ConditionalAgent::new("A2", false)), // success
            Arc::new(ConditionalAgent::new("A3", false)), // success
        ];
        let pipeline = FanoutPipeline::new(agents, false);
        let result = pipeline.run(Msg::user("user", "hi")).await.unwrap();
        // First success is A2.
        assert_eq!(result.get_text(), "hi-A2");
    }

    #[tokio::test]
    async fn test_msghub_agent_error_handling() {
        // MsgHub propagates errors from run_round — if one agent's reply
        // fails, the round returns an error. Verify that the error comes
        // from the failing agent.
        let agents: Vec<Arc<dyn Agent>> = vec![
            Arc::new(AppendAgent::new("A", "-A")),
            Arc::new(ObserveFailAgent {
                name: "Failing".into(),
                id: AgentId::new(),
            }),
            Arc::new(AppendAgent::new("C", "-C")),
        ];
        let hub = MsgHub::new(agents);
        let result = hub.run_round(Msg::user("user", "init")).await;
        // The round should return an error since the second agent fails on reply.
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_msghub_multi_round() {
        let a = Arc::new(AppendAgent::new("A", "-rA"));
        let b = Arc::new(AppendAgent::new("B", "-rB"));
        let c = Arc::new(AppendAgent::new("C", "-rC"));

        let hub = MsgHub::new(vec![
            Arc::clone(&a) as Arc<dyn Agent>,
            Arc::clone(&b) as Arc<dyn Agent>,
            Arc::clone(&c) as Arc<dyn Agent>,
        ]);

        let replies = hub.run_rounds(Msg::user("user", "start"), 2).await.unwrap();
        // 3 agents × 2 rounds = 6 replies.
        assert_eq!(replies.len(), 6);

        // Each round produces 3 replies based on the initial_msg "start".
        assert_eq!(replies[0].get_text(), "start-rA");
        assert_eq!(replies[1].get_text(), "start-rB");
        assert_eq!(replies[2].get_text(), "start-rC");
        // Second round also uses initial_msg "start".
        assert_eq!(replies[3].get_text(), "start-rA");
    }

    #[tokio::test]
    async fn test_msghub_thinking_stripped() {
        let thinker = Arc::new(ThinkingAgent {
            name: "thinker".into(),
            id: AgentId::new(),
            observed: Arc::new(Mutex::new(Vec::new())),
        });

        let peer_observed: Arc<Mutex<Vec<Msg>>> = Arc::new(Mutex::new(Vec::new()));
        let peer = Arc::new(ObserverAgent {
            name: "peer".into(),
            id: AgentId::new(),
            observed: Arc::clone(&peer_observed),
        });

        let hub = MsgHub::new(vec![
            Arc::clone(&thinker) as Arc<dyn Agent>,
            Arc::clone(&peer) as Arc<dyn Agent>,
        ]);

        hub.run_round(Msg::user("user", "go")).await.unwrap();

        let msgs = peer_observed.lock().await;
        let from_thinker: Vec<&Msg> = msgs.iter().filter(|m| m.name == "thinker").collect();
        assert!(!from_thinker.is_empty());
        for msg in from_thinker {
            if let MsgContent::Blocks(blocks) = &msg.content {
                for block in blocks {
                    assert!(
                        !matches!(block, ContentBlock::Thinking(_)),
                        "ThinkingBlock must be stripped in broadcast"
                    );
                }
            }
            assert_eq!(msg.get_text(), "visible reply");
        }
    }
}
