//! Runtime-host owner for local execution-control wake channels.
//!
//! `service.execution_control` remains the authoritative state machine for
//! pause/resume audit evidence. This owner only stores in-process pause flags
//! and resume senders used by local framework middleware until every host can
//! subscribe to service-owned execution-control event streams.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use macaca_proto::RuntimeResumeSignal;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn};

/// Non-serializable local wake handle for one running execution-control session.
#[derive(Clone)]
pub struct ExecutionControlLocalNotification {
    /// Local pause flag read by framework middleware during an active run.
    pub pause_signal: Arc<AtomicBool>,
    /// Local resume channel used after an audited execution-control decision.
    pub resume_tx: mpsc::Sender<RuntimeResumeSignal>,
}

/// Runtime-host owned registry for local execution-control notifications.
#[derive(Default)]
pub struct ExecutionControlLocalNotificationRuntime {
    notifications: RwLock<HashMap<String, ExecutionControlLocalNotification>>,
}

impl ExecutionControlLocalNotificationRuntime {
    /// Create an empty local notification runtime.
    pub fn new() -> Self {
        Self::default()
    }

    /// Install or replace the local notification for a session.
    ///
    /// Replacement is intentional: the route can create a placeholder before
    /// agent execution installs the run-specific pause flag and resume channel.
    pub async fn install(
        &self,
        session_id: impl Into<String>,
        notification: ExecutionControlLocalNotification,
    ) {
        let session_id = session_id.into();
        self.notifications
            .write()
            .await
            .insert(session_id.clone(), notification);
        info!(
            session_id = %session_id,
            "execution-control local notification installed in runtime host"
        );
    }

    /// Return a clone of the local notification for out-of-lock delivery.
    pub async fn get(&self, session_id: &str) -> Option<ExecutionControlLocalNotification> {
        let notification = self.notifications.read().await.get(session_id).cloned();
        debug!(
            session_id,
            found = notification.is_some(),
            "execution-control local notification lookup completed"
        );
        notification
    }

    /// Remove one session notification.
    pub async fn remove(&self, session_id: &str) {
        let removed = self
            .notifications
            .write()
            .await
            .remove(session_id)
            .is_some();
        debug!(
            session_id,
            removed, "execution-control local notification removed from runtime host"
        );
    }

    /// Remove multiple session notifications during stop/cleanup flows.
    pub async fn remove_many<'a>(&self, session_ids: impl IntoIterator<Item = &'a String>) {
        let mut notifications = self.notifications.write().await;
        let mut removed = 0usize;
        for session_id in session_ids {
            if notifications.remove(session_id).is_some() {
                removed += 1;
            }
        }
        info!(
            removed,
            "execution-control local notifications removed from runtime host"
        );
    }

    /// Clear the local pause flag and send a resume signal.
    ///
    /// The return value is `true` when a local notification existed and accepted
    /// the signal. Missing or closed local channels are logged as diagnostics;
    /// execution-control service delivery remains the authoritative evidence.
    pub async fn notify_resume(
        &self,
        session_id: &str,
        resume_signal: RuntimeResumeSignal,
    ) -> bool {
        let Some(notification) = self.get(session_id).await else {
            warn!(
                session_id,
                "execution-control local notification not found for runtime resume"
            );
            return false;
        };

        notification.pause_signal.store(false, Ordering::SeqCst);
        match notification.resume_tx.send(resume_signal).await {
            Ok(()) => {
                debug!(
                    session_id,
                    "execution-control local resume signal delivered by runtime host"
                );
                true
            }
            Err(error) => {
                warn!(
                    session_id,
                    error = %error,
                    "execution-control local resume channel rejected signal"
                );
                false
            }
        }
    }
}
