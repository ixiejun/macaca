//! Pipeline orchestration: Sequential, Fanout, and MsgHub.
//!
//! Pipelines compose multiple agents into structured communication patterns:
//! - `SequentialPipeline` — chain agents so each output feeds the next input
//! - `FanoutPipeline` — broadcast one message to multiple agents in parallel or sequence
//! - `MsgHub` — multi-agent round-table where each reply is broadcast to all others

use std::sync::Arc;

use async_trait::async_trait;
use futures::future::join_all;

use crate::agent::{Agent, AgentResult};
use crate::message::Msg;

// ---------------------------------------------------------------------------
// Pipeline trait
// ---------------------------------------------------------------------------

/// A composable unit that transforms a `Msg` into another `Msg`.
#[async_trait]
pub trait Pipeline: Send + Sync {
    /// Run the pipeline on the given message and return the result.
    async fn run(&self, msg: Msg) -> AgentResult<Msg>;
}

// ---------------------------------------------------------------------------
// SequentialPipeline
// ---------------------------------------------------------------------------

/// Execute agents in series: each agent's output becomes the next agent's input.
///
/// An empty agent list returns the original message unchanged.
pub struct SequentialPipeline {
    agents: Vec<Arc<dyn Agent>>,
}

impl SequentialPipeline {
    /// Create a new sequential pipeline from a list of agents.
    pub fn new(agents: Vec<Arc<dyn Agent>>) -> Self {
        Self { agents }
    }
}

#[async_trait]
impl Pipeline for SequentialPipeline {
    async fn run(&self, msg: Msg) -> AgentResult<Msg> {
        let mut current = msg;
        for agent in &self.agents {
            current = agent.reply(current).await?;
        }
        Ok(current)
    }
}

// ---------------------------------------------------------------------------
// FanoutPipeline
// ---------------------------------------------------------------------------

/// Broadcast the same message to multiple agents and return the first success.
///
/// Each agent receives an independent clone of the input message.
/// - `concurrent = true`: all agents run in parallel via `join_all`
/// - `concurrent = false`: agents run in sequence, each receiving a fresh clone
///
/// Use `run_all` to retrieve every agent's individual result.
pub struct FanoutPipeline {
    agents: Vec<Arc<dyn Agent>>,
    concurrent: bool,
}

impl FanoutPipeline {
    /// Create a fanout pipeline.
    ///
    /// `concurrent` controls whether agents execute in parallel (`true`) or
    /// in order (`false`).
    pub fn new(agents: Vec<Arc<dyn Agent>>, concurrent: bool) -> Self {
        Self { agents, concurrent }
    }

    /// Run all agents and return every individual result.
    pub async fn run_all(&self, msg: Msg) -> Vec<AgentResult<Msg>> {
        if self.concurrent {
            let futures: Vec<_> = self
                .agents
                .iter()
                .map(|a| {
                    let a = Arc::clone(a);
                    let m = msg.clone();
                    async move { a.reply(m).await }
                })
                .collect();
            join_all(futures).await
        } else {
            let mut results = Vec::with_capacity(self.agents.len());
            for agent in &self.agents {
                results.push(agent.reply(msg.clone()).await);
            }
            results
        }
    }
}

#[async_trait]
impl Pipeline for FanoutPipeline {
    /// Run all agents and return the first successful result.
    ///
    /// Returns the last error if every agent fails.
    async fn run(&self, msg: Msg) -> AgentResult<Msg> {
        let results = self.run_all(msg).await;
        let mut last_err = None;
        for r in results {
            match r {
                Ok(m) => return Ok(m),
                Err(e) => last_err = Some(e),
            }
        }
        Err(last_err.unwrap_or_else(|| {
            crate::agent::AgentError::Other("FanoutPipeline has no agents".into())
        }))
    }
}

// ---------------------------------------------------------------------------
// MsgHub
// ---------------------------------------------------------------------------

/// Multi-agent round-table: each participant's reply is broadcast to all others.
///
/// In each round:
/// 1. The initial message is observed by every participant.
/// 2. Each participant replies in order.
/// 3. Each reply (with `ThinkingBlock`s stripped) is observed by all *other*
///    participants before the next agent takes its turn.
///
/// Implements `Pipeline` by running a single round and returning the last reply.
pub struct MsgHub {
    participants: Vec<Arc<dyn Agent>>,
}

impl MsgHub {
    /// Create a hub with the given participants.
    pub fn new(participants: Vec<Arc<dyn Agent>>) -> Self {
        Self { participants }
    }

    /// Run one round: broadcast initial message, then let each agent reply in
    /// order while broadcasting each reply to all other participants.
    ///
    /// Returns all replies produced in this round.
    pub async fn run_round(&self, initial_msg: Msg) -> AgentResult<Vec<Msg>> {
        // Broadcast the initial message to every participant.
        for participant in &self.participants {
            participant.observe(initial_msg.clone()).await?;
        }

        let mut replies = Vec::with_capacity(self.participants.len());

        for (i, agent) in self.participants.iter().enumerate() {
            let reply = agent.reply(initial_msg.clone()).await?;

            // Strip thinking before broadcasting to preserve privacy.
            let broadcast = reply.stripped_for_broadcast();

            // Broadcast this reply to every *other* participant.
            for (j, other) in self.participants.iter().enumerate() {
                if i != j {
                    other.observe(broadcast.clone()).await?;
                }
            }

            replies.push(reply);
        }

        Ok(replies)
    }

