// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

//! Provider-neutral tool system facade.
//!
//! The module keeps the public `crate::tool::*` API stable while splitting the
//! implementation by ownership:
//! - [`types`] owns bounded DTOs and error values.
//! - [`traits`] owns the handler, middleware, and cleanup resource contracts.
//! - [`registry`] owns toolkit registration, group state, and resources.
//! - [`invocation`] owns the command execution path and LLM-facing definitions.
//!
//! This is a Facade module: callers keep importing `Toolkit`, `ToolHandler`,
//! and related types from `crate::tool`, while each submodule stays small enough
//! to audit independently under the Macaca OS file-size constitution.

mod invocation;
mod registry;
mod traits;
mod types;

#[path = "../tool_schema.rs"]
mod tool_schema_impl;

pub use crate::tool_schema;
pub use registry::{RegisteredTool, ToolGroup, Toolkit};
pub use traits::{ToolHandler, ToolMiddleware, ToolkitResource};
pub use types::{ToolError, ToolResponse, ToolTraceEvent};

#[cfg(test)]
#[path = "../tool_tests.rs"]
mod tool_tests;

#[cfg(test)]
#[path = "../tool_schema_tests.rs"]
mod tool_schema_tests;
