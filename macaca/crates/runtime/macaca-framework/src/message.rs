//! Unified message types with rich content blocks (**Facade** module tree).
//!
//! The message system mirrors AgentScope's design:
//! - [`Msg`] is the universal message envelope
//! - [`ContentBlock`] is a tagged enum of seven content types
//! - [`MsgContent`] supports both plain text and block arrays
//!
//! # Module tree (P4 iteration 102)
//! - `blocks` — tagged content block Composite (text, thinking, tools, media)
//! - `content` — [`MsgContent`] Strategy accessors and broadcast filter
//! - `role` — conversation role Value Object
//! - `envelope` — [`Msg`] Builder factories and envelope metadata
//! - `tests` — serde and boundary contract tests

mod blocks;
mod content;
mod envelope;
mod role;

#[cfg(test)]
mod tests;

pub use blocks::{
    AudioBlock, ContentBlock, ImageBlock, TextBlock, ThinkingBlock, ToolResultBlock,
    ToolUseBlock, VideoBlock,
};
pub use content::MsgContent;
pub use envelope::Msg;
pub use role::Role;
