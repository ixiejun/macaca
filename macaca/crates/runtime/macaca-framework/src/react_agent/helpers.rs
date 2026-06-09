//! Shared helpers for the ReAct agent module.

use crate::message::{ContentBlock, MsgContent};
use crate::model::ChatResponse;

/// Convert a `ChatResponse` into a `MsgContent` suitable for storing in memory.
pub(crate) fn response_to_content(response: &ChatResponse) -> MsgContent {
    if response.content.is_empty() {
        MsgContent::Text(String::new())
    } else if response.content.len() == 1 {
        match response.content.first() {
            Some(ContentBlock::Text(t)) => MsgContent::Text(t.text.clone()),
            _ => MsgContent::Blocks(response.content.clone()),
        }
    } else {
        MsgContent::Blocks(response.content.clone())
    }
}
