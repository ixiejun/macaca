//! Strategy trait for provider-specific message formatters.
//!
//! **Design pattern:** Strategy — each concrete formatter implements the same
//! [`Formatter`] interface so HTTP adapters can swap wire shapes at runtime.

use serde_json::Value;

use crate::message::Msg;
use crate::model::ChatResponse;

use super::error::FormatterError;

/// Converts between framework `Msg` values and a provider's wire format.
pub trait Formatter: Send + Sync {
    /// Convert a slice of framework messages into provider-specific JSON objects.
    ///
    /// The returned `Vec<Value>` is passed directly as the `messages` array in
    /// the provider's API request.
    fn format(&self, msgs: &[Msg]) -> Vec<Value>;

    /// Parse the raw JSON body returned by the provider into a `ChatResponse`.
    fn parse_response(&self, raw: Value) -> Result<ChatResponse, FormatterError>;
}
