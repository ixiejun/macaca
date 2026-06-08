//! Workflow engine (**Template Method** + **Strategy** module tree).
//!
//! Executes declarative workflows from application manifests.  Prompt assembly
//! uses pluggable [`WorkflowPromptStrategy`] implementations while coordinator
//! resolution follows a fail-closed Chain of Responsibility over manifest fields.
//!
//! # Module tree (P5 iteration 100)
//! - `types` — DTOs, constants, coordinator tool-policy adapter
//! - `prompt_strategy` — Strategy trait, default SDD prompt, tool-policy renderer
//! - `engine` — [`WorkflowEngine`] coordinator resolution and prompt assembly
//! - `tests` — contract tests with neutral `fixture-*` identifiers

mod engine;
mod prompt_strategy;
mod types;

#[cfg(test)]
mod tests;

pub use engine::{WorkflowContext, WorkflowEngine, WorkflowResult};
pub use prompt_strategy::{DefaultWorkflowPromptStrategy, WorkflowPromptStrategy};
pub use types::{WorkflowPromptContext, WorkflowPromptParts, DEFAULT_WORKFLOW};
