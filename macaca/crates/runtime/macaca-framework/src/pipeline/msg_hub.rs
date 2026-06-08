//! [`MsgHub`] — multi-agent round-table with observe/broadcast (Mediator + Observer).
//!
//! Each participant observes the initial message and every peer reply (with
//! thinking blocks stripped for privacy). Implements [`Pipeline`] by running
//! a single round and returning the last reply.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::debug;

use crate::agent::{Agent, AgentError, AgentResult};
use crate::message::Msg;

use super::core::Pipeline;

/// Multi-agent round-table: each participant's reply is broadcast to all others.
///
/// In each round:
/// 1. The initial message is observed by every participant.
/// 2. Each participant replies in order.
/// 3. Each reply (with `ThinkingBlock`s stripped) is observed by all *other*
///    participants before the next agent takes its turn.
///
/// **Design pattern:** Mediator — the hub coordinates peer-to-peer observation
/// without agents referencing each other directly. Broadcast sanitization acts
/// as a Decorator on outbound observe payloads.
///
/// Implements [`Pipeline`] by running a single round and returning the last reply.
pub struct MsgHub {
    participants: Vec<Arc<dyn Agent>>,
}

impl MsgHub {
    /// Create a hub with the given participants.
    pub fn new(participants: Vec<Arc<dyn Agent>>) -> Self {
        debug!(
            target = "macaca_framework::pipeline::msg_hub",
            participant_count = participants.len(),
            "MsgHub constructed"
        );
        Self { participants }
    }

    /// Run one round: broadcast initial message, then let each agent reply in
    /// order while broadcasting each reply to all other participants.
    ///
    /// Returns all replies produced in this round.
    pub async fn run_round(&self, initial_msg: Msg) -> AgentResult<Vec<Msg>> {
        debug!(
            target = "macaca_framework::pipeline::msg_hub",
            participant_count = self.participants.len(),
            initial_sender = %initial_msg.name,
            "MsgHub::run_round start"
        );

        // Broadcast the initial message to every participant.
        for participant in &self.participants {
            participant.observe(initial_msg.clone()).await?;
        }

        let mut replies = Vec::with_capacity(self.participants.len());

        for (i, agent) in self.participants.iter().enumerate() {
            debug!(
                target = "macaca_framework::pipeline::msg_hub",
                turn = i,
                agent_name = agent.name(),
                "MsgHub turn: agent reply"
            );

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

        debug!(
            target = "macaca_framework::pipeline::msg_hub",
            reply_count = replies.len(),
            "MsgHub::run_round complete"
        );
        Ok(replies)
    }

    /// Run `rounds` consecutive rounds.
    ///
    /// Each round starts with `initial_msg`; replies from earlier rounds are
    /// delivered via `observe` and accumulate in the returned list.
    pub async fn run_rounds(&self, initial_msg: Msg, rounds: usize) -> AgentResult<Vec<Msg>> {
        debug!(
            target = "macaca_framework::pipeline::msg_hub",
            rounds,
            "MsgHub::run_rounds start"
        );

        let mut all_replies = Vec::new();
        for round in 0..rounds {
            debug!(
                target = "macaca_framework::pipeline::msg_hub",
                round,
                "MsgHub::run_rounds iteration"
            );
            let mut round_replies = self.run_round(initial_msg.clone()).await?;
            all_replies.append(&mut round_replies);
        }

        debug!(
            target = "macaca_framework::pipeline::msg_hub",
            total_replies = all_replies.len(),
            "MsgHub::run_rounds complete"
        );
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
            .ok_or_else(|| AgentError::Other("MsgHub has no participants".into()))
    }
}
