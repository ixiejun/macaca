//! In-process Scheduler provider (**Facade** module root).
//!
//! `InProcessSchedulerProvider` is the built-in Scheduler service implementation
//! registered by runtime-host when local autonomy mode is selected.  The type
//! name uses the in-process prefix so escape-hatch gates can freeze the retired
//! local-provider identifier while approved composition roots keep a neutral label.
//!
//! This provider is the first concrete Scheduler implementation slice.  It is
//! intentionally conservative: it stores provider-neutral job definitions,
//! materializes due runs into an auditable run-history memento, and exposes
//! health/snapshot/history.  It does not execute target commands yet.  Dispatch
//! must be added later through service-runtime decorators so policy, resource,
//! trace, and audit gates wrap every side effect.
//!
//! # Module tree (P5 iteration 93)
//! - `support` — constants + accepted/rejected result builders
//! - `store` — in-memory Memento store (jobs, runs, audit ids)
//! - `materialization` — due-run refresh + snapshot assembly
//! - `service` — `SchedulerService` trait implementation
//! - `run_control` — lease, retry, cancellation, expiry transitions
//! - `schedule` — Strategy schedule calculation
//! - `tests` — contract tests

mod materialization;
mod run_control;
mod schedule;
mod service;
mod store;
mod support;

#[cfg(test)]
mod tests;

use macaca_proto::{ServiceDescriptor, ServiceHealth, ServiceLifecycleState};

use crate::service_contract::SchedulerService;

use schedule::DefaultScheduleCalculator;
use store::InMemorySchedulerStore;
use support::LOCAL_PROVIDER_ID;

/// In-process Scheduler provider backed by an in-memory memento store.
///
/// The provider uses the Memento pattern for job/run state, the State pattern
/// for job and run lifecycle values, the Strategy pattern for schedule
/// calculation, and Observer-style `tracing` logs for key execution nodes.
pub struct InProcessSchedulerProvider {
    descriptor: ServiceDescriptor,
    store: InMemorySchedulerStore,
    calculator: DefaultScheduleCalculator,
}

/// Leased Scheduler run plus the generic target data required by dispatchers.
///
/// This memento is intentionally payload-reference-only.  Runtime-host can use
/// it to dispatch through service boundaries, but the Scheduler still does not
/// execute application behavior or expose raw target payloads in snapshots.
#[derive(Debug, Clone)]
pub struct InProcessSchedulerLeasedRun {
    pub summary: macaca_proto::SchedulerRunSummary,
    pub scope: macaca_proto::AutonomyScope,
    pub target: macaca_proto::SchedulerTargetCommand,
}

impl InProcessSchedulerProvider {
    /// Create an empty local provider with the standard Scheduler descriptor.
    pub fn new() -> Self {
        let unavailable = crate::UnavailableSchedulerProvider::default();
        let mut descriptor = unavailable.descriptor();
        descriptor.health = ServiceHealth::Healthy;
        descriptor.lifecycle_state = ServiceLifecycleState::Running;
        descriptor
            .metadata
            .insert("provider_id".into(), LOCAL_PROVIDER_ID.into());
        Self {
            descriptor,
            store: InMemorySchedulerStore::default(),
            calculator: DefaultScheduleCalculator,
        }
    }
}

impl Default for InProcessSchedulerProvider {
    fn default() -> Self {
        Self::new()
    }
}
