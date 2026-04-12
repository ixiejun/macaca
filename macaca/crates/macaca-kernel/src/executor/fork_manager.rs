//! Fork Manager - Manages lifecycle of forked (child) agents.
//!
//! The ForkManager handles:
//! - Creating forks that inherit parent agent's context
//! - Suspending forks when they delegate tasks
//! - Resuming forks when delegate tasks complete via hooks
//! - Validating fork results against acceptance criteria
//! - Merging completed forks back to parent

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use macaca_persist::PersistStore;

use crate::logging::{log_hook_event, log_state_transition, LogContext};
use macaca_proto::{
    AcceptanceCriteria, ApplicationId, ForkId, ForkState, LlmMessage, TaskId, ValidationResult,
};

/// Maximum number of concurrent forks per application.
const MAX_PARALLEL_FORKS: usize = 10;

/// Maximum time to wait for a delegate task to complete.
const DEFAULT_DELEGATE_TIMEOUT_SECS: u64 = 300;

/// Maximum number of messages to inherit from parent.
const MAX_INHERITED_MESSAGES: usize = 10;

/// Callback for hook events.
pub type HookCallback =
    Box<dyn Fn(HookEvent) -> futures::future::BoxFuture<'static, ()> + Send + Sync>;

/// Event emitted when a fork's state changes.
#[derive(Debug, Clone)]
pub enum HookEvent {
    /// Fork created.
    ForkCreated {
        fork_id: ForkId,
        application_id: ApplicationId,
        agent_name: String,
    },
    /// Fork suspended waiting for delegate task.
    ForkWaiting {
        fork_id: ForkId,
        delegate_task_id: TaskId,
    },
    /// Fork resumed after delegate task completed.
    ForkResumed {
        fork_id: ForkId,
        delegate_result: DelegateResult,
    },
    /// Fork's delegate task completed.
    DelegateCompleted {
        fork_id: ForkId,
        task_id: TaskId,
        success: bool,
        output: String,
    },
    /// Fork's delegate task failed.
    DelegateFailed {
        fork_id: ForkId,
        task_id: TaskId,
        error: String,
    },
    /// Fork validated and ready to merge.
    ForkValidated {
        fork_id: ForkId,
        result: ValidationResult,
    },
    /// Fork merged to parent.
    ForkMerged { fork_id: ForkId },
}

/// Result from a delegated task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegateResult {
    pub task_id: TaskId,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub artifacts: Vec<String>,
}

/// Context for a forked agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkContext {
    /// Unique fork identifier.
    pub id: ForkId,
    /// Parent fork ID (None for root/main agent).
    pub parent_fork_id: Option<ForkId>,
    /// Application ID this fork belongs to.
    pub application_id: ApplicationId,
    /// Name of the agent running this fork.
    pub agent_name: String,
    /// Current lifecycle state.
    pub state: ForkState,
    /// Inherited conversation history from parent (last N messages).
    pub inherited_messages: Vec<LlmMessage>,
    /// Conversation history generated during this fork's execution.
    pub own_messages: Vec<LlmMessage>,
    /// System prompt (inherited from parent).
    pub system_prompt: String,
    /// Original task prompt.
    pub task_prompt: String,
    /// Acceptance criteria for validation.
    pub acceptance_criteria: AcceptanceCriteria,
    /// Task ID this fork is waiting on (if any).
    pub waiting_on_task: Option<TaskId>,
    /// Artifacts produced by this fork.
    pub artifacts: Vec<String>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Completion timestamp (if completed).
    pub completed_at: Option<DateTime<Utc>>,
    /// Timeout for delegate tasks (stored as seconds for serialization).
    #[serde(with = "duration_secs")]
    pub delegate_timeout: Duration,
}

