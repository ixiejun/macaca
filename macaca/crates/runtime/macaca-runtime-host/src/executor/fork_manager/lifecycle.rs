//! Fork lifecycle transitions — create, suspend, resume, terminal states.
//!
//! Each public method logs trace/audit nodes and emits [`HookEvent`] observations
//! for downstream coordinator and SSE consumers.

use chrono::Utc;
use macaca_kernel::logging::{log_state_transition, LogContext};
use macaca_proto::{
    AcceptanceCriteria, ApplicationId, ForkId, ForkState, LlmMessage, TaskId, ValidationResult,
};
use tracing::{error, info, warn};

use super::manager::ForkManager;
use super::types::{DelegateResult, ForkContext, HookEvent};

impl ForkManager {
    /// Create a new fork.
    pub async fn create_fork(
        &self,
        parent_fork_id: Option<ForkId>,
        application_id: ApplicationId,
        agent_name: String,
        task_prompt: String,
        inherited_messages: Vec<LlmMessage>,
        system_prompt: String,
        acceptance_criteria: AcceptanceCriteria,
    ) -> Result<ForkId, String> {
        // Create log context for this operation
        let ctx = LogContext::new(&application_id.0.to_string()).with_agent_name(&agent_name);

        // Check fork limit
        let forks = self.forks.read().await;
        let active_count = forks.values().filter(|f| !f.is_terminal()).count();
        drop(forks);

        if active_count >= self.max_parallel_forks {
            error!(
                trace_id = %ctx.trace_id,
                app_id = %application_id.0,
                agent_name = %agent_name,
                active_count = active_count,
                max_limit = self.max_parallel_forks,
                "[FORK] Create failed: max parallel forks exceeded"
            );
            return Err(format!(
                "Maximum parallel forks ({}) exceeded",
                self.max_parallel_forks
            ));
        }

        // Create fork context (clone agent_name for event emission)
        let agent_name_for_event = agent_name.clone();
        let context = ForkContext::new(
            parent_fork_id,
            application_id,
            agent_name,
            task_prompt,
            inherited_messages,
            system_prompt,
            acceptance_criteria,
        );

        let fork_id = context.id;

        // Store fork in memory
        self.forks.write().await.insert(fork_id, context.clone());

        // Persist fork to store
        if let Some(ref store) = self.store {
            let key = format!("fork/{}/{}", self.app_id.0, fork_id.0);
            match serde_json::to_vec(&context) {
                Ok(bytes) => {
                    if let Err(e) = store.set(&key, &bytes).await {
                        warn!(fork_id = %fork_id.0, error = %e, "[FORK] Failed to persist fork");
                    }
                }
                Err(e) => {
                    warn!(fork_id = %fork_id.0, error = %e, "[FORK] Failed to serialize fork for persistence");
                }
            }
        }

        // Create fork-specific context for logging (clone the base context)
        let fork_ctx = LogContext::with_trace_id(&ctx.trace_id)
            .with_agent_name(agent_name_for_event.clone())
            .with_fork_id(&fork_id.0.to_string());

        // Log state transition
        log_state_transition(
            &fork_ctx,
            "ForkManager",
            "None",
            "Created",
            Some(
                [
                    (
                        "parent_fork_id".to_string(),
                        parent_fork_id.map(|f| f.0.to_string()).unwrap_or_default(),
                    ),
                    ("fork_id".to_string(), fork_id.0.to_string()),
                ]
                .into_iter()
                .filter(|(_, v)| !v.is_empty())
                .collect(),
            ),
        );

        // Emit created event
        let _ = self
            .hook_tx
            .send(HookEvent::ForkCreated {
                fork_id,
                application_id,
                agent_name: agent_name_for_event.clone(),
            })
            .await;

        info!(
            trace_id = %ctx.trace_id,
            app_id = %application_id.0,
            fork_id = %fork_id.0,
            agent_name = %agent_name_for_event,
            parent_fork_id = ?parent_fork_id,
            "[FORK] Created successfully"
        );

        Ok(fork_id)
    }

