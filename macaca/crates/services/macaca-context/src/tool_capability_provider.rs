//! Compact Context provider for Tool Capability Plane planning snapshots.
//!
//! The provider follows the Adapter pattern: it adapts a service-owned
//! `ToolCatalogPlanResult` into a bounded context index without copying raw
//! tool documentation, schemas, provider payloads, secrets, or credentials into
//! model-visible context.  Detailed diagnostics remain available through
//! `service.tool` audit/query commands.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{MacacaResult, ToolCatalogPlanResult, ToolHiddenReason};

use crate::{
    estimate_text_tokens, ContextCacheClass, ContextCandidate, ContextCandidateKind,
    ContextComposeContext, ContextProvider, ContextProviderOutcome, ContextProviderStage,
    ContextScope, ContextTarget, TrustLevel,
};

/// Bounded index derived from a tool plan snapshot.
///
/// The index stores only aggregate counts, visible names, family names, and
/// stable reason codes.  This keeps default context useful for planning while
/// preserving the OS rule that raw provider output and unbounded schemas never
/// enter context implicitly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCapabilityIndex {
    visible_names: BTreeSet<String>,
    visible_families: BTreeSet<String>,
    hidden_count: usize,
    conflict_count: usize,
    reason_counts: BTreeMap<String, usize>,
}

impl ToolCapabilityIndex {
    /// Build a compact index from a service-owned plan snapshot.
    pub fn from_plan(plan: &ToolCatalogPlanResult) -> Self {
        let mut visible_names = BTreeSet::new();
        let mut visible_families = BTreeSet::new();
        let mut reason_counts = BTreeMap::<String, usize>::new();

        for entry in &plan.visible {
            visible_names.insert(entry.descriptor.visible_name.clone());
            visible_families.insert(entry.descriptor.family.as_str().to_string());
        }
        for entry in &plan.hidden {
            if let Some(reason) = &entry.hidden_reason {
                *reason_counts.entry(reason_code(reason)).or_default() += 1;
            }
        }

        Self {
            visible_names,
            visible_families,
            hidden_count: plan.hidden.len(),
            conflict_count: plan.conflicts.len(),
            reason_counts,
        }
    }

    fn is_empty(&self) -> bool {
        self.visible_names.is_empty() && self.hidden_count == 0 && self.conflict_count == 0
    }

    fn render(&self) -> String {
        let mut out = format!(
            "<tool_capability_index visible_count=\"{}\" hidden_count=\"{}\" conflict_count=\"{}\">\n",
            self.visible_names.len(),
            self.hidden_count,
            self.conflict_count
        );
        for family in &self.visible_families {
            out.push_str(&format!("  <family name=\"{}\" />\n", xml_escape(family)));
        }
        for tool in &self.visible_names {
            out.push_str(&format!("  <tool name=\"{}\" />\n", xml_escape(tool)));
        }
        for (reason, count) in &self.reason_counts {
            out.push_str(&format!(
                "  <hidden reason=\"{}\" count=\"{}\" />\n",
                xml_escape(reason),
                count
            ));
        }
        out.push_str("</tool_capability_index>");
        out
    }
}

#[derive(Clone)]
pub struct ToolCapabilityContextProvider {
    index: ToolCapabilityIndex,
}

impl ToolCapabilityContextProvider {
    #[must_use]
    pub fn new(index: ToolCapabilityIndex) -> Self {
        Self { index }
    }
}

#[async_trait]
impl ContextProvider for ToolCapabilityContextProvider {
    fn provider_id(&self) -> &'static str {
        "tool_capability_index"
    }

    fn stage(&self) -> ContextProviderStage {
        ContextProviderStage::CapabilityIndex
    }

    async fn contribute(
        &self,
        _ctx: &ContextComposeContext<'_>,
    ) -> MacacaResult<ContextProviderOutcome> {
        if self.index.is_empty() {
            return Ok(ContextProviderOutcome::default());
        }
        let content = self.index.render();
        let token_estimate = estimate_text_tokens(&content);
        Ok(ContextProviderOutcome {
            candidates: vec![ContextCandidate {
                source_id: "tool_capability_index".into(),
                kind: ContextCandidateKind::CapabilityIndex,
                scope: ContextScope::Application,
                priority: 60,
                trust: TrustLevel::Trusted,
                cache_class: ContextCacheClass::Stable,
                target: ContextTarget::SystemStable,
                content,
                token_estimate,
                diagnostics: Vec::new(),
                evidence_memory_ids: Vec::new(),
                digest_strength: None,
                source_report: None,
            }],
            diagnostics: Vec::new(),
            active_recall_report: None,
        })
    }
}

#[must_use]
pub fn tool_capability_provider_arc(index: ToolCapabilityIndex) -> Arc<dyn ContextProvider> {
    Arc::new(ToolCapabilityContextProvider::new(index))
}

fn reason_code(reason: &ToolHiddenReason) -> String {
    serde_json::to_value(reason)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| "unknown".into())
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
