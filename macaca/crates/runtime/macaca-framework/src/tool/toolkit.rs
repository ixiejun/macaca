//! Central tool registry and invocation pipeline (**Registry** + **Chain of Responsibility**).
//!
//! [`Toolkit`] is the single entry point for registering tools, toggling groups, attaching
//! middleware, and executing calls. The `call_tool` pipeline is fixed-order and auditable:
//! lookup → group policy → preset merge → middleware before → handler → middleware after.

use std::collections::HashMap;

use serde_json::Value;

use super::error::ToolError;
use super::response::ToolResponse;
use super::traits::{ToolHandler, ToolMiddleware, ToolkitResource};
use super::types::{RegisteredTool, ToolGroup};

/// Central registry for tools, groups, and middleware (**Registry** pattern).
///
/// # Group semantics
///
/// Every tool belongs to exactly one group. The built-in `"basic"` group is
/// **always active** and cannot be deactivated. Other groups start active by
/// default but can be toggled with [`set_group_active`](Self::set_group_active).
///
/// # Execution order
///
/// [`call_tool`](Self::call_tool) runs:
/// 1. Merge `preset_args` into caller args
/// 2. All `middleware.before()` in insertion order
/// 3. `handler.execute(args)` (or streaming variant when `macaca-compat` is enabled)
/// 4. All `middleware.after()` in insertion order
/// 5. Return [`ToolResponse`]
pub struct Toolkit {
    /// Name → registered handler entry. Exposed to contract tests for preset-arg injection.
    pub(crate) tools: HashMap<String, RegisteredTool>,
    /// Group name → activation metadata. Exposed to contract tests for group assertions.
    pub(crate) groups: HashMap<String, ToolGroup>,
    middlewares: Vec<Box<dyn ToolMiddleware>>,
    resources: Vec<Box<dyn ToolkitResource>>,
    /// Optional event channel for streaming tool execution under `macaca-compat`.
    #[cfg(feature = "macaca-compat")]
    event_tx: Option<tokio::sync::mpsc::UnboundedSender<macaca_tools::TraceEvent>>,
}

