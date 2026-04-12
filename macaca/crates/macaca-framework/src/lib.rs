//! `macaca-framework` — Agent framework for Macaca OS.
//!
//! A Rust implementation inspired by AgentScope, providing:
//! - Unified message types with rich content blocks
//! - Agent trait abstraction with hook injection
//! - ReAct agent with reasoning-action loop
//! - Working and long-term memory with tag system
//! - Toolkit with grouping, middleware, and MCP integration
//! - Pipeline orchestration (Sequential, Fanout, MsgHub)
//! - Plan notebook for agent self-planning
//! - Session persistence and OpenTelemetry tracing
//! - A2A (Agent-to-Agent) protocol support

pub mod a2a;
#[cfg(feature = "macaca-compat")]
pub mod adapter;
pub mod agent;
pub mod execution;
pub mod formatter;
pub mod mcp;
pub mod memory;
pub mod message;
pub mod model;
#[cfg(any(feature = "openai", feature = "anthropic"))]
pub mod model_impls;
pub mod pipeline;
pub mod plan;
pub mod react_agent;
pub mod session;
pub mod state;
pub mod tool;
pub mod tracing_utils;
