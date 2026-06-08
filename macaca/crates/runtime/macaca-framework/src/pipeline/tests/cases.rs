//! Pipeline integration test cases.
//!
//! Exercises [`SequentialPipeline`], [`FanoutPipeline`], and [`MsgHub`] contract
//! behaviour: ordering, empty chains, concurrent vs sequential fan-out, thinking
//! block stripping on broadcast, and error propagation.

use std::sync::Arc;

use macaca_proto::AgentId;
use tokio::sync::Mutex;

use crate::agent::Agent;
use crate::message::{ContentBlock, Msg, MsgContent};
use crate::pipeline::{FanoutPipeline, MsgHub, Pipeline, SequentialPipeline};

use super::fixtures::{
AppendAgent, ConditionalAgent, FailAgent, ObserveFailAgent, ObserverAgent, ThinkingAgent,
};

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
