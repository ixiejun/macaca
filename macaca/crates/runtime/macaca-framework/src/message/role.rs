//! Conversation role taxonomy for framework messages.
//!
//! **Design pattern:** Value Object — roles are fixed enumerants with no
//! application-specific semantics; persona routing happens above this layer.

use serde::{Deserialize, Serialize};

/// Message role in a multi-turn conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
    Tool,
}
