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
#[path = "pipeline_tests.rs"]
mod pipeline_tests;

#[cfg(test)]
#[path = "pipeline_robustness_tests.rs"]
mod pipeline_robustness_tests;
