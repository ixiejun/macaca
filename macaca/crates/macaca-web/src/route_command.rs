//! Route command boundary for future handler-to-service migration.

use std::future::Future;

use macaca_proto::MacacaResult;

/// A small command abstraction for route handlers that can be moved behind services.
pub trait RouteCommand {
    /// Successful command output.
    type Output;

    /// Execute the command.
    fn execute(self) -> impl Future<Output = MacacaResult<Self::Output>> + Send;
}

/// Adapter for commands that are already represented as async closures.
pub struct AsyncRouteCommand<F> {
    command: F,
}

impl<F> AsyncRouteCommand<F> {
    /// Create a command from an async closure.
    pub fn new(command: F) -> Self {
        Self { command }
    }
}

impl<F, Fut, Output> RouteCommand for AsyncRouteCommand<F>
where
    F: FnOnce() -> Fut + Send,
    Fut: Future<Output = MacacaResult<Output>> + Send,
{
    type Output = Output;

    fn execute(self) -> impl Future<Output = MacacaResult<Self::Output>> + Send {
        (self.command)()
    }
}
