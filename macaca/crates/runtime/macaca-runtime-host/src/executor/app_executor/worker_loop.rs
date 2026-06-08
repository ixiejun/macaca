//! Worker command loop — executes delegated tasks and forwards events.
//!
//! Processes `ExecutorCommand` messages, invokes `AgentRunner`, stores results,
//! resumes fork-join waits, and broadcasts lifecycle events to SSE subscribers.

use std::sync::Arc;
use std::time::Instant;

use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

use super::executor::ApplicationExecutor;
use super::types::WorkerState;
use crate::executor::{
    AgentInfo, AgentRunner, ApplicationId, CallbackDispatcher, DelegatedTask, EventBus,
    ExecutionQueue, ExecutorCommand, ExecutorEvent, ExecutorEventFactory, ForkManager,
    RoutingDecision, SystemEvent, TaskContext, TaskId, TaskResult, TaskRouter, TaskStatus,
};

impl ApplicationExecutor {
    /// Worker loop that processes tasks.
    pub(crate) async fn worker_loop(
        runner: Arc<dyn AgentRunner>,
        mut command_rx: mpsc::Receiver<ExecutorCommand>,
        event_tx: mpsc::Sender<ExecutorEvent>,
        event_broadcast: tokio::sync::broadcast::Sender<ExecutorEvent>,
        shutdown: Arc<RwLock<bool>>,
        queue: Arc<ExecutionQueue>,
        application_id: ApplicationId,
        fork_manager: Arc<ForkManager>,
        worker_heartbeat: Arc<RwLock<Instant>>,
        worker_state: Arc<RwLock<WorkerState>>,
    ) {
        info!(
            application_id = %application_id,
            "Worker loop started, waiting for commands"
        );

        // Spawn a background task to forward hook events to executor events
        // Using select! pattern for cleaner async handling
        let mut hook_rx = fork_manager.subscribe_to_hooks();
        let hook_event_broadcast = event_broadcast.clone();
        let hook_shutdown = Arc::clone(&shutdown);
        tokio::spawn(async move {
            // Use interval for polling instead of tight loop with sleep
            let mut poll_interval = tokio::time::interval(tokio::time::Duration::from_millis(50));
            poll_interval.tick().await; // Skip initial tick

            info!("Hook event forwarder started");

            loop {
                tokio::select! {
                    biased;

                    // 1. Shutdown signal (highest priority)
                    _ = async {
                        if *hook_shutdown.read().await {
                            futures::future::ready(()).await
                        } else {
                            std::future::pending().await
                        }
                    } => {
                        info!("Hook forwarder shutting down");
                        break;
                    }

                    // 2. Hook event receive
                    result = hook_rx.recv() => {
                        match result {
                            Ok(hook_event) => {
                                debug!("Forwarding hook event to SSE: {:?}", hook_event);
                                // Convert HookEvent to ExecutorEvent for SSE streaming
                                let executor_event = ExecutorEvent::HookEvent {
                                    event: hook_event,
                                };
                                let _ = hook_event_broadcast.send(executor_event);
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                                info!("Hook event channel closed");
                                break;
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                                warn!("Hook event channel lagged, missed {} messages", n);
                                // Continue on lag
                                continue;
                            }
                        }
                    }

                    // 3. Poll interval (lowest priority - just prevents busy-wait)
                    _ = poll_interval.tick() => {
                        // Heartbeat tick - no action needed
                    }
                }
            }
            info!("Hook event forwarder stopped");
        });

        // Heartbeat interval for worker health tracking
        let mut heartbeat_interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
        heartbeat_interval.tick().await; // Skip initial tick

        // Track when channel was closed (for graceful degradation)
        let mut channel_closed_at: Option<std::time::Instant> = None;
        const MAX_CHANNEL_CLOSED_DURATION: tokio::time::Duration =
            tokio::time::Duration::from_secs(30);

        loop {
            // Use select! with biased; for deterministic priority
            tokio::select! {
                biased;

                // 1. Shutdown check (highest priority)
                _ = async {
                    if *shutdown.read().await {
                        futures::future::ready(()).await
                    } else {
                        std::future::pending().await
                    }
                } => {
                    *worker_state.write().await = WorkerState::Shutdown;
                    info!("Worker shutting down gracefully");
                    break;
                }

                // 2. Heartbeat (fires every 10s regardless of activity)
                _ = heartbeat_interval.tick() => {
                    *worker_heartbeat.write().await = Instant::now();
                    let elapsed = worker_heartbeat.read().await.elapsed();
                    debug!(
                        application_id = %application_id,
                        elapsed_since_last = ?elapsed,
                        "Worker heartbeat"
                    );
                }

                // 3. Command receive
                cmd_opt = command_rx.recv() => {
                    match cmd_opt {
                        Some(command) => {
                            // Reset channel closed timer on successful receive
                            channel_closed_at = None;
                            info!("Worker received command: {:?}", std::mem::discriminant(&command));

                            match command {
                                ExecutorCommand::Execute(task) => {
                                    let task_id = task.id;
                                    let agent_name = task.to_agent.clone();
                                    let prompt = task.prompt.clone();
                                    let context = task.context.clone();
                                    let events =
                                        ExecutorEventFactory::new(task_id, agent_name.clone());

                                    // Notify task started
                                    let start_event = events.started();
                                    let _ = event_tx.send(start_event.clone()).await;
                                    let _ = event_broadcast.send(start_event);

                                    info!(
                                        task_id = %task_id,
                                        agent = %agent_name,
                                        prompt_preview = %prompt.chars().take(100).collect::<String>(),
                                        "Worker: executing task"
                                    );

                                    // Create a channel for agent execution events
                                    let (agent_event_tx, mut agent_event_rx) =
                                        mpsc::channel::<macaca_proto::AgentExecutionEvent>(64);

                                    // Clone event_tx and event_broadcast for the forwarding task
                                    let event_tx_clone = event_tx.clone();
                                    let event_broadcast_clone = event_broadcast.clone();
                                    let agent_name_clone = agent_name.clone();

                                    // Spawn a task to forward agent events to executor events
                                    let forward_handle = tokio::spawn(async move {
                                        while let Some(event) = agent_event_rx.recv().await {
                                            let executor_event = ExecutorEvent::AgentEvent {
                                                task_id,
                                                agent: agent_name_clone.clone(),
                                                event,
                                            };
                                            // Broadcast to external subscribers (SSE, etc.)
                                            // Note: event_tx internal channel is intentionally skipped
                                            // to avoid blocking when nobody consumes event_rx.
                                            let _ = event_broadcast_clone.send(executor_event);
                                        }
                                    });

                                    // Execute the agent with events
                                    let result = runner.execute_agent_with_events(
                                        &application_id,
                                        &agent_name,
                                        &prompt,
                                        context,
                                        Some(agent_event_tx),
                                    ).await;

                                    // Wait for event forwarding to complete
                                    let _ = forward_handle.await;

                                    match result {
                                        Ok(mut task_result) => {
                                            info!(
                                                task_id = %task_id,
                                                agent = %agent_name,
                                                success = task_result.success,
                                                output_len = task_result.output.len(),
                                                error = ?task_result.error,
                                                "Worker: task executed, storing result"
                                            );

                                            // Use the original task_id, not the one from the runner
                                            task_result.task_id = task_id;

                                            // Store result in queue
                                            queue.store_result(task_result.clone()).await;

                                            // Resume any fork waiting on this task
                                            let delegate_result = crate::executor::fork_manager::DelegateResult {
                                                task_id: macaca_proto::TaskId(task_id.0),
                                                success: task_result.success,
                                                output: task_result.output.clone(),
                                                error: task_result.error.clone(),
                                                artifacts: task_result.artifacts.clone(),
                                            };
                                            if let Err(e) = fork_manager.resume_fork_by_task(macaca_proto::TaskId(task_id.0), delegate_result).await {
                                                warn!(task_id = %task_id, error = %e, "Failed to resume fork waiting on task");
                                            }

                                            // Notify task completed
                                            let completed_event =
                                                events.completed_with_result(task_result);
                                            let _ = event_tx.send(completed_event.clone()).await;
                                            let _ = event_broadcast.send(completed_event);
                                        }
                                        Err(e) => {
                                            error!(
                                                task_id = %task_id,
                                                agent = %agent_name,
                                                error = %e,
                                                "Worker: task execution failed"
                                            );

                                            // Store error result with original task_id
                                            let error_result = events.failed_result(e.clone());
                                            queue.store_result(error_result).await;

                                            // Notify task failed
                                            let failed_event = events.failed(e);
                                            let _ = event_tx.send(failed_event.clone()).await;
                                            let _ = event_broadcast.send(failed_event);
                                        }
                                    }
                                }
                                ExecutorCommand::Cancel(task_id) => {
                                    if queue.cancel(task_id).await {
                                        let cancel_event = ExecutorEvent::TaskCancelled { task_id };
                                        let _ = event_tx.send(cancel_event.clone()).await;
                                        let _ = event_broadcast.send(cancel_event);
                                    }
                                }
                                ExecutorCommand::Shutdown => {
                                    info!("Shutdown command received");
                                    break;
                                }
                            }
                        }
                        None => {
                            // Channel closed - track time and wait before exiting
                            let closed_at = channel_closed_at.get_or_insert(std::time::Instant::now());

                            // Update worker state to Disconnected
                            *worker_state.write().await = WorkerState::Disconnected;

                            if closed_at.elapsed() > MAX_CHANNEL_CLOSED_DURATION {
                                error!(
                                    application_id = %application_id,
                                    duration_secs = closed_at.elapsed().as_secs(),
                                    "Command channel closed for too long, worker exiting"
                                );
                                break;
                            }

                            warn!(
                                application_id = %application_id,
                                duration_secs = closed_at.elapsed().as_secs(),
                                "Command channel closed, waiting for recovery..."
                            );
                            // Continue loop - will check again on next iteration
                        }
                    }
                }
            }
        }

        info!(
            application_id = %application_id,
            "Worker loop exited"
        );
    }
}
