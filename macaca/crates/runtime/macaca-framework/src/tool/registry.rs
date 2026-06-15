// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

//! Toolkit registry and group state.
//!
//! The registry is deliberately provider-neutral. It stores handlers behind the
//! [`ToolHandler`] trait and exposes group activation as data, while invocation
//! remains in `invocation.rs` so registration and execution can be audited
//! independently.

use std::collections::HashMap;

use serde_json::Value;

use super::{ToolHandler, ToolMiddleware, ToolTraceEvent, ToolkitResource};

/// A named group of tools that can be activated or deactivated as a unit.
#[derive(Debug, Clone)]
pub struct ToolGroup {
    /// Group identifier.
    pub name: String,
    /// Names of tools belonging to this group.
    pub tool_names: Vec<String>,
    /// Whether this group is currently active.
    pub active: bool,
}

/// A tool stored inside the `Toolkit`, together with its group and preset args.
pub struct RegisteredTool {
    /// The concrete handler.
    pub handler: Box<dyn ToolHandler>,
    /// The group this tool belongs to.
    pub group: String,
    /// Arguments merged into every call before middleware runs.
    ///
    /// Preset args have lower priority than caller-supplied args, so the
    /// caller's values win on key conflicts.
    pub preset_args: Value,
}

/// Central registry for tools, groups, middleware, and cleanup resources.
///
/// # Group semantics
///
/// Every tool belongs to exactly one group. The built-in `"basic"` group is
/// always active and cannot be deactivated. Other groups start active by
/// default but can be toggled with [`Toolkit::set_group_active`].
pub struct Toolkit {
    pub(in crate::tool) tools: HashMap<String, RegisteredTool>,
    pub(in crate::tool) groups: HashMap<String, ToolGroup>,
    pub(in crate::tool) middlewares: Vec<Box<dyn ToolMiddleware>>,
    pub(in crate::tool) resources: Vec<Box<dyn ToolkitResource>>,
    /// Optional provider-neutral event channel for streaming tool traces.
    pub(in crate::tool) event_tx: Option<tokio::sync::mpsc::UnboundedSender<ToolTraceEvent>>,
}

impl Toolkit {
    /// Create an empty toolkit.
    ///
    /// The `"basic"` group is pre-registered and always active so callers can
    /// register simple tools without creating a group first.
    pub fn new() -> Self {
        let mut groups = HashMap::new();
        groups.insert(
            "basic".to_string(),
            ToolGroup {
                name: "basic".to_string(),
                tool_names: Vec::new(),
                active: true,
            },
        );
        Self {
            tools: HashMap::new(),
            groups,
            middlewares: Vec::new(),
            resources: Vec::new(),
            event_tx: None,
        }
    }

    /// Register a tool into the given group.
    ///
    /// If the group does not yet exist it is created and marked active. If a
    /// tool with the same name already exists, the handler is replaced while
    /// the public lookup name stays stable.
    pub fn register(&mut self, handler: Box<dyn ToolHandler>, group: Option<&str>) {
        let group_name = group.unwrap_or("basic").to_string();
        let tool_name = handler.name().to_string();

        self.groups
            .entry(group_name.clone())
            .or_insert_with(|| ToolGroup {
                name: group_name.clone(),
                tool_names: Vec::new(),
                active: true,
            })
            .tool_names
            .push(tool_name.clone());

        self.tools.insert(
            tool_name,
            RegisteredTool {
                handler,
                group: group_name,
                preset_args: Value::Object(Default::default()),
            },
        );
    }

    /// Remove a tool by name and detach it from its group membership list.
    pub fn unregister(&mut self, name: &str) {
        if let Some(tool) = self.tools.remove(name) {
            if let Some(group) = self.groups.get_mut(&tool.group) {
                group.tool_names.retain(|n| n != name);
            }
        }
    }

    /// Look up a registered tool by name.
    pub fn get_tool(&self, name: &str) -> Option<&RegisteredTool> {
        self.tools.get(name)
    }

    /// Total number of registered tools, active or inactive.
    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }

    /// Append a middleware to the invocation chain.
    pub fn add_middleware(&mut self, middleware: Box<dyn ToolMiddleware>) {
        self.middlewares.push(middleware);
    }

    /// Set the provider-neutral event channel for streaming tool execution.
    pub fn set_event_tx(&mut self, tx: tokio::sync::mpsc::UnboundedSender<ToolTraceEvent>) {
        self.event_tx = Some(tx);
    }

    /// Track an external resource for cleanup when this toolkit is dropped.
    pub fn add_resource(&mut self, resource: Box<dyn ToolkitResource>) {
        self.resources.push(resource);
    }

    /// Register a group definition. Existing group entries are overwritten by
    /// name because the group descriptor is the authoritative state object.
    pub fn register_group(&mut self, group: ToolGroup) {
        self.groups.insert(group.name.clone(), group);
    }

    /// Activate or deactivate a group.
    ///
    /// The `"basic"` group cannot be deactivated; calls targeting it are
    /// intentionally ignored to preserve the default toolkit contract.
    pub fn set_group_active(&mut self, group_name: &str, active: bool) {
        if group_name == "basic" {
            return;
        }
        if let Some(group) = self.groups.get_mut(group_name) {
            group.active = active;
        }
    }
}

impl Default for Toolkit {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Toolkit {
    fn drop(&mut self) {
        for resource in self.resources.drain(..) {
            resource.close();
        }
    }
}
