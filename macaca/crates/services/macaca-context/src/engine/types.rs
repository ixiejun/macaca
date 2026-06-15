//! Core context-engine contracts, input/output envelopes, and selection policy.
//!
//! This module owns the **Strategy** interface ([`ContextEngine`]) and the stable
//! DTOs exchanged between upper layers (runtime, composer, adapters) and concrete
//! engine implementations. Keeping types separate from implementations lets the
//! registry and facades depend on contracts without importing every strategy.

use async_trait::async_trait;
use macaca_proto::{ApplicationId, LlmMessage, LlmOptions, MacacaResult, ToolDefinition};
use serde::{Deserialize, Serialize};

use crate::budget::ContextBudget;

/// Stable identifier for the passthrough context engine.
///
/// Shared across selection policy, registry resolution, and report builders so
/// engine ids stay consistent without coupling types to module internals.
pub(crate) const PASSTHROUGH_ENGINE_ID: &str = "passthrough";

/// Stable metadata describing a concrete context engine implementation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextEngineInfo {
    pub id: String,
    pub name: String,
    pub version: String,
}

impl ContextEngineInfo {
    /// Build engine metadata using the current crate version as the engine version.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// Optional engine-authored patch for model options.
///
/// The current runtime keeps this minimal, but the type exists so engines can
/// eventually adjust tools or other request options without changing the
/// high-level runtime boundary again.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContextOptionsPatch {
    pub tools: Option<Vec<ToolDefinition>>,
}

/// Full pre-LLM input envelope sent into a context engine.
///
/// Upper layers provide raw messages plus model options and a context budget.
/// The engine is responsible for transforming those messages into the final
/// prompt slice while preserving enough metadata to emit a structured report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextAssembleInput {
    pub app_id: Option<ApplicationId>,
    pub session_id: Option<String>,
    pub agent_name: String,
    pub model: String,
    pub base_messages: Vec<LlmMessage>,
    pub options: LlmOptions,
    pub budget: ContextBudget,
}

impl ContextAssembleInput {
    /// Constructor for call sites that intentionally do not track app/session identity.
    pub fn unscoped(
        agent_name: impl Into<String>,
        model: impl Into<String>,
        base_messages: Vec<LlmMessage>,
        options: LlmOptions,
    ) -> Self {
        Self {
            app_id: None,
            session_id: None,
            agent_name: agent_name.into(),
            model: model.into(),
            base_messages,
            options,
            budget: ContextBudget::default(),
        }
    }

    /// Returns the **most recent non-empty user** message text, scanning from the end.
    ///
    /// ### Why this exists
    /// Vector memory **active recall** and **knowledge digest** compilation both need a
    /// retrieval-oriented query derived from the live user turn instead of inventing synthetic
    /// keywords. Sharing one helper keeps [`crate::memory::MemoryRecallQuery`] construction
    /// deterministic across providers (Adapter pattern).
    pub fn last_user_message_text(&self) -> Option<String> {
        use macaca_proto::LlmRole;
        for message in self.base_messages.iter().rev() {
            if message.role == LlmRole::User {
                let trimmed = message.content.trim();
                if !trimmed.is_empty() {
                    return Some(message.content.clone());
                }
            }
        }
        None
    }
}

/// Output of one context-engine assembly pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextAssembleResult {
    pub messages: Vec<LlmMessage>,
    pub options: LlmOptions,
    pub options_patch: ContextOptionsPatch,
    pub report: crate::report::ContextReport,
}

/// Post-turn callback input for engines that need to observe completed turns.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContextAfterTurnInput {
    pub app_id: Option<ApplicationId>,
    pub session_id: Option<String>,
    pub agent_name: String,
    pub report: Option<crate::report::ContextReport>,
}

/// Provider-neutral context-engine contract (**Strategy** pattern).
///
/// `assemble()` runs immediately before a model call. Implementations may trim,
/// prune, summarize, or annotate the outgoing context, but must always return a
/// `ContextReport` explaining what happened.
#[async_trait]
pub trait ContextEngine: Send + Sync {
    fn info(&self) -> ContextEngineInfo;

    async fn assemble(&self, input: ContextAssembleInput) -> MacacaResult<ContextAssembleResult>;

    async fn after_turn(&self, _input: ContextAfterTurnInput) -> MacacaResult<()> {
        Ok(())
    }
}

/// Runtime selection and fallback policy for context engines.
///
/// Encodes **Chain of Responsibility** configuration: the runtime tries
/// `engine_id` first and falls back to `fallback_engine_id` on failure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextEngineSelection {
    pub engine_id: String,
    pub fallback_engine_id: String,
}

impl ContextEngineSelection {
    /// Default selection that pins both primary and fallback to the passthrough engine.
    pub fn passthrough_default() -> Self {
        Self {
            engine_id: PASSTHROUGH_ENGINE_ID.into(),
            fallback_engine_id: PASSTHROUGH_ENGINE_ID.into(),
        }
    }
}
