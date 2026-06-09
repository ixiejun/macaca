//! Execution-control Adapter: policy-driven tool-call barriers with in-process resume.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use async_trait::async_trait;
use macaca_sdk::framework::tool::{ToolError, ToolMiddleware, ToolResponse};
use macaca_proto::{ExecutionControlPolicy, ExecutionControlTrigger};
use tokio::sync::{mpsc, Mutex};
use crate::runtime_resume::RuntimeResumeSignal;
pub struct ExecutionControlMiddleware {
    pub(crate) pause_signal: Arc<AtomicBool>,
    pub(crate) resume_rx: Arc<Mutex<mpsc::Receiver<RuntimeResumeSignal>>>,
    pub(crate) policy: ExecutionControlPolicy,
    pub(crate) execution_id: String,
}

impl ExecutionControlMiddleware {
    /// Return whether the policy declares a tool-call barrier for this tool.
    pub(crate) fn policy_pauses_after_tool(policy: &ExecutionControlPolicy, tool_name: &str) -> bool {
        policy.triggers.iter().any(|trigger| {
            matches!(
                trigger,
                ExecutionControlTrigger::ToolCallBarrier { tool_name: configured }
                    if configured == tool_name
            )
        })
    }
}

#[async_trait]
impl ToolMiddleware for ExecutionControlMiddleware {
    async fn before(&self, _name: &str, _args: &mut serde_json::Value) -> Result<(), ToolError> {
        Ok(())
    }

    async fn after(&self, name: &str, response: &mut ToolResponse) -> Result<(), ToolError> {
        if !Self::policy_pauses_after_tool(&self.policy, name) {
            return Ok(());
        }

        tracing::info!(
            execution_id = %self.execution_id,
            tool = %name,
            "execution control barrier reached; pausing runtime execution"
        );
        self.pause_signal.store(true, Ordering::SeqCst);

        // Wait for the configured resume source to complete.  Autonomous goals
        // and delegated work can legitimately run longer than a fixed HTTP-era
        // timeout; ending this wait early loses the paused execution.
        let mut rx = self.resume_rx.lock().await;
        match rx.recv().await {
            Some(reason) => {
                self.pause_signal.store(false, Ordering::SeqCst);
                let context = match &reason {
                    RuntimeResumeSignal::DelegateCompleted { output, .. } => output.clone(),
                    _ => "Goal processing completed.".to_string(),
                };
                response
                    .content
                    .push(macaca_sdk::framework::message::ContentBlock::Text(
                        macaca_sdk::framework::message::TextBlock {
                            text: format!("\n\n[Goal completed: {}]", context),
                        },
                    ));
                tracing::info!(
                    execution_id = %self.execution_id,
                    tool = %name,
                    "execution control resume signal delivered"
                );
            }
            None => {
                self.pause_signal.store(false, Ordering::SeqCst);
                tracing::warn!(
                    execution_id = %self.execution_id,
                    tool = %name,
                    "execution control resume channel closed"
                );
            }
        }
        Ok(())
    }
}
