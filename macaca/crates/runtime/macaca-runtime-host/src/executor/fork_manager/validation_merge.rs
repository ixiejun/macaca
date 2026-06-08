//! Fork validation and parent merge — Specification + Memento merge path.

use chrono::Utc;
use macaca_proto::{ForkId, ForkState, ValidationResult};
use tracing::warn;

use super::manager::ForkManager;
use super::types::{ForkContext, HookEvent, MergeResult};

impl ForkManager {
    /// Validate fork result against acceptance criteria.
    pub fn validate_result(&self, fork: &ForkContext) -> ValidationResult {
        let criteria = &fork.acceptance_criteria;

        // Auto-accept if enabled
        if criteria.auto_accept {
            return ValidationResult::Accepted;
        }

        // Check required artifacts
        for artifact_path in &criteria.required_artifacts {
            if !fork.artifacts.iter().any(|a| a.contains(artifact_path)) {
                return ValidationResult::Rejected {
                    reason: format!("Required artifact not found: {}", artifact_path),
                };
            }
        }

        // Check if there's any output
        let has_output = fork.own_messages.iter().any(|m| !m.content.is_empty());

        if !has_output {
            return ValidationResult::Rejected {
                reason: "No output produced".to_string(),
            };
        }

        // Basic validation passed
        ValidationResult::Accepted
    }

    /// Merge a completed fork back to parent.
    pub async fn merge_fork(&self, fork_id: ForkId) -> Result<MergeResult, String> {
        let mut forks = self.forks.write().await;
        let fork = forks
            .get_mut(&fork_id)
            .ok_or_else(|| format!("Fork {} not found", fork_id))?;

        if fork.state != ForkState::Completed {
            return Err(format!("Fork {} is not completed", fork_id));
        }

        // Validate result
        let validation = self.validate_result(&fork);
        if !matches!(validation, ValidationResult::Accepted) {
            return Err(format!("Validation failed: {:?}", validation));
        }

        // Generate summary message
        let summary = Self::generate_summary(&fork);

        // Mark as merged
        fork.state = ForkState::Merged;
        fork.completed_at = Some(Utc::now());

        // Delete from store — terminal state
        if let Some(ref store) = self.store {
            let key = format!("fork/{}/{}", self.app_id.0, fork_id.0);
            if let Err(e) = store.delete(&key).await {
                warn!(fork_id = %fork_id.0, error = %e, "[FORK] Failed to delete merged fork from store");
            }
        }

        // Emit merged event
        let _ = self.hook_tx.send(HookEvent::ForkMerged { fork_id }).await;

        Ok(MergeResult {
            fork_id,
            summary_message: summary,
            artifacts: fork.artifacts.clone(),
        })
    }

    /// Generate a summary message from fork's conversation.
    fn generate_summary(fork: &ForkContext) -> String {
        let mut summary = String::new();
        summary.push_str(&format!("[Fork {} Summary]\n", fork.id));
        summary.push_str(&format!("Agent: {}\n", fork.agent_name));
        summary.push_str(&format!(
            "Status: {}\n",
            match fork.state {
                ForkState::Completed => "Completed",
                ForkState::Failed { .. } => "Failed",
                _ => "Unknown",
            }
        ));

        if !fork.artifacts.is_empty() {
            summary.push_str("\nArtifacts:\n");
            for artifact in &fork.artifacts {
                summary.push_str(&format!("- {}\n", artifact));
            }
        }

        // Add final output preview
        if let Some(last_msg) = fork.own_messages.last() {
            let preview = last_msg.content.chars().take(500).collect::<String>();
            summary.push_str(&format!("\nFinal Output:\n{}", preview));
        }

        summary
    }
}