    /// Suspend a fork (waiting for delegate task to complete).
    pub async fn suspend_fork(
        &self,
        fork_id: ForkId,
        delegate_task_id: TaskId,
    ) -> Result<(), String> {
        let mut forks = self.forks.write().await;
        let fork = forks.get_mut(&fork_id).ok_or_else(|| {
            error!(fork_id = %fork_id.0, "[FORK] Suspend failed: fork not found");
            format!("Fork {} not found", fork_id)
        })?;

        if fork.state != ForkState::Running {
            warn!(
                fork_id = %fork_id.0,
                current_state = ?fork.state,
                "[FORK] Suspend failed: fork not running"
            );
            return Err(format!("Fork {} is not running", fork_id));
        }

        let agent_name = fork.agent_name.clone();
        fork.state = ForkState::WaitingForHook;
        fork.waiting_on_task = Some(delegate_task_id);

        // Log state transition
        info!(
            fork_id = %fork_id.0,
            task_id = %delegate_task_id.0,
            agent_name = %agent_name,
            from_state = "Running",
            to_state = "WaitingForHook",
            "[FORK] Suspended: waiting for delegate task"
        );

        // Emit waiting event
        let _ = self
            .hook_tx
            .send(HookEvent::ForkWaiting {
                fork_id,
                delegate_task_id,
            })
            .await;

        Ok(())
    }

    /// Resume a suspended fork after delegate task completes.
    pub async fn resume_fork(&self, fork_id: ForkId, result: DelegateResult) -> Result<(), String> {
        let mut forks = self.forks.write().await;
        let fork = forks
            .get_mut(&fork_id)
            .ok_or_else(|| format!("Fork {} not found", fork_id))?;

        if !matches!(fork.state, ForkState::WaitingForHook) {
            return Err(format!("Fork {} is not waiting", fork_id));
        }

        // Check if this result matches what fork was waiting for
        if fork.waiting_on_task != Some(result.task_id) {
            return Err("Task ID mismatch".to_string());
        }

        // Resume the fork
        fork.state = ForkState::Running;
        fork.waiting_on_task = None;

        // Add delegate result as a message in fork's conversation
        let result_message = if result.success {
            format!("[Delegate Completed]\n{}", result.output)
        } else {
            format!(
                "[Delegate Failed]\nError: {}\nOutput: {}",
                result.error.as_deref().unwrap_or("Unknown error"),
                result.output
            )
        };
        fork.own_messages.push(LlmMessage::user(result_message));

        // Collect artifacts (clone to avoid moving)
        fork.artifacts.extend(result.artifacts.clone());

        // Emit resumed event
        let _ = self
            .hook_tx
            .send(HookEvent::ForkResumed {
                fork_id,
                delegate_result: result,
            })
            .await;

        Ok(())
    }

    /// Start a fork (transition from Pending to Running).
    pub async fn start_fork(&self, fork_id: ForkId) -> Result<(), String> {
        let mut forks = self.forks.write().await;
        let fork = forks
            .get_mut(&fork_id)
            .ok_or_else(|| format!("Fork {} not found", fork_id))?;

        if fork.state != ForkState::Pending {
            return Err(format!(
                "Fork {} is not pending (current state: {:?})",
                fork_id, fork.state
            ));
        }

        fork.state = ForkState::Running;
        Ok(())
    }

