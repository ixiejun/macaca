//! Provider-neutral alert primitives for the microkernel.
//!
//! The kernel owns alert identity, severity, deduplication, and the abstract
//! sink port. Concrete transports such as webhooks are service providers owned
//! by runtime-host so the microkernel never constructs network clients.

pub use macaca_proto::{Alert, AlertSeverity};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Configuration for the alert system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    pub enabled: bool,
    /// Deduplication window in seconds (default: 300 = 5 minutes).
    pub dedup_interval_secs: u64,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            dedup_interval_secs: 300,
        }
    }
}

/// Provider-neutral alert sink port.
///
/// Implementations may be test doubles or adapters that forward alerts into a
/// service client. Concrete transport providers must live outside the kernel.
#[async_trait::async_trait]
pub trait AlertChannel: Send + Sync {
    async fn send(&self, alert: &Alert) -> Result<(), String>;
    fn name(&self) -> &str;
}

/// The alert manager. Handles deduplication and routing to channels.
pub struct AlertManager {
    config: AlertConfig,
    channels: Vec<Box<dyn AlertChannel>>,
    /// Dedup map: alert_key → last_sent_time
    dedup: Arc<RwLock<HashMap<String, Instant>>>,
}

impl AlertManager {
    pub fn new(config: AlertConfig) -> Self {
        Self {
            config,
            channels: Vec::new(),
            dedup: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Build a manager with explicit alert sink ports.
    ///
    /// This constructor is intentionally port-based. Runtime-host may provide a
    /// service-client adapter, and tests may provide counters, without making
    /// the kernel depend on concrete alert transports.
    pub fn with_channels(config: AlertConfig, channels: Vec<Box<dyn AlertChannel>>) -> Self {
        Self {
            config,
            channels,
            dedup: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Generate a dedup key from alert title and source.
    fn dedup_key(alert: &Alert) -> String {
        format!("{}:{}", alert.source, alert.title)
    }

    /// Fire an alert. Returns true if actually sent (not deduped).
    pub async fn fire(&self, alert: Alert) -> bool {
        if !self.config.enabled {
            return false;
        }

        let key = Self::dedup_key(&alert);
        let dedup_window = Duration::from_secs(self.config.dedup_interval_secs);

        // Check dedup
        {
            let map = self.dedup.read().await;
            if let Some(last) = map.get(&key) {
                if last.elapsed() < dedup_window {
                    tracing::debug!(key = %key, "Alert deduplicated");
                    return false;
                }
            }
        }

        // Record this alert
        {
            let mut map = self.dedup.write().await;
            map.insert(key, Instant::now());
        }

        tracing::info!(
            severity = ?alert.severity,
            source = %alert.source,
            title = %alert.title,
            channel_count = self.channels.len(),
            "kernel alert accepted by provider-neutral manager"
        );

        // Send to configured abstract channels. Concrete transport remains
        // outside the kernel and is reached only through this port.
        for channel in &self.channels {
            if let Err(e) = channel.send(&alert).await {
                tracing::error!(channel = channel.name(), error = %e, "Failed to send alert");
            }
        }

        true
    }

    // ─── Convenience methods for common alert scenarios ───

    pub async fn budget_warning(&self, spent: f64, limit: f64) {
        self.fire(Alert::warning(
            "Budget Warning",
            format!(
                "Cost ${:.4} has exceeded 80% of budget ${:.4}",
                spent, limit
            ),
            "cost_tracker",
        ))
        .await;
    }

    pub async fn budget_exceeded(&self, spent: f64, limit: f64) {
        self.fire(Alert::critical(
            "Budget Exceeded",
            format!("Cost ${:.4} exceeded budget ${:.4}", spent, limit),
            "cost_tracker",
        ))
        .await;
    }

    pub async fn worker_restart_warning(&self, agent: &str, count: u32) {
        self.fire(Alert::warning(
            "Worker Restart",
            format!("Worker '{}' has restarted {} times", agent, count),
            "supervisor",
        ))
        .await;
    }

    pub async fn all_llm_degraded(&self, model: &str) {
        self.fire(Alert::critical(
            "All LLM Models Failed",
            format!("Primary model '{}' and all fallbacks have failed", model),
            "llm_router",
        ))
        .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    struct CountingChannel {
        count: Arc<AtomicU32>,
    }

    #[async_trait::async_trait]
    impl AlertChannel for CountingChannel {
        async fn send(&self, _alert: &Alert) -> Result<(), String> {
            self.count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        fn name(&self) -> &str {
            "counting"
        }
    }

    fn make_manager_with_counter(enabled: bool) -> (AlertManager, Arc<AtomicU32>) {
        let counter = Arc::new(AtomicU32::new(0));
        let config = AlertConfig {
            enabled,
            dedup_interval_secs: 300,
        };
        let channels: Vec<Box<dyn AlertChannel>> = vec![Box::new(CountingChannel {
            count: counter.clone(),
        })];
        let manager = AlertManager::with_channels(config, channels);
        (manager, counter)
    }

    #[tokio::test]
    async fn test_alert_fires() {
        let (manager, counter) = make_manager_with_counter(true);
        let sent = manager
            .fire(Alert::warning("Test", "test message", "test_source"))
            .await;
        assert!(sent, "Alert should be sent");
        assert_eq!(
            counter.load(Ordering::SeqCst),
            1,
            "Channel should receive one call"
        );
    }

    #[tokio::test]
    async fn test_dedup_blocks_repeat() {
        let (manager, counter) = make_manager_with_counter(true);
        let alert = Alert::warning("DedupTitle", "msg1", "src1");

        let sent1 = manager.fire(alert.clone()).await;
        let sent2 = manager.fire(alert.clone()).await;

        assert!(sent1, "First alert should be sent");
        assert!(!sent2, "Second alert within window should be deduplicated");
        assert_eq!(
            counter.load(Ordering::SeqCst),
            1,
            "Channel should only see one call"
        );
    }

    #[tokio::test]
    async fn test_dedup_different_sources_not_blocked() {
        let (manager, counter) = make_manager_with_counter(true);

        let sent1 = manager
            .fire(Alert::warning("SameTitle", "msg", "source_a"))
            .await;
        let sent2 = manager
            .fire(Alert::warning("SameTitle", "msg", "source_b"))
            .await;

        assert!(sent1);
        assert!(sent2, "Different source should not be deduped");
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_dedup_different_titles_not_blocked() {
        let (manager, counter) = make_manager_with_counter(true);

        let sent1 = manager.fire(Alert::warning("Title A", "msg", "src")).await;
        let sent2 = manager.fire(Alert::warning("Title B", "msg", "src")).await;

        assert!(sent1);
        assert!(sent2, "Different title should not be deduped");
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_disabled_config() {
        let (manager, counter) = make_manager_with_counter(false);
        let sent = manager
            .fire(Alert::critical("Critical", "bad thing", "src"))
            .await;
        assert!(!sent, "Disabled manager should not send alerts");
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_convenience_budget_warning() {
        let (manager, counter) = make_manager_with_counter(true);
        manager.budget_warning(8.5, 10.0).await;
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_convenience_budget_exceeded() {
        let (manager, counter) = make_manager_with_counter(true);
        manager.budget_exceeded(12.0, 10.0).await;
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_convenience_worker_restart() {
        let (manager, counter) = make_manager_with_counter(true);
        manager.worker_restart_warning("agent-1", 3).await;
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_convenience_all_llm_degraded() {
        let (manager, counter) = make_manager_with_counter(true);
        manager.all_llm_degraded("claude-3-opus").await;
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_zero_dedup_window_always_sends() {
        let counter = Arc::new(AtomicU32::new(0));
        let config = AlertConfig {
            enabled: true,
            dedup_interval_secs: 0,
        };
        let channels: Vec<Box<dyn AlertChannel>> = vec![Box::new(CountingChannel {
            count: counter.clone(),
        })];
        let manager = AlertManager::with_channels(config, channels);

        let alert = Alert::info("ZeroWindow", "msg", "src");
        manager.fire(alert.clone()).await;
        manager.fire(alert.clone()).await;
        // With 0-second window, elapsed() will always be >= Duration::from_secs(0),
        // so second fire also sends.
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }
}
