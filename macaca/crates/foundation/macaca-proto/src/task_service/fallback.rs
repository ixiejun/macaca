//! Provider-neutral fallback decomposition DTOs.
//!
//! These DTOs move capability-based fallback planning behind the Task Service
//! boundary.  Shells may request a fallback plan when the planner service is
//! unavailable or incomplete, but the shell must not own keyword matching,
//! phase ordering, task titles, or acceptance criteria.  The Task Service can
//! replace the internal strategy later without changing Web or CLI adapters.

use serde::{Deserialize, Serialize};

use crate::{ApplicationId, TaskId, TraceContext};

/// Worker dossier supplied by a shell or application adapter.
///
/// The fields are deliberately generic: `name` is an agent identity supplied by
/// the manifest/application boundary, while `capabilities` are declarative
/// descriptors.  The Task Service strategy may inspect descriptor text, but the
/// OS layer must not infer application-specific role names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FallbackDecompositionWorkerProfile {
    pub name: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

/// Command to build a fallback task assignment plan.
///
/// This command is pure planning. It does not mutate the task board and does
/// not call an LLM. The caller creates tasks through `task.create_assignment`
/// so trace/audit records remain explicit for every generated task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildFallbackDecompositionCommand {
    pub app_id: ApplicationId,
    pub session_id: Option<String>,
    pub goal_id: TaskId,
    pub goal_description: String,
    #[serde(default)]
    pub workers: Vec<FallbackDecompositionWorkerProfile>,
    pub initial_dependency: Option<TaskId>,
    pub planner_error: String,
    pub trace: Option<TraceContext>,
}

/// Stable phase labels for fallback assignment specs.
///
/// The enum is serialized as snake_case strings so audit output stays compact
/// and independent from Rust variant names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FallbackTaskPhase {
    Research,
    Analyze,
    Produce,
    Validate,
    Finalize,
    Execute,
}

/// One task assignment that the caller may persist through Task Service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FallbackTaskAssignmentSpec {
    pub agent_name: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,
    pub priority: u8,
    #[serde(default)]
    pub depends_on: Vec<TaskId>,
    pub phase: FallbackTaskPhase,
}

/// Pure fallback decomposition result returned by Task Service.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FallbackDecompositionPlan {
    #[serde(default)]
    pub assignments: Vec<FallbackTaskAssignmentSpec>,
}
