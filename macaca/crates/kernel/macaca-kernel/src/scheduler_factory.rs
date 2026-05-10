//! Scheduler factory primitives.

use crate::scheduler::Scheduler;
#[allow(deprecated)]
use crate::scheduler::SimpleScheduler;

/// Kernel scheduler strategy identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerKind {
    /// Current default capability/fallback scheduler.
    Simple,
}

impl Default for SchedulerKind {
    fn default() -> Self {
        Self::Simple
    }
}

/// Constructs scheduler strategies from stable identifiers.
pub struct SchedulerFactory;

impl SchedulerFactory {
    /// Build a scheduler strategy.
    #[allow(deprecated)]
    pub fn build(kind: SchedulerKind) -> Box<dyn Scheduler> {
        match kind {
            SchedulerKind::Simple => Box::new(SimpleScheduler),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use macaca_proto::{Task, TaskId, TaskPriority, TaskStatus};

    use crate::registry::AgentRegistry;

    fn make_task(description: &str) -> Task {
        Task {
            id: TaskId::new(),
            description: description.into(),
            status: TaskStatus::Pending,
            priority: TaskPriority::Normal,
            assigned_agent: None,
            subtasks: vec![],
            parent: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn default_factory_returns_simple_scheduler_behavior() {
        let registry = AgentRegistry::new(10);
        let task = make_task("anything");
        let scheduler = SchedulerFactory::build(SchedulerKind::default());

        let selected = scheduler.select_agent(&registry, &task).await.unwrap();
        assert_eq!(selected, None);
    }
}
