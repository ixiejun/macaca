//! Framework-based agent runner — builds `ReActAgent` instances from persona
//! configuration and bridges events to SSE.
//!
//! Facade module: re-exports the public web-shell API while subdirectory adapters
//! own one integration boundary each (SSE, channels, executor broadcast,
//! execution-control middleware, agent factory).

mod agent_factory;
mod agent_factory_build;
mod agent_factory_coordinator;
mod agent_resolution;
mod build_mode;
mod channel_emitter_adapter;
mod context_prompt_builder;
mod context_snapshot_builder;
mod coordinator_builder;
mod driver_trace_adapter;
mod execution_control_middleware;
mod executor_emitter_adapter;
mod prompt_helpers;
mod request_composition;
mod runtime_agent_builders;
mod runtime_execution_control;
mod skill_policy;
mod sse_emitter_adapter;
mod tool_trace;
mod traced_builders;

#[cfg(test)]
pub(crate) mod contract_source;
#[cfg(test)]
mod tests;

pub(crate) use build_mode::{is_framework_tool_wrapper_trace, should_forward_driver_trace};
pub use channel_emitter_adapter::{ChannelEmitterHook, ChannelToolMiddleware};
pub use execution_control_middleware::ExecutionControlMiddleware;
pub use executor_emitter_adapter::{ExecutorEmitterHook, ExecutorToolMiddleware};
pub use runtime_execution_control::RuntimeExecutionControl;
pub(crate) use tool_trace::{tool_response_text, truncate_tool_output};

pub use sse_emitter_adapter::{SseEmitterHook, SseToolMiddleware};

/// Builds `ReActAgent` instances from the existing Macaca OS infrastructure.
pub struct FrameworkRunner;
