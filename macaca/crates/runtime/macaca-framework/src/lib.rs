//! `macaca-framework` — Agent framework for Macaca OS.
//!
//! A Rust implementation inspired by AgentScope 2.0 concepts, providing:
//! - Unified message types with rich content blocks
//! - Agent trait abstraction with typed runtime boundaries
//! - Event-stream-first ReAct agent with reasoning-action loop
//! - Provider-neutral runtime context, state, and session contracts
//! - Toolkit with grouping, middleware, and MCP integration
//! - Pipeline orchestration (Sequential, Fanout, MsgHub)
//! - Plan notebook for agent self-planning
//! - Session persistence and OpenTelemetry tracing
//! - A2A (Agent-to-Agent) protocol support

pub mod a2a;
pub mod agent;
pub mod agent_contracts;
pub mod construction;
pub mod event_contract;
pub mod execution;
pub mod formatter;
pub mod harness;
pub mod harness_contracts;
/// OpenAI-style chat JSON ⇄ `LlmMessage` bridge (shared carrier for `ContextFacade`).
pub mod llm_wire;
pub mod mcp;
pub mod mcp_content_contract;
pub mod mcp_contract;
pub mod message;
pub mod message_metadata;
pub mod middleware;
pub mod model;
pub mod model_contract;
pub mod model_transport_contract;
pub mod permission_contract;
pub mod pipeline;
pub mod plan;
pub mod plugin_hooks;
pub mod protocol_adapters;
pub mod provider_contract;
pub mod react_agent;
pub mod runtime_context;
pub mod tool;
pub mod tool_contract;
pub mod tracing_utils;
