//! Hook Event Consumer - Listens to fork events and resumes parent execution loops.
//!
//! This module implements the automatic parent-notification system for fork-join
//! delegation.  When a delegated task completes (`ForkValidated`) or fails
//! (`DelegateFailed`), the consumer:
//! 1. Looks up the `fork_to_session` mapping to find the waiting parent session.
//! 2. Delivers a fork-lifecycle resume through `service.execution_control` (audit).
//! 3. Adapts the result into the legacy in-memory resume channel for the parent loop.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use macaca_runtime_host::executor::fork_manager::HookEvent;
use macaca_proto::{TaskId, EXECUTION_CONTROL_SERVICE_ID};
use macaca_runtime_host::ExecutionControlForkJoinCoordinator;
use tokio::sync::broadcast::Receiver;
use tracing::{info, warn};

use crate::fork_join_shell_adapter::{
    deliver_fork_join_resume_and_notify_parent, extract_fork_assistant_output,
    REASON_FORK_JOIN_PARENT_RESUME_FAILURE, REASON_FORK_JOIN_PARENT_RESUME_SUCCESS,
};
use crate::runtime_resume::RuntimeResumeSignal;
use crate::state::AppState;

/// Start the hook event consumer background task.
///
/// The consumer polls kernel hook broadcast channels (one per registered application)
/// and translates terminal fork lifecycle events into execution-control resume
/// commands plus shell-local wakeups.
pub async fn start_hook_event_consumer(state: Arc<AppState>) {
    info!("Hook event consumer started");

    let fork_join_coordinator =
        ExecutionControlForkJoinCoordinator::new(Arc::clone(&state.service_runtime));

    // Store receivers for each app to avoid re-subscribing on every poll tick.
    let mut app_receivers: HashMap<macaca_proto::ApplicationId, Receiver<HookEvent>> =
        HashMap::new();

    loop {
        let executors = state.executor_registry.list_applications().await;

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

        let current_app_ids: std::collections::HashSet<_> =
            executors.iter().map(|(id, _)| id.clone()).collect();
        app_receivers.retain(|app_id, _| {
            let keep = current_app_ids.contains(app_id);
            if !keep {
                info!(app_id = %app_id, "Removed hook subscription for app");
            }
            keep
        });

        for (app_id, hook_rx) in app_receivers.iter_mut() {
            loop {
                match hook_rx.try_recv() {
                    Ok(HookEvent::ForkValidated { fork_id, result: _ }) => {
                        info!(fork_id = %fork_id, app_id = %app_id, "ForkValidated received");

                        let mapping = state
                            .sessions
                            .fork_to_session
                            .read()
                            .await
                            .get(&fork_id)
                            .cloned();

                        if let Some(mapping) = mapping {
                            if let Some(executor) =
                                state.executor_registry.get(&mapping.app_id).await
                            {
                                let fork_manager = executor.fork_manager();
                                if let Some(fork) = fork_manager.get_fork(fork_id).await {
                                    let output =
                                        extract_fork_assistant_output(&fork.own_messages);
                                    let task_id = fork
                                        .waiting_on_task
                                        .map(|task| task.0.to_string())
                                        .unwrap_or_else(|| fork_id.0.to_string());
                                    let delegate_task_id =
                                        fork.waiting_on_task.map(|task| TaskId(task.0));

                                    let sessions =
                                        state.sessions.active_sessions.read().await;
                                    let active_session =
                                        sessions.get(&mapping.session_id);

                                    info!(
                                        session_id = %mapping.session_id,
                                        task_id = %task_id,
                                        service_id = EXECUTION_CONTROL_SERVICE_ID,
                                        "Delivering fork-join parent resume after validation"
                                    );

                                    deliver_fork_join_resume_and_notify_parent(
                                        &fork_join_coordinator,
                                        &mapping,
                                        fork_id,
                                        delegate_task_id,
                                        REASON_FORK_JOIN_PARENT_RESUME_SUCCESS,
                                        RuntimeResumeSignal::DelegateCompleted {
                                            task_id,
                                            success: true,
                                            output,
                                        },
                                        active_session,
                                    )
                                    .await;
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

                        let mapping = state
                            .sessions
                            .fork_to_session
                            .read()
                            .await
                            .get(&fork_id)
                            .cloned();

                        if let Some(mapping) = mapping {
                            let sessions = state.sessions.active_sessions.read().await;
                            let active_session = sessions.get(&mapping.session_id);

                            info!(
                                session_id = %mapping.session_id,
                                task_id = %task_id.0,
                                service_id = EXECUTION_CONTROL_SERVICE_ID,
                                "Delivering fork-join parent failure resume"
                            );

                            deliver_fork_join_resume_and_notify_parent(
                                &fork_join_coordinator,
                                &mapping,
                                fork_id,
                                Some(TaskId(task_id.0)),
                                REASON_FORK_JOIN_PARENT_RESUME_FAILURE,
                                RuntimeResumeSignal::DelegateFailed {
                                    task_id: task_id.0.to_string(),
                                    error,
                                },
                                active_session,
                            )
                            .await;
                        }
                    }

                    Ok(HookEvent::DelegateCompleted {
                        fork_id,
                        task_id,
                        success,
                        output,
                    }) => {
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

                        state
                            .sessions
                            .fork_to_session
                            .write()
                            .await
                            .remove(&fork_id);
                    }

                    Ok(other_event) => {
                        info!(event = ?other_event, "Hook event received");
                    }

                    Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {
                        break;
                    }

                    Err(tokio::sync::broadcast::error::TryRecvError::Closed) => {
                        tracing::debug!(
                            app_id = %app_id,
                            "Hook broadcast channel closed during normal receiver churn; re-subscribing"
                        );
                        break;
                    }

                    Err(tokio::sync::broadcast::error::TryRecvError::Lagged(n)) => {
                        warn!(lagged = n, app_id = %app_id, "Hook consumer lagged, continuing");
                    }
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
