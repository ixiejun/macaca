//! IPC envelopes, gateway events, and channel message formats.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::identity::{AgentId, MessageId, TaskId};

// ── Message Types (IPC) ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcMessage {
    pub id: MessageId,
    pub from: AgentId,
    pub to: Option<AgentId>,
    pub topic: String,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

// ── Gateway Types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GatewayEvent {
    TaskRequest {
        user_id: String,
        channel_id: String,
        content: String,
    },
    StatusQuery {
        user_id: String,
        channel_id: String,
        task_id: Option<TaskId>,
    },
    UserReply {
        user_id: String,
        channel_id: String,
        content: String,
        context_id: String,
    },
    Command {
        user_id: String,
        channel_id: String,
        command: String,
        args: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayMessage {
    pub content: String,
    pub format: MessageFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageFormat {
    PlainText,
    Markdown,
    CodeBlock,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAttachment {
    pub filename: String,
    pub data: Vec<u8>,
    pub mime_type: String,
}
