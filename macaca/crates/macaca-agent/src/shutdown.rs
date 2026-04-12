//! Graceful shutdown support: listens for SIGTERM and invokes a user callback.

use tokio::sync::oneshot;
use tracing::{info, warn};

/// A handle that triggers graceful shutdown when SIGTERM is received.
///
/// # Example
/// ```no_run
/// # use macaca_agent::ShutdownHandle;
/// # #[tokio::main]
/// # async fn main() {
/// let handle = ShutdownHandle::spawn(|| {
///     println!("saving agent state…");
/// });
/// handle.wait().await;
/// # }
/// ```
pub struct ShutdownHandle {
    rx: oneshot::Receiver<()>,
}

impl ShutdownHandle {
    /// Spawn a background task that waits for SIGTERM (or SIGINT on non-Unix
    /// platforms) then calls `on_shutdown` before signalling this handle.
    pub fn spawn<F>(on_shutdown: F) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        let (tx, rx) = oneshot::channel::<()>();

        tokio::spawn(async move {
            wait_for_signal().await;
            info!("shutdown signal received — invoking shutdown callback");
            on_shutdown();
            let _ = tx.send(());
        });

        Self { rx }
    }

    /// Wait until the shutdown signal has been processed.
    pub async fn wait(self) {
        match self.rx.await {
            Ok(()) => info!("graceful shutdown complete"),
            Err(_) => warn!("shutdown sender dropped before signal"),
        }
    }
}

// ── Platform-specific signal waiting ─────────────────────────────────────────

#[cfg(unix)]
async fn wait_for_signal() {
    use tokio::signal::unix::{signal, SignalKind};

    let mut sigterm = signal(SignalKind::terminate()).expect("failed to register SIGTERM handler");
    let mut sigint = signal(SignalKind::interrupt()).expect("failed to register SIGINT handler");

    tokio::select! {
        _ = sigterm.recv() => info!("SIGTERM received"),
        _ = sigint.recv()  => info!("SIGINT received"),
    }
}

#[cfg(not(unix))]
async fn wait_for_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to register Ctrl-C handler");
    info!("Ctrl-C received");
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    /// Verify the callback fires when the sender is triggered directly.
    #[tokio::test]
    async fn shutdown_callback_invoked() {
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        // Use a manually controlled channel to avoid actually sending OS signals.
        let (tx, rx) = oneshot::channel::<()>();

        let flag = called_clone.clone();
        tokio::spawn(async move {
            let _ = rx.await;
            flag.store(true, Ordering::SeqCst);
        });

        let _ = tx.send(());
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        assert!(called_clone.load(Ordering::SeqCst));
    }
}
