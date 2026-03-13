//! Event Bus - Publish-subscribe system for system-wide events.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

/// Maximum number of event subscribers to retain.
const MAX_SUBSCRIBERS: usize = 64;

/// Unique event identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(pub Uuid);

impl EventId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

/// Timestamp for events.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EventTimestamp(pub chrono::DateTime<chrono::Utc>);

impl EventTimestamp {
    pub fn now() -> Self {
        Self(chrono::Utc::now())
    }
}

/// Core system events.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SystemEvent {
    /// A task was delegated from one agent to another.
    TaskDelegated {
        task_id: super::TaskId,
        application_id: String,
        from_agent: String,
        to_agent: String,
        prompt: String,
    },

    /// A task started execution.
    TaskStarted {
        task_id: String,
        agent: String,
    },

    /// A task made progress.
    TaskProgress {
        task_id: String,
        step: usize,
        output: String,
    },

    /// A task completed successfully.
    TaskCompleted {
        task_id: String,
        agent: String,
        success: bool,
        output_preview: String,
    },

    /// A task failed.
    TaskFailed {
        task_id: String,
        agent: String,
        error: String,
    },

    /// A task was cancelled.
    TaskCancelled {
        task_id: String,
    },

    /// An agent was spawned/registered.
    AgentSpawned {
        agent_id: String,
        name: String,
        capabilities: Vec<String>,
    },

    /// An agent became idle (finished all tasks).
    AgentIdle {
        agent_id: String,
        name: String,
    },

    /// An agent started working on a task.
    AgentBusy {
        agent_id: String,
        name: String,
        task_id: String,
    },

    /// An agent is now available (was busy, now idle).
    AgentAvailable {
        agent_id: String,
        name: String,
    },
}

impl SystemEvent {
    /// Get the event type name.
    pub fn event_type(&self) -> &'static str {
        match self {
            SystemEvent::TaskDelegated { .. } => "task_delegated",
            SystemEvent::TaskStarted { .. } => "task_started",
            SystemEvent::TaskProgress { .. } => "task_progress",
            SystemEvent::TaskCompleted { .. } => "task_completed",
            SystemEvent::TaskFailed { .. } => "task_failed",
            SystemEvent::TaskCancelled { .. } => "task_cancelled",
            SystemEvent::AgentSpawned { .. } => "agent_spawned",
            SystemEvent::AgentIdle { .. } => "agent_idle",
            SystemEvent::AgentBusy { .. } => "agent_busy",
            SystemEvent::AgentAvailable { .. } => "agent_available",
        }
    }

    /// Get the related task ID if applicable.
    pub fn task_id(&self) -> Option<String> {
        match self {
            SystemEvent::TaskDelegated { task_id, .. } => Some(task_id.to_string()),
            SystemEvent::TaskStarted { task_id, .. } => Some(task_id.clone()),
            SystemEvent::TaskProgress { task_id, .. } => Some(task_id.clone()),
            SystemEvent::TaskCompleted { task_id, .. } => Some(task_id.clone()),
            SystemEvent::TaskFailed { task_id, .. } => Some(task_id.clone()),
            SystemEvent::TaskCancelled { task_id, .. } => Some(task_id.clone()),
            _ => None,
        }
    }

    /// Get the related agent name if applicable.
    pub fn agent_name(&self) -> Option<&str> {
        match self {
            SystemEvent::TaskDelegated { to_agent, .. } => Some(to_agent),
            SystemEvent::TaskStarted { agent, .. } => Some(agent),
            SystemEvent::TaskCompleted { agent, .. } => Some(agent),
            SystemEvent::TaskFailed { agent, .. } => Some(agent),
            SystemEvent::AgentIdle { name, .. } => Some(name),
            SystemEvent::AgentBusy { name, .. } => Some(name),
            SystemEvent::AgentAvailable { name, .. } => Some(name),
            _ => None,
        }
    }
}

/// A wrapped event with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Unique event identifier.
    pub id: EventId,
    /// Event timestamp.
    pub timestamp: EventTimestamp,
    /// The event payload.
    pub payload: SystemEvent,
}

impl Event {
    /// Create a new event.
    pub fn new(payload: SystemEvent) -> Self {
        Self {
            id: EventId::new(),
            timestamp: EventTimestamp::now(),
            payload,
        }
    }
}

/// Trait for event subscribers.
#[async_trait]
pub trait EventSubscriber: Send + Sync {
    /// Get the subscriber's name/identifier.
    fn name(&self) -> &str;

    /// Handle an incoming event.
    async fn on_event(&self, event: Event);

    /// Check if the subscriber wants to receive a specific event type.
    fn is_interested_in(&self, event_type: &str) -> bool;

    /// Clone the subscriber into a boxed trait object.
    fn clone_box(&self) -> Box<dyn EventSubscriber>;
}

/// Event bus for publish-subscribe.
pub struct EventBus {
    /// Broadcast channel for events.
    tx: broadcast::Sender<Event>,
    /// Subscribers registry.
    subscribers: Arc<RwLock<Vec<Box<dyn EventSubscriber>>>>,
    /// Event history (for late subscribers).
    history: Arc<RwLock<Vec<Event>>>,
    /// Maximum history size.
    max_history: usize,
}