    /// Resume a suspended fork by the task it's waiting on.
    /// This is used when a delegated task completes and we need to resume the waiting fork.
    ///
    /// This method:
    /// 1. Finds the fork waiting on this task
    /// 2. Validates the result against acceptance criteria
    /// 3. Emits DelegateCompleted/DelegateFailed hook events
    /// 4. Marks the fork as completed
    pub async fn resume_fork_by_task(
        &self,
        task_id: TaskId,
        result: DelegateResult,
    ) -> Result<ForkId, String> {
        let fork_id: ForkId;
        let validation_result: ValidationResult;

        {
            let mut forks = self.forks.write().await;

            // Find the fork waiting on this task
            let fork_entry = forks.iter_mut().find(|(_, fork)| {
                matches!(fork.state, ForkState::WaitingForHook)
                    && fork.waiting_on_task == Some(task_id)
            });

            let (fid, fork) =
                fork_entry.ok_or_else(|| format!("No fork waiting on task {}", task_id))?;

            fork_id = *fid;

            // Add delegate result as a message in fork's conversation
            let result_message = if result.success {
                format!("[Delegate Completed]\n{}", result.output)
            } else {
                format!(
                    "[Delegate Failed]\nError: {}\nOutput: {}",
                    result.error.as_deref().unwrap_or("Unknown error"),
                    result.output
                )
            };
            // Store as user message (context) and assistant message (output)
            fork.own_messages.push(LlmMessage::user(result_message));
            // Also store the output as an assistant message so it can be retrieved
            if result.success {
                fork.own_messages
                    .push(LlmMessage::assistant(result.output.clone()));
            }

            // Collect artifacts (clone to avoid moving)
            fork.artifacts.extend(result.artifacts.clone());

            // Validate the result against acceptance criteria
            validation_result = self.validate_result(fork);

            // Mark fork as completed
            fork.state = ForkState::Completed;
            fork.waiting_on_task = None;
            fork.completed_at = Some(Utc::now());

            info!(fork_id = %fork_id, task_id = %task_id, success = result.success, "Fork resumed and completed by task");
        }

        // Delete from store — terminal state
        if let Some(ref store) = self.store {
            let key = format!("fork/{}/{}", self.app_id.0, fork_id.0);
            if let Err(e) = store.delete(&key).await {
                warn!(fork_id = %fork_id.0, error = %e, "[FORK] Failed to delete completed fork from store");
            }
        }

        // Emit hook events (after releasing the lock)
        if result.success {
            // Emit DelegateCompleted event for coordinator notification
            self.emit_hook_event(HookEvent::DelegateCompleted {
                fork_id,
                task_id,
                success: true,
                output: result.output.clone(),
            });

            // If validation passed, emit ForkValidated event
            if matches!(validation_result, ValidationResult::Accepted) {
                self.emit_hook_event(HookEvent::ForkValidated {
                    fork_id,
                    result: validation_result,
                });
                info!(fork_id = %fork_id, "Fork validated successfully");
            } else {
                info!(fork_id = %fork_id, ?validation_result, "Fork validation result");
            }
        } else {
            // Emit DelegateFailed event
            self.emit_hook_event(HookEvent::DelegateFailed {
                fork_id,
                task_id,
                error: result
                    .error
                    .clone()
                    .unwrap_or_else(|| "Unknown error".to_string()),
            });
        }

        Ok(fork_id)
    }

    /// Mark a fork as completed.
    pub async fn complete_fork(&self, fork_id: ForkId) -> Result<(), String> {
        let mut forks = self.forks.write().await;
        let fork = forks
            .get_mut(&fork_id)
            .ok_or_else(|| format!("Fork {} not found", fork_id))?;

        fork.state = ForkState::Completed;
        fork.completed_at = Some(Utc::now());

        // Delete from store — terminal state, no need to restore
        if let Some(ref store) = self.store {
            let key = format!("fork/{}/{}", self.app_id.0, fork_id.0);
            if let Err(e) = store.delete(&key).await {
                warn!(fork_id = %fork_id.0, error = %e, "[FORK] Failed to delete completed fork from store");
            }
        }

        Ok(())
    }

    /// Mark a fork as failed.
    pub async fn fail_fork(&self, fork_id: ForkId, error: String) -> Result<(), String> {
        let mut forks = self.forks.write().await;
        let fork = forks
            .get_mut(&fork_id)
            .ok_or_else(|| format!("Fork {} not found", fork_id))?;

        fork.state = ForkState::Failed { error };
        fork.completed_at = Some(Utc::now());

        // Delete from store — terminal state, no need to restore
        if let Some(ref store) = self.store {
            let key = format!("fork/{}/{}", self.app_id.0, fork_id.0);
            if let Err(e) = store.delete(&key).await {
                warn!(fork_id = %fork_id.0, error = %e, "[FORK] Failed to delete failed fork from store");
            }
        }

        Ok(())
    }
}
