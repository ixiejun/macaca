//! Hook Event Consumer - Listens to fork events and resumes coordinator loops.
//!
//! This module implements the automatic coordinator notification system.
//! When a delegated task completes (ForkValidated event), this consumer
//! finds the waiting coordinator session and sends a resume signal.

use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use macaca_kernel::executor::fork_manager::HookEvent;
use macaca_proto::{ApplicationId, LlmRole};
use tokio::sync::broadcast::Receiver;
use tracing::{info, warn};

use crate::runtime_resume::RuntimeResumeSignal;
use crate::state::AppState;

/// Start the hook event consumer background task.
///
/// This task monitors all ApplicationExecutors for hook events.
/// When a ForkValidated event is received, it:
/// 1. Looks up the fork_to_session mapping to find the coordinator session
/// 2. Gets the task result from the fork
/// 3. Sends a runtime resume signal to the waiting coordinator loop
pub async fn start_hook_event_consumer(state: Arc<AppState>) {
    info!("Hook event consumer started");

    // Store receivers for each app to avoid re-subscribing
    let mut app_receivers: HashMap<ApplicationId, Receiver<HookEvent>> = HashMap::new();

    loop {
        // Get all registered applications
        let executors = state.executor_registry.list_applications().await;

        // Subscribe to new apps
        for (app_id, app_name) in &executors {
            if !app_receivers.contains_key(app_id) {
                if let Some(executor) = state.executor_registry.get(app_id).await {
                    let fork_manager = executor.fork_manager();
                    let hook_rx = fork_manager.subscribe_to_hooks();
                    app_receivers.insert(app_id.clone(), hook_rx);
                    info!(app_id = %app_id, app_name = %app_name, "Subscribed to hook events for app");
                }
            }
        }

        // Remove receivers for apps that no longer exist
        let current_app_ids: std::collections::HashSet<_> =
            executors.iter().map(|(id, _)| id.clone()).collect();
        app_receivers.retain(|app_id, _| {
            let keep = current_app_ids.contains(app_id);
            if !keep {
                info!(app_id = %app_id, "Removed hook subscription for app");
            }
            keep
        });

        // Check for events from all subscribed apps
        for (app_id, hook_rx) in app_receivers.iter_mut() {
            // Non-blocking check for events
            loop {
                match hook_rx.try_recv() {
                    Ok(HookEvent::ForkValidated { fork_id, result: _ }) => {
                        info!(fork_id = %fork_id, app_id = %app_id, "ForkValidated received");

                        // Look up the session mapping
                        let mapping = state
                            .sessions
                            .fork_to_session
                            .read()
                            .await
                            .get(&fork_id)
                            .cloned();

                        if let Some(mapping) = mapping {
                            // Get the executor and fork manager for this app
                            if let Some(executor) =
                                state.executor_registry.get(&mapping.app_id).await
                            {
                                let fork_manager = executor.fork_manager();

                                // Get the fork result
                                if let Some(fork) = fork_manager.get_fork(fork_id).await {
                                    // Extract assistant output from fork messages
                                    let output = fork
                                        .own_messages
                                        .iter()
                                        .filter_map(|m| {
                                            if matches!(m.role, LlmRole::Assistant) {
                                                Some(m.content.clone())
                                            } else {
                                                None
                                            }
                                        })
                                        .collect::<Vec<_>>()
                                        .join("\n");

                                    let task_id = fork
                                        .waiting_on_task
                                        .map(|t| t.0.to_string())
                                        .unwrap_or_else(|| fork_id.to_string());

                                    // Find the active session
                                    let sessions = state.sessions.active_sessions.read().await;
                                    if let Some(session) = sessions.get(&mapping.session_id) {
                                        info!(
                                            session_id = %mapping.session_id,
                                            task_id = %task_id,
                                            "Sending resume signal to coordinator"
                                        );

                                        // Clear pause signal
                                        session.pause_signal.store(false, Ordering::SeqCst);

                                        // Send resume signal
                                        let resume_reason =
                                            RuntimeResumeSignal::DelegateCompleted {
                                                task_id,
                                                success: true,
                                                output,
                                            };

                                        if let Err(e) = session.resume_tx.send(resume_reason).await
                                        {
                                            warn!(error = %e, "Failed to send resume signal");
                                        }
                                    } else {
                                        warn!(
                                            session_id = %mapping.session_id,
                                            "Active session not found for resume"
                                        );
                                    }
                                } else {
                                    warn!(fork_id = %fork_id, "Fork not found when trying to get result");
                                }
                            }
                        } else {
                            warn!(fork_id = %fork_id, "No session mapping found for fork");
                        }
                    }

                    Ok(HookEvent::DelegateFailed {
                        fork_id,
                        task_id,
                        error,
                    }) => {
                        info!(fork_id = %fork_id, task_id = %task_id, "DelegateFailed received");

                        // Look up the session mapping
                        let mapping = state
                            .sessions
                            .fork_to_session
                            .read()
                            .await
                            .get(&fork_id)
                            .cloned();

                        if let Some(mapping) = mapping {
                            let sessions = state.sessions.active_sessions.read().await;
                            if let Some(session) = sessions.get(&mapping.session_id) {
                                info!(
                                    session_id = %mapping.session_id,
                                    task_id = %task_id,
                                    "Sending failure resume signal to coordinator"
                                );

                                session.pause_signal.store(false, Ordering::SeqCst);

                                let resume_reason = RuntimeResumeSignal::DelegateFailed {
                                    task_id: task_id.0.to_string(),
                                    error,
                                };

                                if let Err(e) = session.resume_tx.send(resume_reason).await {
                                    warn!(error = %e, "Failed to send failure resume signal");
                                }
                            }
                        }
                    }

                    Ok(HookEvent::DelegateCompleted {
                        fork_id,
                        task_id,
                        success,
                        output,
                    }) => {
                        // Log but don't resume yet - wait for ForkValidated
                        info!(
                            fork_id = %fork_id,
                            task_id = %task_id,
                            success = success,
                            output_len = output.len(),
                            "DelegateCompleted received, waiting for validation"
                        );
                    }

                    Ok(HookEvent::ForkMerged { fork_id }) => {
                        info!(fork_id = %fork_id, "ForkMerged - cleaning up mapping");

                        // Clean up the mapping
                        state
                            .sessions
                            .fork_to_session
                            .write()
                            .await
                            .remove(&fork_id);
                    }

                    Ok(other_event) => {
                        // Log other events for debugging
                        info!(event = ?other_event, "Hook event received");
                    }

                    Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {
                        // No more events, break inner loop
                        break;
                    }

                    Err(tokio::sync::broadcast::error::TryRecvError::Closed) => {
                        tracing::debug!(
                            app_id = %app_id,
                            "Hook broadcast channel closed during normal receiver churn; re-subscribing"
                        );
                        // Remove this receiver so we re-subscribe next iteration
                        break;
                    }

                    Err(tokio::sync::broadcast::error::TryRecvError::Lagged(n)) => {
                        warn!(lagged = n, app_id = %app_id, "Hook consumer lagged, continuing");
                    }
                }
            }
        }

        // Sleep before next poll
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