impl ForkContext {
    /// Create a new fork context.
    pub fn new(
        parent_fork_id: Option<ForkId>,
        application_id: ApplicationId,
        agent_name: String,
        task_prompt: String,
        inherited_messages: Vec<LlmMessage>,
        system_prompt: String,
        acceptance_criteria: AcceptanceCriteria,
    ) -> Self {
        // Keep only the last N messages to limit context size
        let inherited_messages = inherited_messages
            .into_iter()
            .rev()
            .take(MAX_INHERITED_MESSAGES)
            .rev()
            .collect();

        Self {
            id: ForkId::new(),
            parent_fork_id,
            application_id,
            agent_name,
            state: ForkState::Pending,
            inherited_messages,
            own_messages: vec![],
            system_prompt,
            task_prompt,
            acceptance_criteria,
            waiting_on_task: None,
            artifacts: vec![],
            created_at: Utc::now(),
            completed_at: None,
            delegate_timeout: Duration::from_secs(DEFAULT_DELEGATE_TIMEOUT_SECS),
        }
    }

    /// Check if the fork is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.state,
            ForkState::Completed
                | ForkState::Failed { .. }
                | ForkState::Merged
                | ForkState::Cancelled
        )
    }
}

/// Result from merging a fork back to parent.
#[derive(Debug, Clone)]
pub struct MergeResult {
    pub fork_id: ForkId,
    /// Summary message to append to parent's conversation.
    pub summary_message: String,
    pub artifacts: Vec<String>,
}

/// Fork Manager - manages lifecycle of all forks.
pub struct ForkManager {
    /// All active forks.
    forks: Arc<RwLock<HashMap<ForkId, ForkContext>>>,
    /// Maximum parallel forks allowed.
    max_parallel_forks: usize,
    /// Channel for hook event callbacks.
    hook_tx: mpsc::Sender<HookEvent>,
    /// Broadcast sender for hook events (multiple subscribers).
    hook_broadcast: tokio::sync::broadcast::Sender<HookEvent>,
    /// Optional persistence store for durability across restarts.
    store: Option<Arc<macaca_persist::RedbStore>>,
    /// Application ID used for key namespacing in the store.
    app_id: macaca_proto::ApplicationId,
}

impl ForkManager {
    /// Create a new ForkManager.
    pub fn new() -> Self {
        let (hook_tx, _hook_rx) = mpsc::channel(100);
        let (hook_broadcast, _) = tokio::sync::broadcast::channel(256);
        Self {
            forks: Arc::new(RwLock::new(HashMap::new())),
            max_parallel_forks: MAX_PARALLEL_FORKS,
            hook_tx,
            hook_broadcast,
            store: None,
            app_id: macaca_proto::ApplicationId::new(),
        }
    }

    /// Create a new ForkManager with optional persistence.
    pub fn new_with_store(
        store: Option<Arc<macaca_persist::RedbStore>>,
        app_id: macaca_proto::ApplicationId,
    ) -> Self {
        let (hook_tx, _hook_rx) = mpsc::channel(100);
        let (hook_broadcast, _) = tokio::sync::broadcast::channel(256);
        Self {
            forks: Arc::new(RwLock::new(HashMap::new())),
            max_parallel_forks: MAX_PARALLEL_FORKS,
            hook_tx,
            hook_broadcast,
            store,
            app_id,
        }
    }

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

    /// Get a fork by ID.
    pub async fn get_fork(&self, fork_id: ForkId) -> Option<ForkContext> {
        self.forks.read().await.get(&fork_id).cloned()
    }

    /// List all forks for an application.
    pub async fn list_forks(&self, application_id: &ApplicationId) -> Vec<ForkContext> {
        self.forks
            .read()
            .await
            .values()
            .filter(|f| f.application_id == *application_id)
            .cloned()
            .collect()
    }

    /// Subscribe to hook events (broadcast channel).
    ///
    /// Multiple subscribers can receive all hook events for:
    /// - Monitoring fork lifecycle
    /// - Notifying coordinators when delegated tasks complete
    /// - Validation feedback
    pub fn subscribe_to_hooks(&self) -> tokio::sync::broadcast::Receiver<HookEvent> {
        self.hook_broadcast.subscribe()
    }

    /// Emit a hook event to both internal channel and broadcast subscribers.
    fn emit_hook_event(&self, event: HookEvent) {
        // Send to broadcast channel (don't await, just try)
        let _ = self.hook_broadcast.send(event.clone());
    }