impl Toolkit {
    /// Create an empty toolkit. The `"basic"` group is pre-registered and always active.
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
        tracing::debug!(
            target: "macaca_framework::tool::toolkit",
            group_count = groups.len(),
            "toolkit initialized with default basic group"
        );
        Self {
            tools: HashMap::new(),
            groups,
            middlewares: Vec::new(),
            resources: Vec::new(),
            #[cfg(feature = "macaca-compat")]
            event_tx: None,
        }
    }

    /// Register a tool into the given group (defaults to `"basic"`).
    ///
    /// If the group does not yet exist it is created and marked active.
    /// If a tool with the same name already exists it is replaced in place.
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

        tracing::debug!(
            target: "macaca_framework::tool::toolkit",
            tool_name = %tool_name,
            group = %group_name,
            "toolkit.register stored handler"
        );

        self.tools.insert(
            tool_name,
            RegisteredTool {
                handler,
                group: group_name,
                preset_args: Value::Object(Default::default()),
            },
        );
    }

    /// Remove a tool by name and detach it from its group's `tool_names` list.
    pub fn unregister(&mut self, name: &str) {
        if let Some(tool) = self.tools.remove(name) {
            if let Some(group) = self.groups.get_mut(&tool.group) {
                group.tool_names.retain(|n| n != name);
            }
            tracing::debug!(
                target: "macaca_framework::tool::toolkit",
                tool_name = %name,
                group = %tool.group,
                "toolkit.unregister removed handler"
            );
        } else {
            tracing::debug!(
                target: "macaca_framework::tool::toolkit",
                tool_name = %name,
                "toolkit.unregister no-op; tool not found"
            );
        }
    }

    /// Look up a registered tool by name without executing it.
    pub fn get_tool(&self, name: &str) -> Option<&RegisteredTool> {
        self.tools.get(name)
    }

    /// Total number of registered tools (active or not).
    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }

    /// Append a middleware to the chain (runs in insertion order for both before and after).
    pub fn add_middleware(&mut self, middleware: Box<dyn ToolMiddleware>) {
        let index = self.middlewares.len();
        self.middlewares.push(middleware);
        tracing::debug!(
            target: "macaca_framework::tool::toolkit",
            middleware_index = index,
            middleware_count = self.middlewares.len(),
            "toolkit.add_middleware appended chain link"
        );
    }

    /// Set the event channel for streaming tool execution (`macaca-compat` only).
    #[cfg(feature = "macaca-compat")]
    pub fn set_event_tx(
        &mut self,
        tx: tokio::sync::mpsc::UnboundedSender<macaca_tools::TraceEvent>,
    ) {
        tracing::debug!(
            target: "macaca_framework::tool::toolkit",
            "toolkit.set_event_tx attached trace event channel"
        );
        self.event_tx = Some(tx);
    }

    /// Track an external resource for cleanup when this toolkit is dropped.
    pub fn add_resource(&mut self, resource: Box<dyn ToolkitResource>) {
        tracing::debug!(
            target: "macaca_framework::tool::toolkit",
            resource_count = self.resources.len() + 1,
            "toolkit.add_resource registered lifecycle hook"
        );
        self.resources.push(resource);
    }

    /// Register a group definition. Existing group entries are preserved (insert semantics).
    pub fn register_group(&mut self, group: ToolGroup) {
        tracing::debug!(
            target: "macaca_framework::tool::toolkit",
            group = %group.name,
            active = group.active,
            tool_count = group.tool_names.len(),
            "toolkit.register_group upserted group metadata"
        );
        self.groups.insert(group.name.clone(), group);
    }

    /// Activate or deactivate a group.
    ///
    /// The `"basic"` group **cannot** be deactivated; calls targeting it are silently ignored.
    pub fn set_group_active(&mut self, group_name: &str, active: bool) {
        if group_name == "basic" {
            tracing::debug!(
                target: "macaca_framework::tool::toolkit",
                group = %group_name,
                requested_active = active,
                "toolkit.set_group_active ignored for immutable basic group"
            );
            return;
        }
        if let Some(group) = self.groups.get_mut(group_name) {
            group.active = active;
            tracing::debug!(
                target: "macaca_framework::tool::toolkit",
                group = %group_name,
                active,
                "toolkit.set_group_active updated group policy"
            );
        } else {
            tracing::warn!(
                target: "macaca_framework::tool::toolkit",
                group = %group_name,
                active,
                "toolkit.set_group_active no-op; unknown group"
            );
        }
    }

    /// Invoke a tool by name through the full middleware pipeline.
    ///
    /// Returns [`ToolError::NotFound`] if the tool does not exist.
    /// Returns [`ToolError::PermissionDenied`] if the tool's group is inactive.
    pub async fn call_tool(&self, name: &str, args: Value) -> Result<ToolResponse, ToolError> {
        tracing::debug!(
            target: "macaca_framework::tool::toolkit",
            tool_name = %name,
            middleware_count = self.middlewares.len(),
            "toolkit.call_tool begin invocation pipeline"
        );

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
            tracing::warn!(
                target: "macaca_framework::tool::toolkit",
                tool_name = %name,
                group = %registered.group,
                reason_code = "permission_denied_inactive_group",
                "toolkit.call_tool rejected inactive group"
            );
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

        for (index, mw) in self.middlewares.iter().enumerate() {
            if let Err(err) = mw.before(name, &mut effective_args).await {
                tracing::warn!(
                    target: "macaca_framework::tool::toolkit",
                    tool_name = %name,
                    middleware_index = index,
                    error = %err,
                    reason_code = "middleware_before_failed",
                    "toolkit.call_tool short-circuited before handler"
                );
                return Err(err);
            }
        }

        #[cfg(feature = "macaca-compat")]
        let mut response = {
            registered
                .handler
                .execute_streaming(effective_args, self.event_tx.clone())
                .await?
        };
        #[cfg(not(feature = "macaca-compat"))]
        let mut response = registered.handler.execute(effective_args).await?;

        for (index, mw) in self.middlewares.iter().enumerate() {
            if let Err(err) = mw.after(name, &mut response).await {
                tracing::warn!(
                    target: "macaca_framework::tool::toolkit",
                    tool_name = %name,
                    middleware_index = index,
                    error = %err,
                    reason_code = "middleware_after_failed",
                    "toolkit.call_tool failed during after chain"
                );
                return Err(err);
            }
        }

        tracing::debug!(
            target: "macaca_framework::tool::toolkit",
            tool_name = %name,
            content_blocks = response.content.len(),
            is_stream = response.is_stream,
            "toolkit.call_tool completed successfully"
        );

        Ok(response)
    }

    /// Return tool definitions for all active tools in all active groups.
    ///
    /// Format: `[{"name": "...", "description": "...", "parameters": {schema}}]`
    pub fn get_definitions(&self) -> Vec<Value> {
        let defs: Vec<Value> = self
            .tools
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
            .collect();

        tracing::debug!(
            target: "macaca_framework::tool::toolkit",
            definition_count = defs.len(),
            registered_tool_count = self.tools.len(),
            "toolkit.get_definitions materialized LLM tool schema list"
        );

        defs
    }
}

impl Default for Toolkit {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Toolkit {
    fn drop(&mut self) {
        let resource_count = self.resources.len();
        for resource in self.resources.drain(..) {
            resource.close();
        }
        if resource_count > 0 {
            tracing::debug!(
                target: "macaca_framework::tool::toolkit",
                resource_count,
                "toolkit.drop closed registered external resources"
            );
        }
    }
}
