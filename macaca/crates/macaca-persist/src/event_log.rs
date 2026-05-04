//! Append-only event log backed by RedbStore.
//!
//! Every event is stored at `events/{session_id}/{seq:08d}` as a JSON blob.
//! Events are never modified or deleted during normal operation.

use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::event_index::{
    agent_from_payload, event_matches, index_key, index_prefix_for_query, select_index,
    seq_from_key, EventLogQuery, SelectedIndex,
};
use crate::store::PersistBackend;
use macaca_proto::types::EventEntry;

const EVENTS_PREFIX: &str = "events/";

/// Command object for appending a single persisted event.
#[derive(Debug, Clone)]
pub struct AppendEventCommand {
    pub session_id: String,
    pub event_type: String,
    pub source: String,
    pub payload: serde_json::Value,
    pub app_id: Option<String>,
    pub agent_name: Option<String>,
}

impl AppendEventCommand {
    pub fn new(
        session_id: impl Into<String>,
        event_type: impl Into<String>,
        source: impl Into<String>,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            event_type: event_type.into(),
            source: source.into(),
            payload,
            app_id: None,
            agent_name: None,
        }
    }

    pub fn with_app_id(mut self, app_id: impl Into<String>) -> Self {
        self.app_id = Some(app_id.into());
        self
    }

    pub fn with_agent_name(mut self, agent_name: impl Into<String>) -> Self {
        self.agent_name = Some(agent_name.into());
        self
    }
}

/// Stable replay primitive for ordered session event restoration.
pub struct EventReplayIterator {
    entries: VecDeque<EventEntry>,
}

impl EventReplayIterator {
    fn new(entries: Vec<EventEntry>) -> Self {
        Self {
            entries: entries.into(),
        }
    }
}

impl Iterator for EventReplayIterator {
    type Item = EventEntry;

    fn next(&mut self) -> Option<Self::Item> {
        self.entries.pop_front()
    }
}

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
    store: Arc<dyn PersistBackend>,
    /// Per-session sequence counters. Loaded from DB on first access.
    seq_counters: RwLock<HashMap<String, AtomicU64>>,
    /// Broadcast notification when new events are appended.
    /// Sends (session_id, latest_seq).
    notify_tx: tokio::sync::broadcast::Sender<(String, u64)>,
}

impl EventLog {
    pub fn new<T>(store: Arc<T>) -> Self
    where
        T: PersistBackend + 'static,
    {
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

    async fn write_event_indexes(
        &self,
        entry: &EventEntry,
        canonical_key: &str,
        agent: Option<&str>,
    ) {
        self.write_index(
            SelectedIndex::Source,
            &entry.session_id,
            &entry.source,
            entry.seq,
            canonical_key,
        )
        .await;
        self.write_index(
            SelectedIndex::EventType,
            &entry.session_id,
            &entry.event_type,
            entry.seq,
            canonical_key,
        )
        .await;
        if let Some(agent) = agent.filter(|agent| !agent.is_empty()) {
            self.write_index(
                SelectedIndex::Agent,
                &entry.session_id,
                agent,
                entry.seq,
                canonical_key,
            )
            .await;
        }
    }

    async fn write_index(
        &self,
        index: SelectedIndex,
        session_id: &str,
        value: &str,
        seq: u64,
        canonical_key: &str,
    ) {
        let key = index_key(index, session_id, value, seq);
        let _ = self.store.set(&key, canonical_key.as_bytes()).await;
    }

    async fn get_entry(&self, key: &str) -> Option<EventEntry> {
        match self.store.get(key).await {
            Ok(Some(data)) => serde_json::from_slice::<EventEntry>(&data).ok(),
            _ => None,
        }
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

    /// Append an event command to the log. Returns the assigned sequence number.
    ///
    /// This persists IMMEDIATELY — the event is durable before this function returns.
    pub async fn append_command(&self, command: AppendEventCommand) -> u64 {
        let seq = self.next_seq(&command.session_id).await;
        let agent_name = command
            .agent_name
            .clone()
            .or_else(|| agent_from_payload(&command.payload));
        let entry = EventEntry {
            seq,
            timestamp: chrono::Utc::now(),
            session_id: command.session_id.clone(),
            event_type: command.event_type,
            source: command.source,
            payload: command.payload,
        };

        let key = Self::event_key(&command.session_id, seq);
        if let Ok(data) = serde_json::to_vec(&entry) {
            let _ = self.store.set(&key, &data).await;
            self.write_event_indexes(&entry, &key, agent_name.as_deref())
                .await;
        }

        // Notify subscribers (non-blocking, ok if no subscribers)
        let _ = self.notify_tx.send((command.session_id, seq));

        seq
    }

    /// Append an event to the log. Returns the assigned sequence number.
    ///
    /// Kept for compatibility; internally delegates to `AppendEventCommand`.
    pub async fn append(
        &self,
        session_id: &str,
        event_type: &str,
        source: &str,
        payload: serde_json::Value,
    ) -> u64 {
        self.append_command(AppendEventCommand::new(
            session_id, event_type, source, payload,
        ))
        .await
    }

    /// Replay events for a session, starting from `since_seq` (exclusive).
    pub async fn replay(
        &self,
        session_id: &str,
        since_seq: u64,
        limit: usize,
    ) -> EventReplayIterator {
        let prefix = Self::session_prefix(session_id);
        let keys = self.store.list_keys(&prefix).await.unwrap_or_default();

        let mut entries = Vec::new();
        for key in keys {
            if entries.len() >= limit {
                break;
            }
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

        EventReplayIterator::new(entries)
    }

    /// Query events for a session, starting from `since_seq` (exclusive).
    /// Returns events with seq > since_seq, up to `limit`.
    pub async fn query(&self, session_id: &str, since_seq: u64, limit: usize) -> Vec<EventEntry> {
        self.replay(session_id, since_seq, limit).await.collect()
    }

    /// Query events for a session using secondary indexes when a scope filter is present.
    pub async fn query_indexed(&self, query: EventLogQuery) -> Vec<EventEntry> {
        if query.limit == 0 {
            return Vec::new();
        }
        let Some(selected) = select_index(&query) else {
            return self
                .replay(&query.session_id, query.since_seq, query.limit)
                .await
                .collect();
        };
        let Some(prefix) = index_prefix_for_query(&query, selected) else {
            return Vec::new();
        };
        let keys = self.store.list_keys(&prefix).await.unwrap_or_default();
        let mut entries = Vec::new();
        for key in keys {
            if entries.len() >= query.limit {
                break;
            }
            let seq = seq_from_key(&key);
            if seq <= query.since_seq {
                continue;
            }
            let Some(pointer) = self.store.get(&key).await.ok().flatten() else {
                continue;
            };
            let Ok(canonical_key) = String::from_utf8(pointer) else {
                continue;
            };
            if let Some(entry) = self.get_entry(&canonical_key).await {
                if event_matches(&entry, &query, Some(selected)) {
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
