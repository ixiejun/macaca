//! Task Scheduler: cron-based and fixed-interval scheduling.
//!
//! Schedules are persisted to RedbStore with prefix-based keys:
//! - `schedule/{app_id}/{schedule_id}` → ScheduleEntry
//!
//! The scheduler runs as a background tokio task, emitting [`ScheduleEvent`]s
//! when entries are due. The caller is responsible for acting on those events
//! (creating Goals or TodoItems via TaskSpace / TodoStore).

use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use macaca_persist::PersistBackend;
use macaca_proto::{ApplicationId, TaskId};

// ── Key prefixes ─────────────────────────────────────────────────────────────

const SCHEDULE_PREFIX: &str = "schedule/";

// ── ScheduleAction ────────────────────────────────────────────────────────────

/// What action to take when a schedule triggers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScheduleAction {
    /// Push a high-level goal to TaskSpace for LLM decomposition.
    CreateGoal { description: String },
    /// Create a specific TodoItem directly (no LLM decomposition).
    CreateTask {
        agent: String,
        title: String,
        description: String,
        priority: u8,
    },
}

// ── ScheduleEntry ─────────────────────────────────────────────────────────────

/// A scheduled entry — either cron-based or fixed-interval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleEntry {
    pub id: TaskId,
    pub app_id: ApplicationId,
    pub name: String,
    /// Cron expression (7-field: sec min hour day month weekday year).
    /// For 5-field user input, the caller should normalise via
    /// [`ScheduleEntry::normalise_cron`] before storing.
    /// When set, `interval_secs` is ignored.
    pub cron_expr: Option<String>,
    /// Fixed interval in seconds (alternative to cron).
    pub interval_secs: Option<u64>,
    pub action: ScheduleAction,
    pub enabled: bool,
    pub last_run_at: Option<DateTime<Utc>>,
    pub next_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub run_count: u64,
}

impl ScheduleEntry {
    /// Normalise a 5-field cron string to 7-field by prepending "0 " (seconds)
    /// and appending " *" (year).  7-field strings are returned unchanged.
    pub fn normalise_cron(expr: &str) -> String {
        let fields = expr.split_whitespace().count();
        match fields {
            5 => format!("0 {} *", expr),
            _ => expr.to_owned(),
        }
    }

    /// Create a cron-based schedule entry (not yet saved).
    ///
    /// `cron_expr` may be 5-field or 7-field; 5-field inputs are normalised
    /// automatically.
    pub fn new_cron(
        app_id: ApplicationId,
        name: impl Into<String>,
        cron_expr: impl Into<String>,
        action: ScheduleAction,
    ) -> Self {
        let expr = Self::normalise_cron(&cron_expr.into());
        let now = Utc::now();
        let mut entry = Self {
            id: TaskId::new(),
            app_id,
            name: name.into(),
            cron_expr: Some(expr),
            interval_secs: None,
            action,
            enabled: true,
            last_run_at: None,
            next_run_at: None,
            created_at: now,
            run_count: 0,
        };
        entry.compute_next_run();
        entry
    }

    /// Create a fixed-interval schedule entry (not yet saved).
    pub fn new_interval(
        app_id: ApplicationId,
        name: impl Into<String>,
        interval_secs: u64,
        action: ScheduleAction,
    ) -> Self {
        let now = Utc::now();
        let mut entry = Self {
            id: TaskId::new(),
            app_id,
            name: name.into(),
            cron_expr: None,
            interval_secs: Some(interval_secs),
            action,
            enabled: true,
            last_run_at: None,
            next_run_at: None,
            created_at: now,
            run_count: 0,
        };
        entry.compute_next_run();
        entry
    }

    /// Compute and store `next_run_at` based on the cron expression or
    /// interval.  Must be called after any state change that affects timing
    /// (creation, completion of a run, re-enabling).
    pub fn compute_next_run(&mut self) {
        let now = Utc::now();
        if let Some(ref expr) = self.cron_expr.clone() {
            match cron::Schedule::from_str(expr) {
                Ok(schedule) => {
                    self.next_run_at = schedule.upcoming(Utc).next();
                }
                Err(e) => {
                    tracing::warn!(expr = %expr, error = %e, "Invalid cron expression");
                    self.next_run_at = None;
                }
            }
        } else if let Some(secs) = self.interval_secs {
            let base = self.last_run_at.unwrap_or(now);
            self.next_run_at = Some(base + chrono::Duration::seconds(secs as i64));
        }
    }