    /// Restore non-terminal forks from the persistence store after a restart.
    pub async fn restore_forks(&mut self) {
        let store = match &self.store {
            Some(s) => Arc::clone(s),
            None => return,
        };

        let prefix = format!("fork/{}/", self.app_id.0);
        let keys = match store.list_keys(&prefix).await {
            Ok(k) => k,
            Err(e) => {
                error!(error = %e, "[FORK] Failed to list fork keys during restore");
                return;
            }
        };

        let mut forks = self.forks.write().await;
        let mut restored = 0usize;

        for key in &keys {
            let bytes = match store.get(key).await {
                Ok(Some(b)) => b,
                Ok(None) => continue,
                Err(e) => {
                    warn!(key = %key, error = %e, "[FORK] Failed to read fork during restore");
                    continue;
                }
            };

            match serde_json::from_slice::<ForkContext>(&bytes) {
                Ok(fork) => {
                    // Skip terminal forks — they don't need restoring
                    if fork.is_terminal() {
                        // Clean up stale terminal entries from store
                        if let Err(e) = store.delete(key).await {
                            warn!(key = %key, error = %e, "[FORK] Failed to clean up terminal fork from store");
                        }
                        continue;
                    }
                    forks.insert(fork.id, fork);
                    restored += 1;
                }
                Err(e) => {
                    warn!(key = %key, error = %e, "[FORK] Failed to deserialize fork during restore");
                }
            }
        }

        if restored > 0 {
            info!(restored = restored, app_id = %self.app_id.0, "[FORK] Restored forks from store");
        }
    }

    /// Clean up old merged forks.
    pub async fn cleanup(&self, older_than: Duration) -> usize {
        use chrono::TimeDelta;
        let now = Utc::now();
        let older_than_delta = TimeDelta::from_std(older_than).unwrap_or(TimeDelta::zero());
        let mut to_remove = vec![];

        {
            let forks = self.forks.read().await;
            for (id, fork) in forks.iter() {
                if fork.is_terminal() {
                    if let Some(completed_at) = fork.completed_at {
                        if (now - completed_at) > older_than_delta {
                            to_remove.push(*id);
                        }
                    }
                }
            }
        }

        let mut forks = self.forks.write().await;
        for id in &to_remove {
            forks.remove(id);
        }

        to_remove.len()
    }
}

impl Default for ForkManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Serde helper for serializing `Duration` as seconds (u64).
mod duration_secs {
    use serde::{Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u64(d.as_secs())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
        let secs = u64::deserialize(d)?;
        Ok(Duration::from_secs(secs))
    }

    use serde::Deserialize;
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::ApplicationId;

    #[tokio::test]
    async fn test_create_fork() {
        let manager = ForkManager::new();
        let app_id = ApplicationId::new();

        let fork_id = manager
            .create_fork(
                None,
                app_id,
                "backend".to_string(),
                "Create API".to_string(),
                vec![],
                "You are a backend agent".to_string(),
                AcceptanceCriteria::default(),
            )
            .await
            .unwrap();

        let fork = manager.get_fork(fork_id).await;
        assert!(fork.is_some());
    }

    #[tokio::test]
    async fn test_suspend_resume_fork() {
        let manager = ForkManager::new();
        let app_id = ApplicationId::new();

        let fork_id = manager
            .create_fork(
                None,
                app_id,
                "backend".to_string(),
                "Create API".to_string(),
                vec![],
                "You are a backend agent".to_string(),
                AcceptanceCriteria::default(),
            )
            .await
            .unwrap();

        // Mark as running
        {
            let mut forks = manager.forks.write().await;
            forks.get_mut(&fork_id).unwrap().state = ForkState::Running;
        }

        let task_id = TaskId::new();
        manager.suspend_fork(fork_id, task_id).await.unwrap();

        let fork = manager.get_fork(fork_id).await.unwrap();
        assert!(matches!(fork.state, ForkState::WaitingForHook));

        // Resume
        manager
            .resume_fork(
                fork_id,
                DelegateResult {
                    task_id,
                    success: true,
                    output: "API created".to_string(),
                    error: None,
                    artifacts: vec!["/api/main.rs".to_string()],
                },
            )
            .await
            .unwrap();

        let fork = manager.get_fork(fork_id).await.unwrap();
        assert!(matches!(fork.state, ForkState::Running));
    }
}
