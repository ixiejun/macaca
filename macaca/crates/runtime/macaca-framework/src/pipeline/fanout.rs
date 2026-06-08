//! [`FanoutPipeline`] — broadcast one message to many agents (Strategy + Composite).
//!
//! Supports parallel (`join_all`) or sequential fan-out. The `run` entry point
//! returns the first successful reply; `run_all` exposes every individual result
//! for callers that need full aggregation or custom selection logic.

use std::sync::Arc;

use async_trait::async_trait;
use futures::future::join_all;
use tracing::debug;

use crate::agent::{Agent, AgentError, AgentResult};
use crate::message::Msg;

use super::core::Pipeline;

/// Broadcast the same message to multiple agents and return the first success.
///
/// Each agent receives an independent clone of the input message.
/// - `concurrent = true`: all agents run in parallel via `join_all`
/// - `concurrent = false`: agents run in sequence, each receiving a fresh clone
///
/// Use [`FanoutPipeline::run_all`] to retrieve every agent's individual result.
///
/// **Design pattern:** Strategy — `concurrent` flag selects parallel vs sequential
/// execution without changing the caller-facing `Pipeline` interface.
pub struct FanoutPipeline {
    agents: Vec<Arc<dyn Agent>>,
    /// When true, agents execute concurrently; otherwise sequentially.
    concurrent: bool,
}

impl FanoutPipeline {
    /// Create a fanout pipeline.
    ///
    /// `concurrent` controls whether agents execute in parallel (`true`) or
    /// in order (`false`).
    pub fn new(agents: Vec<Arc<dyn Agent>>, concurrent: bool) -> Self {
        debug!(
            target = "macaca_framework::pipeline::fanout",
            agent_count = agents.len(),
            concurrent,
            "FanoutPipeline constructed"
        );
        Self { agents, concurrent }
    }

    /// Run all agents and return every individual result.
    ///
    /// Clones the input message per agent so side effects in one branch do not
    /// leak into siblings. Errors are collected per-agent rather than short-circuited.
    pub async fn run_all(&self, msg: Msg) -> Vec<AgentResult<Msg>> {
        debug!(
            target = "macaca_framework::pipeline::fanout",
            agent_count = self.agents.len(),
            concurrent = self.concurrent,
            "FanoutPipeline::run_all start"
        );

        let results = if self.concurrent {
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
        };

        debug!(
            target = "macaca_framework::pipeline::fanout",
            result_count = results.len(),
            "FanoutPipeline::run_all complete"
        );
        results
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
                Ok(m) => {
                    debug!(
                        target = "macaca_framework::pipeline::fanout",
                        output_sender = %m.name,
                        "FanoutPipeline::run first success"
                    );
                    return Ok(m);
                }
                Err(e) => last_err = Some(e),
            }
        }
        Err(last_err.unwrap_or_else(|| {
            AgentError::Other("FanoutPipeline has no agents".into())
        }))
    }
}
