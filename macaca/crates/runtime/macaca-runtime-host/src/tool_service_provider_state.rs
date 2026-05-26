//! In-memory caches for Tool planning snapshots.
//!
//! These caches are Memento stores for bounded planning evidence.  They never
//! store raw provider payloads, credentials, or tool outputs.

use std::sync::RwLock;

use macaca_proto::{ServiceHealth, ToolCatalogPlanResult, ToolServiceSnapshotResult, TraceContext};

#[derive(Debug, Default)]
pub struct ToolServiceProviderState {
    last_plan: RwLock<Option<ToolCatalogPlanResult>>,
    provider_count: RwLock<usize>,
}

impl ToolServiceProviderState {
    pub fn record_plan(&self, plan: ToolCatalogPlanResult, provider_count: usize) {
        *self
            .last_plan
            .write()
            .expect("tool plan cache lock poisoned") = Some(plan);
        *self
            .provider_count
            .write()
            .expect("tool provider cache lock poisoned") = provider_count;
    }

    pub fn snapshot(&self, trace: TraceContext) -> ToolServiceSnapshotResult {
        let provider_count = *self
            .provider_count
            .read()
            .expect("tool provider cache lock poisoned");
        let plan = self
            .last_plan
            .read()
            .expect("tool plan cache lock poisoned");
        let plan = plan.as_ref().map(|plan| macaca_proto::ToolPlan {
            visible: plan.visible.clone(),
            hidden: plan.hidden.clone(),
            conflicts: plan.conflicts.clone(),
        });
        ToolServiceSnapshotResult {
            trace,
            health: ServiceHealth::Healthy,
            plan: plan.unwrap_or(macaca_proto::ToolPlan {
                visible: Vec::new(),
                hidden: Vec::new(),
                conflicts: Vec::new(),
            }),
            provider_count,
            captured_at: chrono::Utc::now(),
            metadata: Default::default(),
        }
    }
}