impl EventBus {
    /// Create a new event bus.
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(MAX_SUBSCRIBERS);
        Self {
            tx,
            subscribers: Arc::new(RwLock::new(Vec::new())),
            history: Arc::new(RwLock::new(Vec::new())),
            max_history: 100,
        }
    }

    /// Create with custom history size.
    pub fn with_history_size(mut self, size: usize) -> Self {
        self.max_history = size;
        self
    }

    /// Subscribe to events.
    pub async fn subscribe(&self, subscriber: Box<dyn EventSubscriber>) {
        let mut subs = self.subscribers.write().await;
        tracing::info!(subscriber = subscriber.name(), "Subscriber added");
        subs.push(subscriber);
    }

    /// Unsubscribe by name.
    pub async fn unsubscribe(&self, name: &str) {
        let mut subs = self.subscribers.write().await;
        subs.retain(|s| s.name() != name);
    }

    /// Publish an event to all subscribers.
    pub async fn publish(&self, event: Event) {
        // Store in history
        {
            let mut history = self.history.write().await;
            history.push(event.clone());
            if history.len() > self.max_history {
                history.remove(0);
            }
        }

        // Broadcast to all subscribers
        if let Err(e) = self.tx.send(event.clone()) {
            tracing::warn!("No active receivers: {}", e);
        }

        // Also dispatch to registered subscribers
        // Get subscribers to notify while holding the lock
        let subscriber_handles: Vec<(String, Box<dyn EventSubscriber>)> = {
            let subs = self.subscribers.read().await;
            subs.iter()
                .filter(|s| s.is_interested_in(event.payload.event_type()))
                .map(|s| (s.name().to_string(), s.as_ref().clone_box()))
                .collect()
        };

        // Now notify each subscriber (lock is released)
        for (name, subscriber) in subscriber_handles {
            let evt = event.clone();
            tokio::spawn(async move {
                subscriber.on_event(evt).await;
            });
        }
    }

    /// Publish a system event (convenience method).
    pub async fn emit(&self, payload: SystemEvent) {
        let event = Event::new(payload);
        self.publish(event).await;
    }

    /// Get event history.
    pub async fn history(&self) -> Vec<Event> {
        self.history.read().await.clone()
    }

    /// Get recent events of a specific type.
    pub async fn recent_of_type(&self, event_type: &str, count: usize) -> Vec<Event> {
        let history = self.history.read().await;
        history
            .iter()
            .rev()
            .filter(|e| e.payload.event_type() == event_type)
            .take(count)
            .cloned()
            .collect()
    }

    /// Get subscriber count.
    pub async fn subscriber_count(&self) -> usize {
        self.subscribers.read().await.len()
    }

    /// Create a subscriber handle for receiving events.
    pub fn subscribe_to_broadcast(&self) -> broadcast::Receiver<Event> {
        self.tx.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to create event subscribers.
#[macro_export]
macro_rules! subscriber {
    ($name:expr, |$event:ident| $body:expr) => {
        struct Subscriber<F> {
            name: String,
            handler: std::sync::Arc<F>,
        }

        #[async_trait::async_trait]
        impl<F> $crate::executor::EventSubscriber for Subscriber<F>
        where
            F: Fn($crate::executor::Event) -> futures::future::BoxFuture<'static, ()> + Send + Sync + 'static,
        {
            fn name(&self) -> &str {
                &self.name
            }

            async fn on_event(&self, $event: $crate::executor::Event) {
                (self.handler)($event).await;
            }

            fn is_interested_in(&self, event_type: &str) -> bool {
                true // Receive all events
            }
        }

        Subscriber {
            name: $name.to_string(),
            handler: std::sync::Arc::new(|event| {
                async move { $body }.boxed()
            }),
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::broadcast;

    #[tokio::test]
    async fn test_publish_subscribe() {
        let bus = EventBus::new();

        let received = Arc::new(std::sync::Mutex::new(Vec::new()));
        let received_clone = received.clone();

        // Simple subscriber
        let subscriber = Box::new(TestSubscriber {
            name: "test".to_string(),
            received: received_clone,
        });
        bus.subscribe(subscriber).await;

        // Publish event
        bus.emit(SystemEvent::TaskStarted {
            task_id: "test-1".to_string(),
            agent: "backend".to_string(),
        }).await;

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let events = received.lock().unwrap();
        assert_eq!(events.len(), 1);
    }

    struct TestSubscriber {
        name: String,
        received: Arc<std::sync::Mutex<Vec<Event>>>,
    }

    #[async_trait]
    impl EventSubscriber for TestSubscriber {
        fn name(&self) -> &str {
            &self.name
        }

        async fn on_event(&self, event: Event) {
            self.received.lock().unwrap().push(event);
        }

        fn is_interested_in(&self, _event_type: &str) -> bool {
            true
        }
    }
}
