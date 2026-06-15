// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

//! Toolkit invocation path.
//!
//! This module owns the Command-style execution sequence for a registered tool:
//! lookup, group policy, preset merge, middleware decorators, handler call, and
//! LLM-facing definition projection. It stays separate from registry mutation
//! so audit reviews can reason about side-effecting invocation independently.

use serde_json::{self, Value};

use super::{ToolError, ToolResponse, Toolkit};

impl Toolkit {
    /// Invoke a tool by name.
    ///
    /// Execution order:
    /// 1. Look up the registered handler.
    /// 2. Enforce group activation.
    /// 3. Merge preset args with caller args.
    /// 4. Run `before` middleware in registration order.
    /// 5. Execute the handler with optional trace streaming.
    /// 6. Run `after` middleware in registration order.
    pub async fn call_tool(&self, name: &str, args: Value) -> Result<ToolResponse, ToolError> {
        let registered = self
            .tools
            .get(name)
            .ok_or_else(|| ToolError::NotFound(name.to_string()))?;

        let group_active = if registered.group == "basic" {
            true
        } else {
            self.groups
                .get(&registered.group)
                .map(|g| g.active)
                .unwrap_or(false)
        };
        if !group_active {
            return Err(ToolError::PermissionDenied(format!(
                "tool '{}' belongs to inactive group '{}'",
                name, registered.group
            )));
        }

        let mut merged = match registered.preset_args.clone() {
            Value::Object(map) => map,
            _ => Default::default(),
        };
        if let Value::Object(caller_map) = args {
            for (k, v) in caller_map {
                merged.insert(k, v);
            }
        }
        let mut effective_args = Value::Object(merged);

        for mw in &self.middlewares {
            mw.before(name, &mut effective_args).await?;
        }

        let mut response = registered
            .handler
            .execute_streaming(effective_args, self.event_tx.clone())
            .await?;

        for mw in &self.middlewares {
            mw.after(name, &mut response).await?;
        }

        Ok(response)
    }

    /// Return tool definitions for all active tools in all active groups.
    ///
    /// Format: `[{"name": "...", "description": "...", "parameters": {schema}}]`.
    /// The projection is intentionally read-only and provider-neutral; it does
    /// not expose handler internals or service-specific payloads to callers.
    pub fn get_definitions(&self) -> Vec<Value> {
        self.tools
            .values()
            .filter(|t| {
                if t.group == "basic" {
                    true
                } else {
                    self.groups.get(&t.group).map(|g| g.active).unwrap_or(false)
                }
            })
            .map(|t| {
                serde_json::json!({
                    "name": t.handler.name(),
                    "description": t.handler.description(),
                    "parameters": t.handler.schema(),
                })
            })
            .collect()
    }
}
