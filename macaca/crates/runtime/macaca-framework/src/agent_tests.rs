mod tests {
    use super::super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use async_trait::async_trait;
    use macaca_proto::AgentId;

    use crate::message::Msg;

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
