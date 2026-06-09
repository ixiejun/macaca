mod tests {
    use super::super::*;
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
}
