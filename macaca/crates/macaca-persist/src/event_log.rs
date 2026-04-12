//! Append-only event log backed by RedbStore.
//!
//! Every event is stored at `events/{session_id}/{seq:08d}` as a JSON blob.
//! Events are never modified or deleted during normal operation.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::store::PersistStore;
use crate::RedbStore;
use macaca_proto::types::EventEntry;

const EVENTS_PREFIX: &str = "events/";

/// Append-only event log for session events.
///
/// Key schema: `events/{session_id}/{seq:08d}` → EventEntry JSON
///
/// Design principles:
/// - Append-only: events are never modified after write
/// - Immediate: each append() writes to DB before returning
/// - Per-session sequences: each session has its own monotonic counter
/// - Independent of SSE/browser: this is backend infrastructure
pub struct EventLog {
    store: Arc<RedbStore>,
    /// Per-session sequence counters. Loaded from DB on first access.
    seq_counters: RwLock<HashMap<String, AtomicU64>>,
    /// Broadcast notification when new events are appended.
    /// Sends (session_id, latest_seq).
    notify_tx: tokio::sync::broadcast::Sender<(String, u64)>,
}

impl EventLog {
    pub fn new(store: Arc<RedbStore>) -> Self {
        let (notify_tx, _) = tokio::sync::broadcast::channel(1024);
        Self {
            store,
            seq_counters: RwLock::new(HashMap::new()),
            notify_tx,
        }
    }

    /// Subscribe to event notifications.
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<(String, u64)> {
        self.notify_tx.subscribe()
    }

    fn event_key(session_id: &str, seq: u64) -> String {
        format!("{}{}/{:08}", EVENTS_PREFIX, session_id, seq)
    }

    fn session_prefix(session_id: &str) -> String {
        format!("{}{}/", EVENTS_PREFIX, session_id)
    }

    /// Get or initialize the sequence counter for a session.
    /// On first access, scans DB to find the highest existing seq.
    async fn next_seq(&self, session_id: &str) -> u64 {
        // Fast path: counter already loaded
        {
            let counters = self.seq_counters.read().await;
            if let Some(counter) = counters.get(session_id) {
                return counter.fetch_add(1, Ordering::SeqCst) + 1;
            }
        }

        // Slow path: initialize from DB
        let mut counters = self.seq_counters.write().await;
        // Double-check after acquiring write lock
        if let Some(counter) = counters.get(session_id) {
            return counter.fetch_add(1, Ordering::SeqCst) + 1;
        }

        // Scan existing keys to find highest seq
        let prefix = Self::session_prefix(session_id);
        let keys = self.store.list_keys(&prefix).await.unwrap_or_default();
        let max_seq = keys
            .iter()
            .filter_map(|k| k.rsplit('/').next().and_then(|s| s.parse::<u64>().ok()))
            .max()
            .unwrap_or(0);

        // AtomicU64::new(max_seq): fetch_add(1) returns max_seq, so +1 gives max_seq+1
        let counter = AtomicU64::new(max_seq);
        let next = counter.fetch_add(1, Ordering::SeqCst) + 1;
        counters.insert(session_id.to_string(), counter);
        next
    }

    /// Append an event to the log. Returns the assigned sequence number.
    ///
    /// This persists IMMEDIATELY — the event is durable before this function returns.
    pub async fn append(
        &self,
        session_id: &str,
        event_type: &str,
        source: &str,
        payload: serde_json::Value,
    ) -> u64 {
        let seq = self.next_seq(session_id).await;
        let entry = EventEntry {
            seq,
            timestamp: chrono::Utc::now(),
            session_id: session_id.to_string(),
            event_type: event_type.to_string(),
            source: source.to_string(),
            payload,
        };

        let key = Self::event_key(session_id, seq);
        if let Ok(data) = serde_json::to_vec(&entry) {
            let _ = self.store.set(&key, &data).await;
        }

        // Notify subscribers (non-blocking, ok if no subscribers)
        let _ = self.notify_tx.send((session_id.to_string(), seq));

        seq
    }

