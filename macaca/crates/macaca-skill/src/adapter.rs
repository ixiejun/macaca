//! Executable skill tool adapter and runtime proxy.

use async_trait::async_trait;
use serde_json::Value;

use macaca_proto::{MacacaError, MacacaResult};

use crate::definition::{SkillDefinition, SkillEntryPoint};
use crate::tool::execute_shell_entry;

/// Runtime proxy for executable skill definitions.
#[async_trait]
pub trait SkillRuntimeProxy: Send + Sync {
    async fn execute(&self, definition: &SkillDefinition, input: Value) -> MacacaResult<Value>;
}

/// Local shell/script runtime proxy.
#[derive(Debug, Clone, Default)]
pub struct LocalSkillRuntimeProxy;

#[async_trait]
impl SkillRuntimeProxy for LocalSkillRuntimeProxy {
    async fn execute(&self, definition: &SkillDefinition, input: Value) -> MacacaResult<Value> {
        match &definition.entry_point {
            SkillEntryPoint::ShellCommand { command, args } => {
                execute_shell_entry(command, args, &input).await
            }
            SkillEntryPoint::Script { path, interpreter } => {
                let cmd = interpreter.as_deref().unwrap_or("sh");
                execute_shell_entry(cmd, &[path.clone()], &input).await
            }
            SkillEntryPoint::McpServer { .. } => Err(MacacaError::Agent(
                "MCP skills should be loaded via McpDriver, not SkillTool".into(),
            )),
        }
    }
}

/// Adapter that exposes an executable skill definition through a runtime proxy.
#[derive(Debug, Clone)]
pub struct SkillToolAdapter {
    definition: SkillDefinition,
    runtime: LocalSkillRuntimeProxy,
}

impl SkillToolAdapter {
    pub fn local(definition: SkillDefinition) -> Self {
        Self {
            definition,
            runtime: LocalSkillRuntimeProxy,
        }
    }

    pub fn definition(&self) -> &SkillDefinition {
        &self.definition
    }

    pub async fn execute(&self, input: Value) -> MacacaResult<Value> {
        self.runtime.execute(&self.definition, input).await
    }
}
