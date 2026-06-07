//! HTTP request/response DTOs for chat routes.

use serde::Deserialize;

use crate::routes::default_model;

#[derive(Deserialize)]
pub(crate) struct ChatRequest {
    pub app_id: String,
    pub prompt: String,
    #[serde(default = "default_model")]
    pub model: String,
    /// Optional session_id for continuing a conversation, or null for new session
    #[serde(default)]
    pub session_id: Option<String>,
    /// Execution engine: "legacy" (default) or "framework" (ReActAgent-based).
    #[serde(default)]
    pub engine: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct StopRequest {
    pub app_id: String,
}
