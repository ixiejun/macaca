//! Deterministic ToolPlan builder for `service.tool`.
//!
//! The planner uses Adapter contributors, Strategy-style toolset resolution,
//! and Specification availability checks.  It plans metadata only: provider
//! lifecycles and invocation remain owned by Driver, Skill, MCP, Memory,
//! Scheduler, Gateway, and future provider services.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    CapabilityToolDescriptor, CapabilityToolOriginKind, IndustrialToolDescriptor, MacacaResult,
    ServiceHealth, ToolAuditRef, ToolCatalogPlanCommand, ToolCatalogPlanResult,
    ToolConflictDiagnostic, ToolFamilyRef, ToolHiddenReason, ToolPlanEntry, ToolsetRef,
};

use crate::tool_service_availability::{AvailabilitySignalSet, ToolAvailabilityEvaluator};

#[async_trait]
pub trait ToolDescriptorContributor: Send + Sync {
    fn contributor_id(&self) -> &str;
    fn health(&self) -> ServiceHealth;
    async fn descriptors(&self) -> MacacaResult<Vec<IndustrialToolDescriptor>>;
}

/// Adapter contributor for existing service-owned capability descriptors.
///
/// Driver, Skill, MCP, Memory, Task, Scheduler, Gateway, workspace, and future
/// provider services can expose the shared `CapabilityToolDescriptor` envelope
/// without giving `service.tool` ownership of their lifecycle or invocation.
/// The contributor only enriches those descriptors into planning metadata.
#[derive(Clone)]
pub struct CapabilityToolDescriptorContributor {
    contributor_id: String,
    health: ServiceHealth,
    descriptors: Vec<CapabilityToolDescriptor>,
}

impl CapabilityToolDescriptorContributor {
    pub fn new(
        contributor_id: impl Into<String>,
        health: ServiceHealth,
        descriptors: Vec<CapabilityToolDescriptor>,
    ) -> Self {
        Self {
            contributor_id: contributor_id.into(),
            health,
            descriptors,
        }
    }
}

#[async_trait]
impl ToolDescriptorContributor for CapabilityToolDescriptorContributor {
    fn contributor_id(&self) -> &str {
        &self.contributor_id
    }

    fn health(&self) -> ServiceHealth {
        self.health.clone()
    }

    async fn descriptors(&self) -> MacacaResult<Vec<IndustrialToolDescriptor>> {
        self.descriptors
            .iter()
            .cloned()
            .map(industrial_from_capability_descriptor)
            .collect()
    }
}

#[derive(Clone)]
pub struct StaticToolDescriptorContributor {
    contributor_id: String,
    health: ServiceHealth,
    descriptors: Vec<IndustrialToolDescriptor>,
}

fn industrial_from_capability_descriptor(
    descriptor: CapabilityToolDescriptor,
) -> MacacaResult<IndustrialToolDescriptor> {
    let family = descriptor
        .metadata
        .get("tool.family")
        .cloned()
        .unwrap_or_else(|| default_family(&descriptor.origin_kind).to_string());
    let title = descriptor
        .display_name
        .clone()
        .unwrap_or_else(|| descriptor.tool_name.clone());
    let stable_tool_id = descriptor
        .metadata
        .get("tool.stable_id")
        .cloned()
        .unwrap_or_else(|| {
            format!(
                "tool.{}.{}.{}",
                descriptor.service_id, descriptor.provider_id, descriptor.tool_name
            )
        });
    let toolsets = descriptor
        .metadata
        .get("tool.toolsets")
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(ToolsetRef::new)
                .collect::<MacacaResult<Vec<_>>>()
        })
        .transpose()?
        .unwrap_or_default();

    let mut industrial = IndustrialToolDescriptor::new(
        stable_tool_id,
        descriptor.tool_name.clone(),
        title,
        ToolFamilyRef::new(family)?,
        descriptor,
    )?;
    industrial.toolsets = toolsets;
    Ok(industrial)
}

fn default_family(origin_kind: &CapabilityToolOriginKind) -> &'static str {
    match origin_kind {
        CapabilityToolOriginKind::Driver => "driver",
        CapabilityToolOriginKind::Skill => "skill",
        CapabilityToolOriginKind::Mcp => "mcp",
    }
}

impl StaticToolDescriptorContributor {
    pub fn new(
        contributor_id: impl Into<String>,
        health: ServiceHealth,
        descriptors: Vec<IndustrialToolDescriptor>,
    ) -> Self {
        Self {
            contributor_id: contributor_id.into(),
            health,
            descriptors,
        }
    }
}

#[async_trait]
impl ToolDescriptorContributor for StaticToolDescriptorContributor {
    fn contributor_id(&self) -> &str {
        &self.contributor_id
    }

    fn health(&self) -> ServiceHealth {
        self.health.clone()
    }

    async fn descriptors(&self) -> MacacaResult<Vec<IndustrialToolDescriptor>> {
        Ok(self.descriptors.clone())
    }
}

#[derive(Debug, Clone, Default)]
pub struct ToolPlanningToolsetResolver {
    families_by_toolset: BTreeMap<String, BTreeSet<String>>,
}

impl ToolPlanningToolsetResolver {
    pub fn with_toolset<I, S>(mut self, toolset: impl Into<String>, families: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.families_by_toolset.insert(
            toolset.into(),
            families.into_iter().map(Into::into).collect(),
        );
        self
    }

    fn resolve(&self, toolsets: &[ToolsetRef]) -> BTreeSet<String> {
        let mut families = BTreeSet::new();
        for toolset in toolsets {
            if let Some(items) = self.families_by_toolset.get(toolset.as_str()) {
                families.extend(items.iter().cloned());
            }
        }
        families
    }
}