    /// Returns `true` when this entry is enabled and its `next_run_at` is in
    /// the past (i.e., it is due to fire).
    pub fn is_due(&self) -> bool {
        if !self.enabled {
            return false;
        }
        match self.next_run_at {
            Some(next) => Utc::now() >= next,
            None => false,
        }
    }
}

// ── SchedulerConfig ───────────────────────────────────────────────────────────

/// Configuration for the scheduler background loop.
pub struct SchedulerConfig {
    /// How often to poll for due entries (default: 60 s).
    pub check_interval_secs: u64,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: 60,
        }
    }
}

// ── ScheduleEvent ─────────────────────────────────────────────────────────────

/// Events emitted by the scheduler when an entry is due.
///
/// The caller is responsible for acting on these events.
#[derive(Debug, Clone)]
pub enum ScheduleEvent {
    Triggered {
        schedule_id: TaskId,
        action: ScheduleAction,
    },
}

// ── TaskScheduler ─────────────────────────────────────────────────────────────

/// Persistent task scheduler.  Backed by a [`RedbStore`].
///
/// # Usage
///
/// ```no_run
/// use std::sync::Arc;
/// use std::sync::atomic::{AtomicBool, Ordering};
/// use macaca_task::scheduler::{TaskScheduler, SchedulerConfig, ScheduleEntry, ScheduleAction};
/// use macaca_proto::ApplicationId;
///
/// async fn example(store: Arc<macaca_persist::RedbStore>) {
///     let store: Arc<dyn macaca_persist::PersistBackend> = store;
///     let scheduler = TaskScheduler::new(store, SchedulerConfig::default());
///     let app_id = ApplicationId::new();
///     let shutdown = Arc::new(AtomicBool::new(false));
///     let (tx, mut rx) = tokio::sync::mpsc::channel(32);
///     tokio::spawn(async move {
///         scheduler.run(app_id, shutdown, tx).await;
///     });
///     while let Some(event) = rx.recv().await {
///         // handle event
///     }
/// }
/// ```
pub struct TaskScheduler {
    store: Arc<dyn PersistBackend>,
    config: SchedulerConfig,
}

impl TaskScheduler {
    pub fn new(store: Arc<dyn PersistBackend>, config: SchedulerConfig) -> Self {
        Self { store, config }
    }

    // ── Key helpers ──────────────────────────────────────────────────────────

    fn schedule_key(app_id: &ApplicationId, id: &TaskId) -> String {
        format!("{}{}/{}", SCHEDULE_PREFIX, app_id, id)
    }

    fn app_prefix(app_id: &ApplicationId) -> String {
        format!("{}{}/", SCHEDULE_PREFIX, app_id)
    }

    // ── CRUD ─────────────────────────────────────────────────────────────────

    /// Persist a schedule entry (create or update).
    pub async fn save(&self, entry: &ScheduleEntry) {
        let key = Self::schedule_key(&entry.app_id, &entry.id);
        if let Ok(data) = serde_json::to_vec(entry) {
            let _ = self.store.set(&key, &data).await;
        }
    }

    /// Delete a schedule entry.
    pub async fn delete(&self, app_id: &ApplicationId, id: &TaskId) {
        let key = Self::schedule_key(app_id, id);
        let _ = self.store.delete(&key).await;
    }

    /// Get a single schedule entry by ID.
    pub async fn get(&self, app_id: &ApplicationId, id: &TaskId) -> Option<ScheduleEntry> {
        let key = Self::schedule_key(app_id, id);
        self.store
            .get(&key)
            .await
            .ok()
            .flatten()
            .and_then(|data| serde_json::from_slice(&data).ok())
    }

