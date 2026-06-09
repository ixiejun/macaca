//! Agent abstraction and hook system.
//!
//! The `Agent` trait defines the core interface for all agents.
//! `HookedAgent` wraps any `Agent` to inject pre/post hooks on reply and observe.

use std::sync::OnceLock;

use async_trait::async_trait;
use macaca_proto::AgentId;
use tokio::sync::RwLock as TokioRwLock;

use crate::message::Msg;

/// Errors from agent execution.
#[derive(Debug, Clone, thiserror::Error)]
pub enum AgentError {
    #[error("LLM error: {0}")]
    Llm(String),
    #[error("Tool error: {0}")]
    Tool(String),
    #[error("Interrupted")]
    Interrupted,
    #[error("Max iterations reached: {0}")]
    MaxIterations(usize),
    #[error("{0}")]
    Other(String),
}

pub type AgentResult<T> = Result<T, AgentError>;

/// The core Agent trait.
///
/// Every agent in the framework implements this trait. The two essential
/// methods are:
/// - `reply` — generate a response to an input message
/// - `observe` — receive a message without generating a response
#[async_trait]
pub trait Agent: Send + Sync {
    /// Generate a reply to the given message.
    ///
    /// This is the core method that drives agent behavior. Implementations
    /// may call LLMs, execute tools, consult memory, etc.
    async fn reply(&self, msg: Msg) -> AgentResult<Msg>;

    /// Observe a message without generating a reply.
    ///
    /// Used in multi-agent scenarios where an agent needs to be aware of
    /// messages exchanged between other agents. Default implementation
    /// is a no-op; agents with memory should override to store the message.
    async fn observe(&self, _msg: Msg) -> AgentResult<()> {
        Ok(())
    }

    /// Request interruption of the current reply execution.
    ///
    /// Default implementation is a no-op. Agents that support interruption
    /// should check a cancellation signal during their reply loop.
    async fn interrupt(&self, _msg: Msg) -> AgentResult<()> {
        Ok(())
    }

    /// The agent's display name.
    fn name(&self) -> &str;

    /// The agent's unique identifier.
    fn id(&self) -> &AgentId;
}

// ---------------------------------------------------------------------------
// Hook System
// ---------------------------------------------------------------------------

/// A hook that can intercept agent method calls.
///
/// Hooks execute in order: instance pre → class pre → method → instance post → class post.
#[async_trait]
pub trait Hook: Send + Sync {
    /// Called before `reply`. Can modify the input message.
    async fn pre_reply(&self, msg: Msg) -> AgentResult<Msg> {
        Ok(msg)
    }

    /// Called after `reply`. Can modify the output message.
    async fn post_reply(&self, msg: Msg) -> AgentResult<Msg> {
        Ok(msg)
    }

    /// Called before `observe`. Can modify the observed message.
    async fn pre_observe(&self, msg: Msg) -> AgentResult<Msg> {
        Ok(msg)
    }

    /// Called after `observe`.
    async fn post_observe(&self) -> AgentResult<()> {
        Ok(())
    }
}

/// Registry of hooks (instance-level and global-level).
pub struct HookRegistry {
    instance_hooks: Vec<Box<dyn Hook>>,
    global_hooks: Vec<Box<dyn Hook>>,
}

impl HookRegistry {
    pub fn new() -> Self {
        Self {
            instance_hooks: Vec::new(),
            global_hooks: Vec::new(),
        }
    }

    /// Register an instance-level hook.
    pub fn register_instance_hook(&mut self, hook: Box<dyn Hook>) {
        self.instance_hooks.push(hook);
    }

    /// Register a global-level hook (affects all HookedAgents sharing this registry).
    pub fn register_global_hook(&mut self, hook: Box<dyn Hook>) {
        self.global_hooks.push(hook);
    }

    /// Run all pre-reply hooks in order: instance → global.
    pub async fn run_pre_reply(&self, mut msg: Msg) -> AgentResult<Msg> {
        for hook in &self.instance_hooks {
            msg = hook.pre_reply(msg).await?;
        }
        for hook in &self.global_hooks {
            msg = hook.pre_reply(msg).await?;
        }
        Ok(msg)
    }

    /// Run all post-reply hooks in order: instance → global.
    pub async fn run_post_reply(&self, mut msg: Msg) -> AgentResult<Msg> {
        for hook in &self.instance_hooks {
            msg = hook.post_reply(msg).await?;
        }
        for hook in &self.global_hooks {
            msg = hook.post_reply(msg).await?;
        }
        Ok(msg)
    }

