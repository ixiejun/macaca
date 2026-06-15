//! Batch `create_todos` tool — Composite over [`CreateTodoTool`].
//!
//! Optimized for planner decomposition: persist the whole task graph in one ReAct action.

use async_trait::async_trait;
use macaca_proto::MacacaResult;
use serde_json::{json, Value};

use crate::tool::{Tool, ToolCommand};

use super::create_todo::CreateTodoTool;

/// Create multiple todos in one tool call.
///
/// This is optimized for planner decomposition: the planner can persist the
/// whole task graph in a single ReAct action instead of spending one LLM round
/// trip per task.
pub struct CreateTodosTool {
    pub create_todo: CreateTodoTool,
}

#[async_trait]
impl Tool for CreateTodosTool {
    fn name(&self) -> &str {
        "create_todos"
    }

    fn description(&self) -> &str {
        "Create multiple tasks and assign each to a specific agent's task board in one call. Preferred for goal decomposition."
    }

    fn tool_schema(&self) -> Value {
        let item_schema = crate::tool::ToolSchemaProvider::tool_schema(&self.create_todo);
        json!({
            "type": "object",
            "properties": {
                "tasks": {
                    "type": "array",
                    "minItems": 1,
                    "description": "Tasks to create, in dependency order. Each item uses the same schema as create_todo.",
                    "items": item_schema
                }
            },
            "required": ["tasks"]
        })
    }

    async fn invoke(&self, command: ToolCommand) -> MacacaResult<Value> {
        let input = command.input;
        let tasks = input["tasks"].as_array().ok_or_else(|| {
            macaca_proto::MacacaError::Task("missing required field: tasks".into())
        })?;
        if tasks.is_empty() {
            return Err(macaca_proto::MacacaError::Task(
                "tasks must contain at least one task".into(),
            ));
        }

        let mut created = Vec::with_capacity(tasks.len());
        for task in tasks {
            created.push(self.create_todo.create_one(task.clone()).await?);
        }

        Ok(json!({
            "count": created.len(),
            "created": created,
        }))
    }
}
