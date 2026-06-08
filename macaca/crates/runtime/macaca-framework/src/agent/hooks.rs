//! Hook trait and per-instance [`HookRegistry`].
//!
//! **Design pattern:** Chain of Responsibility — hooks run in FIFO order
//! (instance → registry-global) before and after core agent methods.

use async_trait::async_trait;
use tracing::debug;

use crate::message::Msg;

use super::error::AgentResult;

/// A hook that can intercept agent method calls.
///
/// Execution order for `reply`: instance pre → registry global pre → agent →
/// registry global post → instance post.
#[async_trait]
pub trait Hook: Send + Sync {
    /// Called before `reply`. May modify the input message.
    async fn pre_reply(&self, msg: Msg) -> AgentResult<Msg> {
        Ok(msg)
    }

    /// Called after `reply`. May modify the output message.
    async fn post_reply(&self, msg: Msg) -> AgentResult<Msg> {
        Ok(msg)
    }

    /// Called before `observe`. May modify the observed message.
    async fn pre_observe(&self, msg: Msg) -> AgentResult<Msg> {
        Ok(msg)
    }

    /// Called after `observe`.
    async fn post_observe(&self) -> AgentResult<()> {
        Ok(())
    }
}

/// Registry of instance-level and registry-scoped global hooks.
pub struct HookRegistry {
    instance_hooks: Vec<Box<dyn Hook>>,
    global_hooks: Vec<Box<dyn Hook>>,
}

impl HookRegistry {
    /// Create an empty hook registry.
    pub fn new() -> Self {
        Self {
            instance_hooks: Vec::new(),
            global_hooks: Vec::new(),
        }
    }

    /// Register an instance-level hook (runs before registry-global hooks).
    pub fn register_instance_hook(&mut self, hook: Box<dyn Hook>) {
        debug!(
            target: "macaca_framework::agent",
            scope = "instance",
            count = self.instance_hooks.len() + 1,
            "hook registered"
        );
        self.instance_hooks.push(hook);
    }

    /// Register a registry-scoped global hook.
    pub fn register_global_hook(&mut self, hook: Box<dyn Hook>) {
        debug!(
            target: "macaca_framework::agent",
            scope = "registry_global",
            count = self.global_hooks.len() + 1,
            "hook registered"
        );
        self.global_hooks.push(hook);
    }

    /// Run all pre-reply hooks: instance → registry global.
    pub async fn run_pre_reply(&self, mut msg: Msg) -> AgentResult<Msg> {
        for hook in &self.instance_hooks {
            msg = hook.pre_reply(msg).await?;
        }
        for hook in &self.global_hooks {
            msg = hook.pre_reply(msg).await?;
        }
        Ok(msg)
    }

    /// Run all post-reply hooks: instance → registry global.
    pub async fn run_post_reply(&self, mut msg: Msg) -> AgentResult<Msg> {
        for hook in &self.instance_hooks {
            msg = hook.post_reply(msg).await?;
        }
        for hook in &self.global_hooks {
            msg = hook.post_reply(msg).await?;
        }
        Ok(msg)
    }

    /// Run all pre-observe hooks: instance → registry global.
    pub async fn run_pre_observe(&self, mut msg: Msg) -> AgentResult<Msg> {
        for hook in &self.instance_hooks {
            msg = hook.pre_observe(msg).await?;
        }
        for hook in &self.global_hooks {
            msg = hook.pre_observe(msg).await?;
        }
        Ok(msg)
    }

    /// Run all post-observe hooks: instance → registry global.
    pub async fn run_post_observe(&self) -> AgentResult<()> {
        for hook in &self.instance_hooks {
            hook.post_observe().await?;
        }
        for hook in &self.global_hooks {
            hook.post_observe().await?;
        }
        Ok(())
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}