    /// List all schedule entries for an application.
    pub async fn list(&self, app_id: &ApplicationId) -> Vec<ScheduleEntry> {
        let prefix = Self::app_prefix(app_id);
        let keys = self.store.list_keys(&prefix).await.unwrap_or_default();
        let mut entries = Vec::new();
        for key in keys {
            if let Ok(Some(data)) = self.store.get(&key).await {
                if let Ok(entry) = serde_json::from_slice::<ScheduleEntry>(&data) {
                    entries.push(entry);
                }
            }
        }
        entries
    }

    /// Create a new schedule entry: computes `next_run_at` and persists it.
    pub async fn create(&self, mut entry: ScheduleEntry) -> ScheduleEntry {
        entry.compute_next_run();
        self.save(&entry).await;
        entry
    }

    /// Toggle a schedule's enabled state.  Re-computes `next_run_at` when
    /// re-enabling.  Returns `false` if the entry was not found.
    pub async fn set_enabled(&self, app_id: &ApplicationId, id: &TaskId, enabled: bool) -> bool {
        if let Some(mut entry) = self.get(app_id, id).await {
            entry.enabled = enabled;
            if enabled {
                entry.compute_next_run();
            }
            self.save(&entry).await;
            true
        } else {
            false
        }
    }

    // ── Background loop ───────────────────────────────────────────────────────

    /// Run the scheduler loop for `app_id`.
    ///
    /// Checks for due entries every [`SchedulerConfig::check_interval_secs`]
    /// seconds, emits [`ScheduleEvent::Triggered`] for each due entry, then
    /// updates `last_run_at`, increments `run_count`, and recomputes
    /// `next_run_at`.
    ///
    /// The loop exits when `shutdown` is set to `true`.
    pub async fn run(
        &self,
        app_id: ApplicationId,
        shutdown: Arc<AtomicBool>,
        event_tx: tokio::sync::mpsc::Sender<ScheduleEvent>,
    ) {
        tracing::info!(app_id = %app_id, "Scheduler started");
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(
            self.config.check_interval_secs,
        ));
        // Skip the immediate first tick so we don't fire at startup.
        interval.tick().await;

        loop {
            interval.tick().await;

            if shutdown.load(Ordering::SeqCst) {
                tracing::info!(app_id = %app_id, "Scheduler shutting down");
                break;
            }

            let entries = self.list(&app_id).await;
            for mut entry in entries {
                if !entry.is_due() {
                    continue;
                }

                tracing::info!(
                    schedule_id = %entry.id,
                    name = %entry.name,
                    app_id = %app_id,
                    "Schedule triggered"
                );

                let event = ScheduleEvent::Triggered {
                    schedule_id: entry.id,
                    action: entry.action.clone(),
                };

                if let Err(e) = event_tx.send(event).await {
                    tracing::warn!(error = %e, "Scheduler event channel closed; stopping");
                    return;
                }

                entry.last_run_at = Some(Utc::now());
                entry.run_count += 1;
                entry.compute_next_run();
                self.save(&entry).await;
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_persist::RedbStore;
    use tempfile::tempdir;

    fn make_store() -> (Arc<RedbStore>, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("sched.redb");
        let store = Arc::new(RedbStore::open(db_path).unwrap());
        (store, dir)
    }

    fn make_scheduler(store: Arc<RedbStore>) -> TaskScheduler {
        TaskScheduler::new(
            store,
            SchedulerConfig {
                check_interval_secs: 60,
            },
        )
    }

    // ── is_due for interval schedules ────────────────────────────────────────

    #[test]
    fn test_interval_schedule_is_due() {
        let app_id = ApplicationId::new();
        let action = ScheduleAction::CreateGoal {
            description: "daily sync".into(),
        };
        let mut entry = ScheduleEntry::new_interval(app_id, "test", 3600, action);

        // Freshly created: next_run_at is now + 3600 s → not due yet.
        assert!(
            !entry.is_due(),
            "should not be due immediately after creation"
        );

        // Simulate that the last run happened more than 3600 s ago.
        entry.last_run_at = Some(Utc::now() - chrono::Duration::seconds(7200));
        entry.compute_next_run();
        // next_run_at = last_run_at + 3600 s = now - 3600 s → past → due.
        assert!(entry.is_due(), "should be due after interval elapsed");
    }

    // ── disabled schedule is never due ───────────────────────────────────────

    #[test]
    fn test_disabled_schedule_not_due() {
        let app_id = ApplicationId::new();
        let action = ScheduleAction::CreateGoal {
            description: "disabled".into(),
        };
        let mut entry = ScheduleEntry::new_interval(app_id, "test", 1, action);

        // Force next_run_at into the past.
        entry.last_run_at = Some(Utc::now() - chrono::Duration::seconds(100));
        entry.compute_next_run();

        entry.enabled = false;
        assert!(!entry.is_due(), "disabled schedule must never be due");
    }

    // ── cron: compute_next_run sets a future next_run_at ────────────────────

    #[test]
    fn test_cron_compute_next_run() {
        let app_id = ApplicationId::new();
        let action = ScheduleAction::CreateTask {
            agent: "backend".into(),
            title: "Daily report".into(),
            description: "Generate daily report".into(),
            priority: 5,
        };
        // "0 9 * * *" (5-field) → should parse after normalisation.
        let entry = ScheduleEntry::new_cron(app_id, "daily", "0 9 * * *", action);

        assert!(
            entry.next_run_at.is_some(),
            "next_run_at must be computed for a valid cron expression"
        );
        assert!(
            entry.next_run_at.unwrap() > Utc::now(),
            "next_run_at must be in the future"
        );
    }

    // ── CRUD: create → list → get → delete ──────────────────────────────────

    #[tokio::test]
    async fn test_schedule_crud() {
        let (store, _dir) = make_store();
        let scheduler = make_scheduler(Arc::clone(&store));
        let app_id = ApplicationId::new();

        let action = ScheduleAction::CreateGoal {
            description: "cleanup".into(),
        };
        let entry = ScheduleEntry::new_interval(app_id.clone(), "cleanup", 300, action);
        let id = entry.id;

        // Create
        let created = scheduler.create(entry).await;
        assert_eq!(created.id, id);

        // List
        let list = scheduler.list(&app_id).await;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "cleanup");

        // Get
        let fetched = scheduler.get(&app_id, &id).await;
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().interval_secs, Some(300));

        // Delete
        scheduler.delete(&app_id, &id).await;
        let after_delete = scheduler.list(&app_id).await;
        assert!(after_delete.is_empty());
    }

