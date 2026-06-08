//! Tagged message value object used by working memory mark filters.

use crate::message::Msg;

// ---------------------------------------------------------------------------
// TaggedMsg — message with attached string labels
// ---------------------------------------------------------------------------

/// A message annotated with one or more string labels (marks).
///
/// Marks enable selective retrieval, bulk deletion, and lifecycle management
/// without mutating the underlying `Msg`.
#[derive(Debug, Clone)]
pub struct TaggedMsg {
    /// The wrapped message.
    pub msg: Msg,
    /// Labels attached to this message (e.g. "compressed", "pinned", "draft").
    pub marks: Vec<String>,
}