pub struct ToolPlanningService {
    contributors: Vec<Arc<dyn ToolDescriptorContributor>>,
    availability: ToolAvailabilityEvaluator,
    toolsets: ToolPlanningToolsetResolver,
}

impl ToolPlanningService {
    pub fn builder() -> ToolPlanningServiceBuilder {
        ToolPlanningServiceBuilder::default()
    }

    pub async fn plan(
        &self,
        command: ToolCatalogPlanCommand,
    ) -> MacacaResult<ToolCatalogPlanResult> {
        let mut visible = Vec::new();
        let mut hidden = Vec::new();
        let mut reason_counts = BTreeMap::<String, usize>::new();
        let mut descriptors = Vec::new();
        let contributor_count = self.contributors.len();
        for contributor in &self.contributors {
            match contributor.health() {
                ServiceHealth::Unavailable { reason }
                | ServiceHealth::DisabledByPolicy { reason } => {
                    tracing::warn!(
                        trace_id = %command.trace.trace_id,
                        contributor_id = contributor.contributor_id(),
                        reason,
                        "tool planner contributor unavailable"
                    );
                    continue;
                }
                _ => descriptors.extend(contributor.descriptors().await?),
            }
        }

        let requested_families = self.requested_families(&command);
        let allowed_tools: BTreeSet<_> = command.allowed_tools.iter().cloned().collect();
        let denied_families: BTreeSet<_> = command
            .denied_families
            .iter()
            .map(|family| family.as_str().to_string())
            .collect();

        for descriptor in descriptors {
            if !requested_families.is_empty()
                && !requested_families.contains(descriptor.family.as_str())
            {
                continue;
            }
            let hidden_reason =
                if !allowed_tools.is_empty() && !allowed_tools.contains(&descriptor.visible_name) {
                    Some(ToolHiddenReason::PolicyDenied)
                } else if denied_families.contains(descriptor.family.as_str()) {
                    Some(ToolHiddenReason::PolicyDenied)
                } else {
                    self.availability.hidden_reason(&descriptor.availability)
                };
            if let Some(reason) = hidden_reason {
                *reason_counts.entry(reason_code(&reason)).or_default() += 1;
                if command.include_hidden {
                    hidden.push(ToolPlanEntry::hidden(descriptor, reason, Vec::new()));
                }
            } else {
                visible.push(ToolPlanEntry::visible(descriptor, vec!["planned".into()]));
            }
        }
        let conflicts = conflicts(&visible);
        let mut metadata = BTreeMap::new();
        metadata.insert(
            "tool.contributor_count".into(),
            contributor_count.to_string(),
        );
        metadata.insert("tool.visible_count".into(), visible.len().to_string());
        metadata.insert("tool.hidden_count".into(), hidden.len().to_string());
        metadata.insert("tool.conflict_count".into(), conflicts.len().to_string());
        for (reason, count) in reason_counts {
            metadata.insert(format!("tool.reason.{reason}"), count.to_string());
        }
        tracing::info!(
            trace_id = %command.trace.trace_id,
            application_id = command.application_id.as_ref().map(|id| id.0.to_string()).unwrap_or_else(|| "none".into()),
            agent_name = command.agent_name.as_deref().unwrap_or("none"),
            contributor_count,
            visible_count = visible.len(),
            hidden_count = hidden.len(),
            conflict_count = conflicts.len(),
            "tool catalog plan completed"
        );
        Ok(ToolCatalogPlanResult {
            trace: command.trace,
            visible,
            hidden,
            conflicts,
            audit_refs: vec![ToolAuditRef::new("tool.plan.local")],
            captured_at: chrono::Utc::now(),
            metadata,
        })
    }

    fn requested_families(&self, command: &ToolCatalogPlanCommand) -> BTreeSet<String> {
        let mut families = self.toolsets.resolve(&command.requested_toolsets);
        families.extend(
            command
                .requested_families
                .iter()
                .map(|family| family.as_str().to_string()),
        );
        families
    }
}

#[derive(Default)]
pub struct ToolPlanningServiceBuilder {
    contributors: Vec<Arc<dyn ToolDescriptorContributor>>,
    availability: AvailabilitySignalSet,
    toolsets: ToolPlanningToolsetResolver,
}

impl ToolPlanningServiceBuilder {
    pub fn with_contributor<C>(mut self, contributor: C) -> Self
    where
        C: ToolDescriptorContributor + 'static,
    {
        self.contributors.push(Arc::new(contributor));
        self
    }

    pub fn with_availability(mut self, availability: AvailabilitySignalSet) -> Self {
        self.availability = availability;
        self
    }

    pub fn with_toolsets(mut self, toolsets: ToolPlanningToolsetResolver) -> Self {
        self.toolsets = toolsets;
        self
    }

    pub fn build(self) -> ToolPlanningService {
        ToolPlanningService {
            contributors: self.contributors,
            availability: ToolAvailabilityEvaluator::new(self.availability),
            toolsets: self.toolsets,
        }
    }
}

fn conflicts(entries: &[ToolPlanEntry]) -> Vec<ToolConflictDiagnostic> {
    let mut buckets = BTreeMap::<String, Vec<String>>::new();
    for entry in entries {
        buckets
            .entry(entry.descriptor.visible_name.clone())
            .or_default()
            .push(entry.descriptor.stable_tool_id.clone());
    }
    buckets
        .into_iter()
        .filter_map(|(namespace, mut tool_ids)| {
            tool_ids.sort();
            tool_ids.dedup();
            (tool_ids.len() > 1).then_some(ToolConflictDiagnostic {
                namespace,
                tool_ids,
                reason: "visible_name_conflict".into(),
            })
        })
        .collect()
}

fn reason_code(reason: &ToolHiddenReason) -> String {
    serde_json::to_value(reason)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| "unknown".into())
}