    // ── set_enabled toggles and recomputes ───────────────────────────────────

    #[tokio::test]
    async fn test_set_enabled_toggle() {
        let (store, _dir) = make_store();
        let scheduler = make_scheduler(Arc::clone(&store));
        let app_id = ApplicationId::new();

        let action = ScheduleAction::CreateGoal {
            description: "ping".into(),
        };
        let entry = ScheduleEntry::new_interval(app_id.clone(), "ping", 60, action);
        let id = entry.id;
        scheduler.create(entry).await;

        // Disable
        let ok = scheduler.set_enabled(&app_id, &id, false).await;
        assert!(ok);
        let e = scheduler.get(&app_id, &id).await.unwrap();
        assert!(!e.enabled);

        // Re-enable — next_run_at must be recomputed (not None).
        let ok = scheduler.set_enabled(&app_id, &id, true).await;
        assert!(ok);
        let e = scheduler.get(&app_id, &id).await.unwrap();
        assert!(e.enabled);
        assert!(e.next_run_at.is_some());
    }

    // ── set_enabled returns false for unknown id ─────────────────────────────

    #[tokio::test]
    async fn test_set_enabled_unknown() {
        let (store, _dir) = make_store();
        let scheduler = make_scheduler(store);
        let app_id = ApplicationId::new();
        let unknown_id = TaskId::new();

        let ok = scheduler.set_enabled(&app_id, &unknown_id, false).await;
        assert!(!ok);
    }

    // ── normalise_cron: 5-field → 7-field, 7-field unchanged ────────────────

    #[test]
    fn test_normalise_cron() {
        let five = ScheduleEntry::normalise_cron("0 9 * * *");
        assert_eq!(five.split_whitespace().count(), 7);

        let seven = ScheduleEntry::normalise_cron("0 0 9 * * * *");
        assert_eq!(seven.split_whitespace().count(), 7);
        assert_eq!(seven, "0 0 9 * * * *");
    }
}
