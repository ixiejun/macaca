//! Tool system extension traits (**Strategy** + **Chain of Responsibility**).
//!
//! - [`ToolHandler`] — Strategy: each tool implements execution semantics.
//! - [`ToolMiddleware`] — Chain of Responsibility: cross-cutting hooks around every call.
//! - [`ToolkitResource`] — explicit lifecycle for external runtimes owned by a toolkit.

use async_trait::async_trait;
use serde_json::Value;

use super::error::ToolError;
use super::response::ToolResponse;

/// The core trait every tool must implement (**Strategy** pattern).
///
/// Implementations must be `Send + Sync` so they can be stored behind a shared
/// reference inside [`super::toolkit::Toolkit`] and invoked from async agent loops.
#[async_trait]
pub trait ToolHandler: Send + Sync {
    /// Execute the tool with the given arguments after preset-arg merge and middleware `before` hooks.
    async fn execute(&self, args: Value) -> Result<ToolResponse, ToolError>;

    /// Execute with streaming events support when the `macaca-compat` feature is enabled.
    ///
    /// Default implementation ignores the event channel and delegates to [`execute`](Self::execute),
    /// preserving backward compatibility for handlers that do not emit trace events.
    #[cfg(feature = "macaca-compat")]
    async fn execute_streaming(
        &self,
        args: Value,
        event_tx: Option<tokio::sync::mpsc::UnboundedSender<macaca_tools::TraceEvent>>,
    ) -> Result<ToolResponse, ToolError> {
        let _ = event_tx;
        self.execute(args).await
    }

    /// The unique tool name used for registry lookup and LLM function definitions.
    fn name(&self) -> &str;

    /// Human-readable description exposed to the LLM as tool documentation.
    fn description(&self) -> &str;

    /// JSON Schema describing the `args` object expected by [`execute`](Self::execute).
    fn schema(&self) -> Value;
}

/// A middleware that wraps every tool invocation (**Chain of Responsibility**).
///
/// `before` runs after argument preparation but before `handler.execute`.
/// `after` runs after `handler.execute` returns successfully.
/// Either hook may fail with [`ToolError`], which short-circuits the remaining chain.
#[async_trait]
pub trait ToolMiddleware: Send + Sync {
    /// Called before the tool handler executes.
    ///
    /// Implementations may mutate `args` in place (e.g. inject defaults or redact fields).
    async fn before(&self, name: &str, args: &mut Value) -> Result<(), ToolError>;

    /// Called after the tool handler executes successfully.
    ///
    /// Implementations may mutate `response` in place (e.g. append metadata blocks).
    async fn after(&self, name: &str, response: &mut ToolResponse) -> Result<(), ToolError>;
}

/// Resource owned by a toolkit and cleaned up when the toolkit is dropped.
///
/// Tool handlers can hold shared references to external runtimes such as MCP
/// subprocesses. Registering a cleanup resource makes the ownership explicit:
/// once the agent toolkit goes away, the external runtime is closed too.
pub trait ToolkitResource: Send + Sync {
    /// Release the external resource. Called exactly once during [`super::toolkit::Toolkit`] drop.
    fn close(self: Box<Self>);
}