    /// Run `rounds` consecutive rounds.
    ///
    /// Each round starts with `initial_msg`; replies from earlier rounds are
    /// delivered via `observe` and accumulate in the returned list.
    pub async fn run_rounds(&self, initial_msg: Msg, rounds: usize) -> AgentResult<Vec<Msg>> {
        let mut all_replies = Vec::new();
        for _ in 0..rounds {
            let mut round_replies = self.run_round(initial_msg.clone()).await?;
            all_replies.append(&mut round_replies);
        }
        Ok(all_replies)
    }
}

#[async_trait]
impl Pipeline for MsgHub {
    /// Run one round and return the last participant's reply.
    async fn run(&self, msg: Msg) -> AgentResult<Msg> {
        let replies = self.run_round(msg).await?;
        replies
            .into_iter()
            .last()
            .ok_or_else(|| crate::agent::AgentError::Other("MsgHub has no participants".into()))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{ContentBlock, MsgContent, ThinkingBlock};
    use macaca_proto::AgentId;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    // ------------------------------------------------------------------
    // Test double: AppendAgent
    // ------------------------------------------------------------------

    struct AppendAgent {
        name: String,
        id: AgentId,
        suffix: String,
        observed: Arc<Mutex<Vec<String>>>,
    }

    impl AppendAgent {
        fn new(name: &str, suffix: &str) -> Self {
            Self {
                name: name.to_string(),
                id: AgentId::new(),
                suffix: suffix.to_string(),
                observed: Arc::new(Mutex::new(Vec::new())),
            }
        }

        async fn observed_texts(&self) -> Vec<String> {
            self.observed.lock().await.clone()
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

        async fn observe(&self, msg: Msg) -> AgentResult<()> {
            self.observed.lock().await.push(msg.get_text());
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
    // SequentialPipeline
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_sequential_pipeline() {
        let a: Arc<dyn Agent> = Arc::new(AppendAgent::new("A", "-A"));
        let b: Arc<dyn Agent> = Arc::new(AppendAgent::new("B", "-B"));
        let c: Arc<dyn Agent> = Arc::new(AppendAgent::new("C", "-C"));

        let pipeline = SequentialPipeline::new(vec![a, b, c]);
        let result = pipeline.run(Msg::user("user", "start")).await.unwrap();
        assert_eq!(result.get_text(), "start-A-B-C");
    }

    #[tokio::test]
    async fn test_sequential_empty() {
        let pipeline = SequentialPipeline::new(vec![]);
        let msg = Msg::user("user", "unchanged");
        let result = pipeline.run(msg).await.unwrap();
        assert_eq!(result.get_text(), "unchanged");
    }

    // ------------------------------------------------------------------
    // FanoutPipeline
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_fanout_concurrent() {
        let a = Arc::new(AppendAgent::new("A", "-A"));
        let b = Arc::new(AppendAgent::new("B", "-B"));

        let agents: Vec<Arc<dyn Agent>> = vec![Arc::clone(&a) as _, Arc::clone(&b) as _];
        let pipeline = FanoutPipeline::new(agents, true);

        // run_all should give every agent the same input text.
        let results = pipeline.run_all(Msg::user("user", "hello")).await;
        assert_eq!(results.len(), 2);

        let texts: Vec<String> = results.into_iter().map(|r| r.unwrap().get_text()).collect();
        assert!(texts.contains(&"hello-A".to_string()));
        assert!(texts.contains(&"hello-B".to_string()));
    }

    #[tokio::test]
    async fn test_fanout_sequential() {
        let a = Arc::new(AppendAgent::new("A", "-A"));
        let b = Arc::new(AppendAgent::new("B", "-B"));

        let agents: Vec<Arc<dyn Agent>> = vec![Arc::clone(&a) as _, Arc::clone(&b) as _];
        let pipeline = FanoutPipeline::new(agents, false);

        // run returns first success (agent A).
        let result = pipeline.run(Msg::user("user", "hi")).await.unwrap();
        assert_eq!(result.get_text(), "hi-A");

        // run_all returns both, each from a fresh clone.
        let results = pipeline.run_all(Msg::user("user", "hi")).await;
        assert_eq!(results.len(), 2);
        let t0 = results[0].as_ref().unwrap().get_text();
        let t1 = results[1].as_ref().unwrap().get_text();
        assert_eq!(t0, "hi-A");
        assert_eq!(t1, "hi-B");
    }

    // ------------------------------------------------------------------
    // MsgHub
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_msghub_round() {
        let a = Arc::new(AppendAgent::new("A", "-reply-A"));
        let b = Arc::new(AppendAgent::new("B", "-reply-B"));
        let c = Arc::new(AppendAgent::new("C", "-reply-C"));

        let hub = MsgHub::new(vec![
            Arc::clone(&a) as Arc<dyn Agent>,
            Arc::clone(&b) as Arc<dyn Agent>,
            Arc::clone(&c) as Arc<dyn Agent>,
        ]);

        let replies = hub.run_round(Msg::user("user", "init")).await.unwrap();
        assert_eq!(replies.len(), 3);

        // B should have observed: initial "init", then A's reply "init-reply-A".
        let b_obs = b.observed_texts().await;
        assert!(b_obs.contains(&"init".to_string()));
        assert!(b_obs.contains(&"init-reply-A".to_string()));

        // C should have observed: initial "init", A's reply, and B's reply.
        let c_obs = c.observed_texts().await;
        assert!(c_obs.contains(&"init".to_string()));
        assert!(c_obs.contains(&"init-reply-A".to_string()));
        assert!(c_obs.contains(&"init-reply-B".to_string()));

        // A should NOT have observed its own reply (only B's and C's).
        let a_obs = a.observed_texts().await;
        // A observed the initial message and later B's and C's replies
        // but NOT its own reply.
        assert!(!a_obs.contains(&"init-reply-A".to_string()));
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
