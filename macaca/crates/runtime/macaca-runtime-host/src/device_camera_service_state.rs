//! Bounded resource State for the synthetic device-camera Strategy.
//!
//! The ledger uses opaque trace-derived references and aggregate counts only.
//! Raw frames, media, hardware identifiers, and provider payloads never enter.

use std::collections::BTreeSet;

use tokio::sync::RwLock;

/// Aggregate Memento for replay-safe resource diagnostics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CameraResourceSnapshot {
    pub active_session_count: usize,
    pub active_output_count: usize,
}

/// Tracks synthetic camera resource ownership for lifecycle and cleanup tests.
#[derive(Default)]
pub struct CameraLifecycleLedger {
    sessions: RwLock<BTreeSet<String>>,
    outputs: RwLock<BTreeSet<String>>,
}

impl CameraLifecycleLedger {
    /// Apply a completed command's generic resource effect without retaining input data.
    pub async fn record_completion(&self, operation: &str, trace_id: &str) {
        match operation {
            "camera.open_session" => {
                self.sessions
                    .write()
                    .await
                    .insert(format!("session:{trace_id}"));
            }
            "camera.start_preview" | "camera.start_recording" => {
                self.outputs
                    .write()
                    .await
                    .insert(format!("output:{trace_id}"));
            }
            "camera.stop_preview" | "camera.stop_recording" => {
                self.outputs.write().await.clear();
            }
            "camera.close_session" => {
                self.outputs.write().await.clear();
                self.sessions.write().await.clear();
            }
            _ => {}
        }
    }

    /// Release all synthetic resources during shutdown or cleanup.
    pub async fn clear(&self) {
        self.sessions.write().await.clear();
        self.outputs.write().await.clear();
    }

    /// Return only aggregate resource evidence for snapshots and audit diagnostics.
    pub async fn snapshot(&self) -> CameraResourceSnapshot {
        CameraResourceSnapshot {
            active_session_count: self.sessions.read().await.len(),
            active_output_count: self.outputs.read().await.len(),
        }
    }
}
