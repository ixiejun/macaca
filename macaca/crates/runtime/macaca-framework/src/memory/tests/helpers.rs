//! Shared test helpers for memory contract tests.

use crate::message::Msg;

/// Build a deterministic user message for memory tests.
pub(crate) fn user_msg(text: &str) -> Msg {
    Msg::user("tester", text)
}
