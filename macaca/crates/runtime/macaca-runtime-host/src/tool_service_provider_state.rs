//! In-memory caches for Tool planning snapshots.
//!
//! These caches are Memento stores for bounded planning evidence.  They never
//! store raw provider payloads, credentials, or tool outputs.

use std::collections::BTreeMap;
use std::sync::RwLock;

use macaca_proto::{
    IndustrialToolDescriptor, ServiceHealth, ToolArtifactRef, ToolCatalogPlanResult,
    ToolCommandResult, ToolInvocationRef, ToolServiceSnapshotResult, TraceContext,
};

#[derive(Debug, Default)]
pub struct ToolServiceProviderState {
    last_plan: RwLock<Option<ToolCatalogPlanResult>>,
    provider_count: RwLock<usize>,
    invocation_results: RwLock<BTreeMap<String, ToolCommandResult>>,
    artifact_results: RwLock<BTreeMap<String, serde_json::Value>>,
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

    /// Resolve a planned descriptor by stable id from the last bounded plan.
    ///
    /// This is a compatibility fallback for callers that send only a stable
    /// `tool_id`.  New framework adapters pass the descriptor selected from the
    /// model-visible plan so invocation routing does not depend on a stale
    /// cache.  The cache stores only sanitized descriptors, never raw tool
    /// input/output or provider payloads.
    pub fn descriptor_by_tool_id(&self, tool_id: &str) -> Option<IndustrialToolDescriptor> {
        let plan = self
            .last_plan
            .read()
            .expect("tool plan cache lock poisoned");
        let plan = plan.as_ref()?;
        plan.visible
            .iter()
            .chain(plan.hidden.iter())
            .find(|entry| entry.descriptor.stable_tool_id == tool_id)
            .map(|entry| entry.descriptor.clone())
    }

    /// Store the normalized command result under its invocation ref.
    ///
    /// The state acts as a small in-memory Memento store for this proposal
    /// slice.  Later artifact providers can replace the backing store without
    /// changing `service.tool` command/result contracts.
    pub fn record_invocation_result(&self, result: ToolCommandResult) {
        let Some(invocation_ref) = result.invocation_ref.as_ref() else {
            return;
        };
        self.invocation_results
            .write()
            .expect("tool invocation result cache lock poisoned")
            .insert(invocation_ref.0.clone(), result);
    }

    pub fn invocation_result(
        &self,
        invocation_ref: &ToolInvocationRef,
    ) -> Option<ToolCommandResult> {
        self.invocation_results
            .read()
            .expect("tool invocation result cache lock poisoned")
            .get(&invocation_ref.0)
            .cloned()
    }

    pub fn record_artifact(&self, artifact_ref: &ToolArtifactRef, payload: serde_json::Value) {
        self.artifact_results
            .write()
            .expect("tool artifact cache lock poisoned")
            .insert(artifact_ref.0.clone(), payload);
    }

    pub fn artifact(&self, artifact_ref: &ToolArtifactRef) -> Option<serde_json::Value> {
        self.artifact_results
            .read()
            .expect("tool artifact cache lock poisoned")
            .get(&artifact_ref.0)
            .cloned()
    }
}
