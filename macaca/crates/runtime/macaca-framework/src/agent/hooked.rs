//! [`HookedAgent`] — Decorator that injects hooks around an inner agent.
//!
//! **Design pattern:** Decorator — transparent `Agent` wrapper; hook chains run
//! before/after delegating to the wrapped implementation.

use async_trait::async_trait;
use macaca_proto::AgentId;
use tracing::debug;

use crate::message::Msg;

use super::core::Agent;
use super::error::AgentResult;
use super::hooks::HookRegistry;
use super::static_hooks::{
    run_static_post_observe, run_static_post_reply, run_static_pre_observe,
    run_static_pre_reply,
};

/// Transparent wrapper injecting pre/post hooks around an inner [`Agent`].
pub struct HookedAgent<A: Agent> {
    inner: A,
    hooks: HookRegistry,
}

impl<A: Agent> HookedAgent<A> {
    /// Wrap an agent with a hook registry.
    pub fn new(inner: A, hooks: HookRegistry) -> Self {
        Self { inner, hooks }
    }

    /// Wrap an agent with an empty registry (hooks can be added later).
    pub fn wrap(inner: A) -> Self {
        Self::new(inner, HookRegistry::new())
    }

    /// Mutable access to the per-instance hook registry.
    pub fn hooks_mut(&mut self) -> &mut HookRegistry {
        &mut self.hooks
    }

    /// Immutable reference to the inner agent.
    pub fn inner(&self) -> &A {
        &self.inner
    }
}

#[async_trait]
impl<A: Agent> Agent for HookedAgent<A> {
    async fn reply(&self, msg: Msg) -> AgentResult<Msg> {
        debug!(
            target: "macaca_framework::agent",
            agent = self.inner.name(),
            msg_id = %msg.id,
            "hooked reply start"
        );

        let msg = self.hooks.run_pre_reply(msg).await?;
        let msg = run_static_pre_reply(msg).await?;

        let reply = self.inner.reply(msg).await?;

        let reply = run_static_post_reply(reply).await?;
        let reply = self.hooks.run_post_reply(reply).await?;

        debug!(
            target: "macaca_framework::agent",
            agent = self.inner.name(),
            reply_id = %reply.id,
            "hooked reply complete"
        );
        Ok(reply)
    }

    async fn observe(&self, msg: Msg) -> AgentResult<()> {
        debug!(
            target: "macaca_framework::agent",
            agent = self.inner.name(),
            msg_id = %msg.id,
            "hooked observe start"
        );

        let msg = self.hooks.run_pre_observe(msg).await?;
        let msg = run_static_pre_observe(msg).await?;

        self.inner.observe(msg).await?;

        run_static_post_observe().await?;
        self.hooks.run_post_observe().await?;

        debug!(
            target: "macaca_framework::agent",
            agent = self.inner.name(),
            "hooked observe complete"
        );
        Ok(())
    }

    async fn interrupt(&self, msg: Msg) -> AgentResult<()> {
        self.inner.interrupt(msg).await
    }

    fn name(&self) -> &str {
        self.inner.name()
    }

    fn id(&self) -> &AgentId {
        self.inner.id()
    }
}
