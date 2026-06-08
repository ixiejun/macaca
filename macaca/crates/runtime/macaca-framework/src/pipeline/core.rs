//! [`Pipeline`] trait — composable message transformation unit (Strategy).
//!
//! Any type implementing this trait can be chained, nested, or executed as a
//! standalone orchestration step. Callers depend on the trait, not concrete
//! pipeline implementations, which keeps the framework application-agnostic.

use async_trait::async_trait;

use crate::agent::AgentResult;
use crate::message::Msg;

/// A composable unit that transforms an inbound [`Msg`] into an outbound [`Msg`].
///
/// **Design pattern:** Strategy — multiple pipeline algorithms (`SequentialPipeline`,
/// `FanoutPipeline`, `MsgHub`) share this uniform execution interface.
#[async_trait]
pub trait Pipeline: Send + Sync {
    /// Execute the pipeline on `msg` and return the transformed result.
    ///
    /// Implementations may invoke one or many underlying [`Agent`](crate::agent::Agent)
    /// instances; errors from any agent propagate immediately unless the concrete
    /// pipeline defines alternate aggregation semantics (e.g. fanout first-success).
    async fn run(&self, msg: Msg) -> AgentResult<Msg>;
}
