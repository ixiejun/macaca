//! Registry value types for tool groups and registered entries.
//!
//! [`ToolGroup`] models activation policy at the group level (e.g. enable/disable a
//! family of tools). [`RegisteredTool`] binds a [`super::traits::ToolHandler`] to its
//! group and optional preset arguments merged on every invocation.

use serde_json::Value;

use super::traits::ToolHandler;

/// A named group of tools that can be activated or deactivated as a unit.
///
/// The built-in `"basic"` group is always active and cannot be deactivated; see
/// [`super::toolkit::Toolkit::set_group_active`].
#[derive(Debug, Clone)]
pub struct ToolGroup {
    /// Group identifier (e.g. "basic", "file_io", "web").
    pub name: String,
    /// Names of tools belonging to this group.
    pub tool_names: Vec<String>,
    /// Whether this group is currently active.
    pub active: bool,
}

/// A tool stored inside the [`super::toolkit::Toolkit`], together with its group and preset args.
pub struct RegisteredTool {
    /// The concrete handler (**Strategy** implementation).
    pub handler: Box<dyn ToolHandler>,
    /// The group this tool belongs to.
    pub group: String,
    /// Arguments merged into every call before middleware runs.
    ///
    /// Preset args have lower priority than caller-supplied args — the caller's values
    /// win on key conflicts during [`super::toolkit::Toolkit::call_tool`].
    pub preset_args: Value,
}
