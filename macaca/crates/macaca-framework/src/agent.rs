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
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    struct EchoAgent {
        agent_name: String,
        agent_id: AgentId,
    }

    impl EchoAgent {
        fn new(name: &str) -> Self {
            Self {
                agent_name: name.to_string(),
                agent_id: AgentId::new(),
            }
        }
    }

    #[async_trait]
    impl Agent for EchoAgent {
        async fn reply(&self, msg: Msg) -> AgentResult<Msg> {
            Ok(Msg::assistant(
                &self.agent_name,
                format!("Echo: {}", msg.get_text()),
            ))
        }

        fn name(&self) -> &str {
            &self.agent_name
        }

        fn id(&self) -> &AgentId {
            &self.agent_id
        }
    }

    struct CountingHook {
        pre_count: Arc<AtomicUsize>,
        post_count: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl Hook for CountingHook {
        async fn pre_reply(&self, msg: Msg) -> AgentResult<Msg> {
            self.pre_count.fetch_add(1, Ordering::SeqCst);
            Ok(msg)
        }

        async fn post_reply(&self, msg: Msg) -> AgentResult<Msg> {
            self.post_count.fetch_add(1, Ordering::SeqCst);
            Ok(msg)
        }
    }

    #[tokio::test]
    async fn test_echo_agent() {
        let agent = EchoAgent::new("bot");
        let msg = Msg::user("alice", "hello");
        let reply = agent.reply(msg).await.unwrap();
        assert_eq!(reply.get_text(), "Echo: hello");
        assert_eq!(reply.name, "bot");
    }

    #[tokio::test]
    async fn test_hooked_agent_runs_hooks() {
        let pre = Arc::new(AtomicUsize::new(0));
        let post = Arc::new(AtomicUsize::new(0));

        let hook = CountingHook {
            pre_count: Arc::clone(&pre),
            post_count: Arc::clone(&post),
        };

        let mut hooks = HookRegistry::new();
        hooks.register_instance_hook(Box::new(hook));

        let agent = HookedAgent::new(EchoAgent::new("bot"), hooks);
        let msg = Msg::user("alice", "hi");
        let reply = agent.reply(msg).await.unwrap();

        assert_eq!(reply.get_text(), "Echo: hi");
        assert_eq!(pre.load(Ordering::SeqCst), 1);
        assert_eq!(post.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_hooked_agent_transparent_name() {
        let agent = HookedAgent::wrap(EchoAgent::new("mybot"));
        assert_eq!(agent.name(), "mybot");
    }

    // -----------------------------------------------------------------------
    // Boundary tests
    // -----------------------------------------------------------------------

    /// Hook that modifies the input message's name field.
    struct RenameInputHook {
        new_name: String,
    }

    #[async_trait]
    impl Hook for RenameInputHook {
        async fn pre_reply(&self, mut msg: Msg) -> AgentResult<Msg> {
            msg.name = self.new_name.clone();
            Ok(msg)
        }
    }

    /// Hook that appends a suffix to the reply content.
    struct AppendOutputHook {
        suffix: String,
    }

    #[async_trait]
    impl Hook for AppendOutputHook {
        async fn post_reply(&self, msg: Msg) -> AgentResult<Msg> {
            let new_text = format!("{}{}", msg.get_text(), self.suffix);
            Ok(Msg::assistant(&msg.name, new_text.as_str()))
        }
    }

    /// Hook that appends a tag to the message name (for ordering tests).
    struct TaggingHook {
        tag: String,
    }

    #[async_trait]
    impl Hook for TaggingHook {
        async fn pre_reply(&self, mut msg: Msg) -> AgentResult<Msg> {
            msg.name = format!("{},{}", msg.name, self.tag);
            Ok(msg)
        }
    }

    /// Hook that always returns an error.
    struct ErrorHook;

    #[async_trait]
    impl Hook for ErrorHook {
        async fn pre_reply(&self, _msg: Msg) -> AgentResult<Msg> {
            Err(AgentError::Other("hook error".into()))
        }
    }

    /// Agent that captures the received message name (to verify hook modifications).
    struct CapturingAgent {
        agent_name: String,
        agent_id: AgentId,
        captured_name: Arc<tokio::sync::Mutex<String>>,
    }

    impl CapturingAgent {
        fn new(name: &str) -> (Self, Arc<tokio::sync::Mutex<String>>) {
            let captured = Arc::new(tokio::sync::Mutex::new(String::new()));
            (
                Self {
                    agent_name: name.to_string(),
                    agent_id: AgentId::new(),
                    captured_name: Arc::clone(&captured),
                },
                captured,
            )
        }
    }

    #[async_trait]
    impl Agent for CapturingAgent {
        async fn reply(&self, msg: Msg) -> AgentResult<Msg> {
            *self.captured_name.lock().await = msg.name.clone();
            Ok(Msg::assistant(&self.agent_name, msg.get_text()))
        }

        fn name(&self) -> &str {
            &self.agent_name
        }

        fn id(&self) -> &AgentId {
            &self.agent_id
        }
    }

    #[tokio::test]
    async fn test_hook_modifies_input() {
        let (agent, captured) = CapturingAgent::new("bot");
        let mut hooks = HookRegistry::new();
        hooks.register_instance_hook(Box::new(RenameInputHook {
            new_name: "modified_sender".into(),
        }));
        let hooked = HookedAgent::new(agent, hooks);
        let msg = Msg::user("original_sender", "hello");
        let _ = hooked.reply(msg).await.unwrap();
        assert_eq!(*captured.lock().await, "modified_sender");
    }

    #[tokio::test]
    async fn test_hook_modifies_output() {
        let agent = EchoAgent::new("bot");
        let mut hooks = HookRegistry::new();
        hooks.register_instance_hook(Box::new(AppendOutputHook {
            suffix: " [modified]".into(),
        }));
        let hooked = HookedAgent::new(agent, hooks);
        let msg = Msg::user("alice", "hi");
        let reply = hooked.reply(msg).await.unwrap();
        assert_eq!(reply.get_text(), "Echo: hi [modified]");
    }

    #[tokio::test]
    async fn test_multiple_hooks_fifo_order() {
        let (agent, captured) = CapturingAgent::new("bot");
        let mut hooks = HookRegistry::new();
        hooks.register_instance_hook(Box::new(TaggingHook { tag: "A".into() }));
        hooks.register_instance_hook(Box::new(TaggingHook { tag: "B".into() }));
        hooks.register_instance_hook(Box::new(TaggingHook { tag: "C".into() }));
        let hooked = HookedAgent::new(agent, hooks);
        let msg = Msg::user("start", "hello");
        let _ = hooked.reply(msg).await.unwrap();
        // Hooks run in registration (FIFO) order: start → A → B → C
        assert_eq!(*captured.lock().await, "start,A,B,C");
    }

    #[tokio::test]
    async fn test_hook_error_propagation() {
        let agent = EchoAgent::new("bot");
        let mut hooks = HookRegistry::new();
        hooks.register_instance_hook(Box::new(ErrorHook));
        let hooked = HookedAgent::new(agent, hooks);
        let msg = Msg::user("alice", "hi");
        let result = hooked.reply(msg).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("hook error"));
    }

    #[tokio::test]
    async fn test_empty_hook_registry_passthrough() {
        let agent_direct = EchoAgent::new("bot");
        let agent_hooked = HookedAgent::wrap(EchoAgent::new("bot"));

        let msg1 = Msg::user("alice", "hello");
        let msg2 = Msg::user("alice", "hello");

        let reply_direct = agent_direct.reply(msg1).await.unwrap();
        let reply_hooked = agent_hooked.reply(msg2).await.unwrap();

        assert_eq!(reply_direct.get_text(), reply_hooked.get_text());
        assert_eq!(reply_direct.name, reply_hooked.name);
        assert_eq!(reply_direct.role, reply_hooked.role);
    }

    #[tokio::test]
    async fn test_observe_default_noop() {
        let agent = EchoAgent::new("bot");
        let msg = Msg::user("alice", "just observing");
        // Default observe() should succeed without error
        let result = agent.observe(msg).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_agent_send_sync_bounds() {
        fn assert_send_sync<T: Send + Sync>() {}
        // Compile-time check: Agent trait requires Send + Sync
        assert_send_sync::<EchoAgent>();
        assert_send_sync::<HookedAgent<EchoAgent>>();
    }

    // -----------------------------------------------------------------------
    // Static global hook tests
    // These tests share the process-wide GLOBAL_HOOK_REGISTRY, so they must
    // not run concurrently.  We use a shared Mutex to serialise them.
    // -----------------------------------------------------------------------

    static HOOK_TEST_MUTEX: std::sync::LazyLock<tokio::sync::Mutex<()>> =
        std::sync::LazyLock::new(|| tokio::sync::Mutex::new(()));

    #[tokio::test]
    async fn test_static_hook_affects_hooked_agent() {
        let _guard = HOOK_TEST_MUTEX.lock().await;
        // Clear any leftover state from other tests.
        clear_static_hooks().await;

        let pre = Arc::new(AtomicUsize::new(0));
        let post = Arc::new(AtomicUsize::new(0));

        register_static_hook(Box::new(CountingHook {
            pre_count: Arc::clone(&pre),
            post_count: Arc::clone(&post),
        }))
        .await;

        let agent = HookedAgent::wrap(EchoAgent::new("bot"));
        let msg = Msg::user("alice", "hello");
        let reply = agent.reply(msg).await.unwrap();

        assert_eq!(reply.get_text(), "Echo: hello");
        assert_eq!(pre.load(Ordering::SeqCst), 1);
        assert_eq!(post.load(Ordering::SeqCst), 1);

        // Cleanup
        clear_static_hooks().await;
    }

    #[tokio::test]
    async fn test_static_hook_clear() {
        let _guard = HOOK_TEST_MUTEX.lock().await;
        clear_static_hooks().await;

        let pre = Arc::new(AtomicUsize::new(0));
        let post = Arc::new(AtomicUsize::new(0));

        register_static_hook(Box::new(CountingHook {
            pre_count: Arc::clone(&pre),
            post_count: Arc::clone(&post),
        }))
        .await;

        // Clear immediately
        clear_static_hooks().await;

        let agent = HookedAgent::wrap(EchoAgent::new("bot"));
        let msg = Msg::user("alice", "hello");
        let _ = agent.reply(msg).await.unwrap();

        // Hook should NOT have fired
        assert_eq!(pre.load(Ordering::SeqCst), 0);
        assert_eq!(post.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_static_and_instance_hooks_order() {
        let _guard = HOOK_TEST_MUTEX.lock().await;
        clear_static_hooks().await;

        // Static hook appends ",static"
        register_static_hook(Box::new(TaggingHook {
            tag: "static".into(),
        }))
        .await;

        // Instance hook appends ",instance"
        let (agent, captured) = CapturingAgent::new("bot");
        let mut hooks = HookRegistry::new();
        hooks.register_instance_hook(Box::new(TaggingHook {
            tag: "instance".into(),
        }));
        let hooked = HookedAgent::new(agent, hooks);

        let msg = Msg::user("start", "hello");
        let _ = hooked.reply(msg).await.unwrap();

        // Order: instance pre → static pre → agent
        // So name should be "start,instance,static"
        assert_eq!(*captured.lock().await, "start,instance,static");

        // Cleanup
        clear_static_hooks().await;
    }
}
