//! Entrypoint and workflow declaration types for application manifests.
//!
//! Workflow graphs are declarative: the Application Framework interprets step
//! dependencies and agent bindings without hard-coding application-specific
//! orchestration logic in the OS layer.

use serde::{Deserialize, Serialize};

/// Entry point type for an application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntrypointType {
    /// Execute a named workflow.
    Workflow,
    /// Invoke a specific agent directly.
    Agent,
    /// Custom entry point (future extensibility).
    Custom,
}

impl Default for EntrypointType {
    fn default() -> Self {
        Self::Workflow
    }
}

/// Configuration for the application entry point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntrypointConfig {
    /// Type of entry point.
    #[serde(default, rename = "type")]
    pub type_: EntrypointType,
    /// Name of the workflow or agent to invoke.
    pub name: String,
}

/// A single step in a workflow definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    /// Step name/identifier.
    pub name: String,
    /// Agent ID to execute this step.
    pub agent: String,
    /// Optional prompt template file or inline prompt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_template: Option<String>,
    /// Names of steps that must complete before this step.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
}

/// Definition of a workflow that can be executed by the application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    /// Human-readable description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Ordered list of steps to execute.
    pub steps: Vec<WorkflowStep>,
}

/// Resource path configuration for an application.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceConfig {
    /// Path to personas directory (relative to app root).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub personas: Option<String>,
    /// Path to skills directory (relative to app root).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skills: Option<String>,
    /// Path to prompts directory (relative to app root).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompts: Option<String>,
    /// Path to workflows directory (relative to app root).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflows: Option<String>,
}
