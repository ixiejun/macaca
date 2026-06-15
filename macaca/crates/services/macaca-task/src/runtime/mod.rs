//! Local Task Service runtime skeleton.
//!
//! Split from monolithic `runtime.rs` (P5 iteration 87) using a Facade module tree.
//! Host-side orchestration for task commands, service events, and snapshots.
//! Compatible with existing TaskBoard and Plan/Worker loops; execution hooks are
//! pluggable via [`TaskServiceExecutionStrategy`].
//!
//! # Design patterns
//! - **Facade**: `mod.rs` re-exports preserve `macaca_task::runtime::*`.
//! - **Strategy**: [`TaskServiceExecutionStrategy`] replaces planner/worker hooks.
//! - **Command**: command DTO handlers per lifecycle concern (goal/assignment/task).
//! - **Observer**: [`TaskServiceEventSink`] publishes audit events.
//! - **Memento**: in-memory [`TaskServiceSnapshot`] cache per app/session key.
//! - **Specification**: graph admission rules for authoritative execution graphs.

mod assignment_commands;
mod coordinator_commands;
mod event_sink;
mod goal_commands;
mod graph_admission;
mod prompt_commands;
mod snapshot;
mod strategy;
mod task_lifecycle_commands;

#[cfg(test)]
mod tests;

pub use event_sink::{InMemoryTaskServiceEventSink, TaskServiceEventSink};
pub use strategy::{NoopTaskServiceExecutionStrategy, TaskServiceExecutionStrategy};

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use tracing::info;

use crate::events::{TaskServiceEvent, TaskServiceSnapshot};
use crate::todo_store::TodoStore;

use strategy::NoopTaskServiceExecutionStrategy as DefaultStrategy;

/// Runtime-owned Task Service facade (generic over execution [`Strategy`](TaskServiceExecutionStrategy)).
pub struct TaskServiceRuntime<S = DefaultStrategy> {
    pub(crate) store: Arc<TodoStore>,
    pub(crate) execution: Arc<S>,
    pub(crate) event_sink: Arc<dyn TaskServiceEventSink>,
    pub(crate) snapshots: RwLock<BTreeMap<(String, Option<String>), TaskServiceSnapshot>>,
}

impl<S> TaskServiceRuntime<S>
where
    S: TaskServiceExecutionStrategy + 'static,
{
    pub fn new(
        store: Arc<TodoStore>,
        execution: Arc<S>,
        event_sink: Arc<dyn TaskServiceEventSink>,
    ) -> Self {
        Self {
            store,
            execution,
            event_sink,
            snapshots: RwLock::new(BTreeMap::new()),
        }
    }

    /// Emit a structured event through the configured sink (shared by command handler submodules).
    pub(crate) async fn emit(&self, event: TaskServiceEvent) {
        info!(
            app_id = %event.app_id.0,
            session_id = ?event.session_id,
            event_type = ?event.event_type,
            "task service event emitted"
        );
        self.event_sink.publish(event).await;
    }
}
