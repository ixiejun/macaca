//! Shared test doubles for pipeline integration tests.
//!
//! Each struct implements [`Agent`] with deterministic, application-agnostic
//! behaviour so pipeline ordering, fan-out, and MsgHub broadcast rules can be
//! verified without LLM or external I/O.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::AgentId;
use tokio::sync::Mutex;

use crate::agent::{Agent, AgentResult};
use crate::message::{ContentBlock, Msg, MsgContent, TextBlock, ThinkingBlock};

// ------------------------------------------------------------------
// Test double: AppendAgent
// ------------------------------------------------------------------

pub(crate) struct AppendAgent {
    name: String,
    id: AgentId,
    suffix: String,
    observed: Arc<Mutex<Vec<String>>>,
}

impl AppendAgent {
    pub(crate) fn new(name: &str, suffix: &str) -> Self {
        Self {
            name: name.to_string(),
            id: AgentId::new(),
            suffix: suffix.to_string(),
            observed: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub(crate) async fn observed_texts(&self) -> Vec<String> {
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


// FanoutPipeline
// ------------------------------------------------------------------


// MsgHub
// ------------------------------------------------------------------


// ThinkingAgent & ObserverAgent for broadcast-strip tests
// ------------------------------------------------------------------

pub(crate) struct ThinkingAgent {
    pub(crate) name: String,
    pub(crate) id: AgentId,
    pub(crate) observed: Arc<Mutex<Vec<Msg>>>,
}

#[async_trait]
impl Agent for ThinkingAgent {
    async fn reply(&self, _msg: Msg) -> AgentResult<Msg> {
        let blocks = vec![
            ContentBlock::Thinking(ThinkingBlock {
                thinking: "secret thought".into(),
            }),
            ContentBlock::Text(crate::message::TextBlock {
                text: "visible reply".into(),
            }),
        ];
        Ok(Msg::assistant(&self.name, MsgContent::Blocks(blocks)))
    }

    async fn observe(&self, msg: Msg) -> AgentResult<()> {
        self.observed.lock().await.push(msg);
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn id(&self) -> &AgentId {
        &self.id
    }
}

pub(crate) struct ObserverAgent {
    pub(crate) name: String,
    pub(crate) id: AgentId,
    pub(crate) observed: Arc<Mutex<Vec<Msg>>>,
}

#[async_trait]
impl Agent for ObserverAgent {
    async fn reply(&self, msg: Msg) -> AgentResult<Msg> {
        Ok(Msg::assistant(&self.name, msg.get_text()))
    }

    async fn observe(&self, msg: Msg) -> AgentResult<()> {
        self.observed.lock().await.push(msg);
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn id(&self) -> &AgentId {
        &self.id
    }
}


// Robustness tests
// ------------------------------------------------------------------

/// An agent that always fails.
pub(crate) struct FailAgent {
    name: String,
    id: AgentId,
}

impl FailAgent {
    pub(crate) fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            id: AgentId::new(),
        }
    }
}

#[async_trait]
impl Agent for FailAgent {
    async fn reply(&self, _msg: Msg) -> AgentResult<Msg> {
        Err(crate::agent::AgentError::Other(format!(
            "{} failed",
            self.name
        )))
    }

    async fn observe(&self, _msg: Msg) -> AgentResult<()> {
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn id(&self) -> &AgentId {
        &self.id
    }
}

/// An agent that can be configured to fail or succeed.
pub(crate) struct ConditionalAgent {
    name: String,
    id: AgentId,
    should_fail: bool,
}

impl ConditionalAgent {
    pub(crate) fn new(name: &str, should_fail: bool) -> Self {
        Self {
            name: name.to_string(),
            id: AgentId::new(),
            should_fail,
        }
    }
}

#[async_trait]
impl Agent for ConditionalAgent {
    async fn reply(&self, msg: Msg) -> AgentResult<Msg> {
        if self.should_fail {
            Err(crate::agent::AgentError::Other(format!(
                "{} failed",
                self.name
            )))
        } else {
            Ok(Msg::assistant(
                &self.name,
                format!("{}-{}", msg.get_text(), self.name),
            ))
        }
    }

    async fn observe(&self, _msg: Msg) -> AgentResult<()> {
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn id(&self) -> &AgentId {
        &self.id
    }
}

/// Agent that fails on observe but succeeds on reply (for MsgHub error test).
pub(crate) struct ObserveFailAgent {
    pub(crate) name: String,
    pub(crate) id: AgentId,
}

#[async_trait]
impl Agent for ObserveFailAgent {
    async fn reply(&self, _msg: Msg) -> AgentResult<Msg> {
        Err(crate::agent::AgentError::Other(format!(
            "{} reply error",
            self.name
        )))
    }

    async fn observe(&self, _msg: Msg) -> AgentResult<()> {
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn id(&self) -> &AgentId {
        &self.id
    }
}