    /// Run all pre-observe hooks.
    pub async fn run_pre_observe(&self, mut msg: Msg) -> AgentResult<Msg> {
        for hook in &self.instance_hooks {
            msg = hook.pre_observe(msg).await?;
        }
        for hook in &self.global_hooks {
            msg = hook.pre_observe(msg).await?;
        }
        Ok(msg)
    }

    /// Run all post-observe hooks.
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

// ---------------------------------------------------------------------------
// Static global hook registry
// ---------------------------------------------------------------------------

/// Process-wide global hook registry.
/// Hooks registered here are automatically applied to ALL HookedAgent instances.
static GLOBAL_HOOK_REGISTRY: OnceLock<TokioRwLock<Vec<Box<dyn Hook>>>> = OnceLock::new();

fn global_registry() -> &'static TokioRwLock<Vec<Box<dyn Hook>>> {
    GLOBAL_HOOK_REGISTRY.get_or_init(|| TokioRwLock::new(Vec::new()))
}

/// Register a hook that will be applied to ALL HookedAgent instances process-wide.
/// This is the Rust equivalent of agentscope's class-level hooks via metaclasses.
pub async fn register_static_hook(hook: Box<dyn Hook>) {
    global_registry().write().await.push(hook);
}

/// Clear all process-wide global hooks (useful for testing).
pub async fn clear_static_hooks() {
    global_registry().write().await.clear();
}

/// Run static global pre-reply hooks on a message.
pub(crate) async fn run_static_pre_reply(mut msg: Msg) -> AgentResult<Msg> {
    let hooks = global_registry().read().await;
    for hook in hooks.iter() {
        msg = hook.pre_reply(msg).await?;
    }
    Ok(msg)
}

/// Run static global post-reply hooks on a message.
pub(crate) async fn run_static_post_reply(mut msg: Msg) -> AgentResult<Msg> {
    let hooks = global_registry().read().await;
    for hook in hooks.iter() {
        msg = hook.post_reply(msg).await?;
    }
    Ok(msg)
}

/// Run static global pre-observe hooks.
pub(crate) async fn run_static_pre_observe(mut msg: Msg) -> AgentResult<Msg> {
    let hooks = global_registry().read().await;
    for hook in hooks.iter() {
        msg = hook.pre_observe(msg).await?;
    }
    Ok(msg)
}

/// Run static global post-observe hooks.
pub(crate) async fn run_static_post_observe() -> AgentResult<()> {
    let hooks = global_registry().read().await;
    for hook in hooks.iter() {
        hook.post_observe().await?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// HookedAgent — transparent wrapper that injects hooks
// ---------------------------------------------------------------------------

/// A wrapper that injects pre/post hooks around an inner agent's methods.
///
/// The wrapper is transparent: it implements `Agent` and delegates to the
/// inner agent, running hooks before and after each method call.
pub struct HookedAgent<A: Agent> {
    inner: A,
    hooks: HookRegistry,
}

impl<A: Agent> HookedAgent<A> {
    /// Wrap an agent with a hook registry.
    pub fn new(inner: A, hooks: HookRegistry) -> Self {
        Self { inner, hooks }
    }

    /// Wrap an agent with no hooks (can be added later via mutable access).
    pub fn wrap(inner: A) -> Self {
        Self::new(inner, HookRegistry::new())
    }

    /// Get mutable access to the hook registry.
    pub fn hooks_mut(&mut self) -> &mut HookRegistry {
        &mut self.hooks
    }

    /// Get a reference to the inner agent.
    pub fn inner(&self) -> &A {
        &self.inner
    }
}

#[async_trait]
impl<A: Agent> Agent for HookedAgent<A> {
    async fn reply(&self, msg: Msg) -> AgentResult<Msg> {
        // Pre hooks: instance → global (registry) → static global
        let msg = self.hooks.run_pre_reply(msg).await?;
        let msg = run_static_pre_reply(msg).await?;

        // Core agent method
        let reply = self.inner.reply(msg).await?;

        // Post hooks: static global → global (registry) → instance
        let reply = run_static_post_reply(reply).await?;
        let reply = self.hooks.run_post_reply(reply).await?;

        Ok(reply)
    }

    async fn observe(&self, msg: Msg) -> AgentResult<()> {
        // Pre hooks: instance → global (registry) → static global
        let msg = self.hooks.run_pre_observe(msg).await?;
        let msg = run_static_pre_observe(msg).await?;

        // Core agent method
        self.inner.observe(msg).await?;

        // Post hooks: static global → global (registry) → instance
        run_static_post_observe().await?;
        self.hooks.run_post_observe().await
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "agent_tests.rs"]
mod agent_tests;