    /// Query events for a session, starting from `since_seq` (exclusive).
    /// Returns events with seq > since_seq, up to `limit`.
    pub async fn query(&self, session_id: &str, since_seq: u64, limit: usize) -> Vec<EventEntry> {
        let prefix = Self::session_prefix(session_id);
        let keys = self.store.list_keys(&prefix).await.unwrap_or_default();

        let mut entries = Vec::new();
        // Keys are sorted lexicographically, which matches numeric order due to zero-padding
        for key in keys {
            if entries.len() >= limit {
                break;
            }
            // Extract seq from key
            let seq = key
                .rsplit('/')
                .next()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
            if seq <= since_seq {
                continue;
            }
            if let Ok(Some(ref data)) = self.store.get(&key).await {
                if let Ok(entry) = serde_json::from_slice::<EventEntry>(data) {
                    entries.push(entry);
                }
            }
        }
        entries
    }

    /// Get the latest sequence number for a session (0 if no events).
    pub async fn latest_seq(&self, session_id: &str) -> u64 {
        {
            let counters = self.seq_counters.read().await;
            if let Some(counter) = counters.get(session_id) {
                return counter.load(Ordering::SeqCst);
            }
        }

        // Not cached — scan DB
        let prefix = Self::session_prefix(session_id);
        let keys = self.store.list_keys(&prefix).await.unwrap_or_default();
        keys.iter()
            .filter_map(|k| k.rsplit('/').next().and_then(|s| s.parse::<u64>().ok()))
            .max()
            .unwrap_or(0)
    }

    /// Get total event count for a session.
    pub async fn count(&self, session_id: &str) -> usize {
        let prefix = Self::session_prefix(session_id);
        self.store
            .list_keys(&prefix)
            .await
            .unwrap_or_default()
            .len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::tempdir;

    async fn test_log() -> EventLog {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.redb");
        // Leak the tempdir so it outlives the test
        let _dir = Box::leak(Box::new(dir));
        let store = Arc::new(RedbStore::open(db_path).unwrap());
        EventLog::new(store)
    }

    #[tokio::test]
    async fn append_and_query() {
        let log = test_log().await;
        let s1 = log
            .append(
                "sess1",
                "thinking",
                "coordinator",
                serde_json::json!({"iteration": 1}),
            )
            .await;
        let s2 = log
            .append(
                "sess1",
                "tool_call",
                "coordinator",
                serde_json::json!({"tool": "list_agents"}),
            )
            .await;
        assert_eq!(s1, 1);
        assert_eq!(s2, 2);

        let events = log.query("sess1", 0, 100).await;
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, "thinking");
        assert_eq!(events[1].event_type, "tool_call");
    }

    #[tokio::test]
    async fn query_since_seq() {
        let log = test_log().await;
        log.append("sess1", "a", "src", serde_json::json!({})).await;
        log.append("sess1", "b", "src", serde_json::json!({})).await;
        log.append("sess1", "c", "src", serde_json::json!({})).await;

        let events = log.query("sess1", 1, 100).await;
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, "b");
        assert_eq!(events[1].event_type, "c");
    }

    #[tokio::test]
    async fn query_with_limit() {
        let log = test_log().await;
        for i in 0..10 {
            log.append("sess1", &format!("e{}", i), "src", serde_json::json!({}))
                .await;
        }
        let events = log.query("sess1", 0, 3).await;
        assert_eq!(events.len(), 3);
    }

    #[tokio::test]
    async fn sessions_are_isolated() {
        let log = test_log().await;
        log.append("sess1", "a", "src", serde_json::json!({})).await;
        log.append("sess2", "b", "src", serde_json::json!({})).await;

        assert_eq!(log.query("sess1", 0, 100).await.len(), 1);
        assert_eq!(log.query("sess2", 0, 100).await.len(), 1);
    }

    #[tokio::test]
    async fn latest_seq() {
        let log = test_log().await;
        assert_eq!(log.latest_seq("sess1").await, 0);
        log.append("sess1", "a", "src", serde_json::json!({})).await;
        log.append("sess1", "b", "src", serde_json::json!({})).await;
        // After two appends: counter is at 2 (next_seq returned 1 then 2,
        // counter.fetch_add leaves it at 2 after the second call)
        assert_eq!(log.latest_seq("sess1").await, 2);
    }

    #[tokio::test]
    async fn notify_on_append() {
        let log = test_log().await;
        let mut rx = log.subscribe();
        log.append("sess1", "a", "src", serde_json::json!({})).await;
        let (sid, seq) = rx.recv().await.unwrap();
        assert_eq!(sid, "sess1");
        assert_eq!(seq, 1);
    }
}
