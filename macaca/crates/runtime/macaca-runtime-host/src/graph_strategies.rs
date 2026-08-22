//! Replaceable Strategies for knowledge-graph validation and mutation planning.
//!
//! These Strategies deliberately operate on opaque references and bounded
//! envelopes.  They validate the parts the OS can understand without parsing
//! provider-native query languages, importing graph bytes, or deciding an
//! application-specific ontology.  Concrete adapters can implement the same
//! traits at an approved runtime composition root.

use macaca_proto::{GraphImportPlan, GraphQuery};

/// A sanitized result shared by graph validation Strategies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphStrategyDecision {
    /// Whether the request may proceed to a provider adapter.
    pub accepted: bool,
    /// Stable, provider-neutral reason code for denied requests.
    pub reason_code: &'static str,
}

impl GraphStrategyDecision {
    fn accepted() -> Self {
        Self {
            accepted: true,
            reason_code: "accepted",
        }
    }

    fn denied(reason_code: &'static str) -> Self {
        Self {
            accepted: false,
            reason_code,
        }
    }
}

/// Strategy boundary for portable and declared graph query dialects.
pub trait GraphQueryValidationStrategy: Send + Sync {
    /// Return the provider-neutral mode handled by this Strategy.
    fn mode(&self) -> &str;

    /// Validate only bounded query metadata before provider execution.
    fn validate(&self, query: &GraphQuery) -> GraphStrategyDecision;
}

/// Generic envelope validator used by all declared graph query modes.
///
/// Dialect parsing remains an adapter concern; this Strategy only enforces
/// that the caller supplied an opaque query reference and a bounded row limit.
#[derive(Debug, Clone)]
pub struct BoundedGraphQueryStrategy {
    mode: String,
    max_rows: u32,
}

impl BoundedGraphQueryStrategy {
    /// Construct a Strategy for one provider-neutral query mode.
    pub fn new(mode: impl Into<String>, max_rows: u32) -> Self {
        Self {
            mode: mode.into(),
            max_rows: max_rows.max(1),
        }
    }
}

impl GraphQueryValidationStrategy for BoundedGraphQueryStrategy {
    fn mode(&self) -> &str {
        &self.mode
    }

    fn validate(&self, query: &GraphQuery) -> GraphStrategyDecision {
        if query.dialect != self.mode {
            return GraphStrategyDecision::denied("query_dialect_not_supported");
        }
        if query.is_bounded(self.max_rows) {
            GraphStrategyDecision::accepted()
        } else {
            GraphStrategyDecision::denied("query_envelope_unbounded")
        }
    }
}

/// Build the default portable mode set.  New modes can be registered without
/// changing provider routing or the service command surface.
pub fn default_graph_query_strategies() -> Vec<Box<dyn GraphQueryValidationStrategy>> {
    [
        "portable",
        "cypher_like",
        "sparql_like",
        "gremlin_like",
        "gsql_like",
        "provider_declared",
    ]
    .into_iter()
    .map(|mode| Box::new(BoundedGraphQueryStrategy::new(mode, 10_000)) as _)
    .collect()
}

/// Strategy boundary for bounded graph import/export formats.
pub trait GraphImportExportStrategy: Send + Sync {
    /// Return the provider-neutral format handled by this Strategy.
    fn format(&self) -> &str;

    /// Validate an import plan without reading source bytes.
    fn validate_import(&self, plan: &GraphImportPlan) -> GraphStrategyDecision;

    /// Validate a bounded export request identified by an opaque handle.
    fn validate_export(&self, format: &str, max_items: u32) -> GraphStrategyDecision;
}

/// Format Strategy that enforces opaque handles and bounded batches/pages.
#[derive(Debug, Clone)]
pub struct BoundedGraphFormatStrategy {
    format: String,
    max_batch_size: u32,
}

impl BoundedGraphFormatStrategy {
    /// Construct a Strategy for one provider-neutral import/export format.
    pub fn new(format: impl Into<String>, max_batch_size: u32) -> Self {
        Self {
            format: format.into(),
            max_batch_size: max_batch_size.max(1),
        }
    }
}

impl GraphImportExportStrategy for BoundedGraphFormatStrategy {
    fn format(&self) -> &str {
        &self.format
    }

    fn validate_import(&self, plan: &GraphImportPlan) -> GraphStrategyDecision {
        if plan.format != self.format {
            return GraphStrategyDecision::denied("import_format_not_supported");
        }
        if plan.import_ref.trim().is_empty() {
            return GraphStrategyDecision::denied("import_reference_missing");
        }
        if plan.batch_size == 0 || plan.batch_size > self.max_batch_size {
            return GraphStrategyDecision::denied("import_batch_unbounded");
        }
        GraphStrategyDecision::accepted()
    }

    fn validate_export(&self, format: &str, max_items: u32) -> GraphStrategyDecision {
        if format != self.format {
            return GraphStrategyDecision::denied("export_format_not_supported");
        }
        if max_items == 0 || max_items > self.max_batch_size {
            return GraphStrategyDecision::denied("export_limit_unbounded");
        }
        GraphStrategyDecision::accepted()
    }
}

/// Build the generic formats documented by the pack contract.
pub fn default_graph_import_export_strategies() -> Vec<Box<dyn GraphImportExportStrategy>> {
    ["graph_bundle", "rdf_dataset", "json_ld_like", "csv_like"]
        .into_iter()
        .map(|format| Box::new(BoundedGraphFormatStrategy::new(format, 10_000)) as _)
        .collect()
}

/// Provider-neutral request passed to merge/conflict Strategies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphMergeRequest {
    /// Opaque entity references; values never enter runtime observability.
    pub source_ref: String,
    pub target_ref: String,
    /// Conflict policy declared by the caller.
    pub conflict_policy: String,
    /// Whether the caller requested a reversible alias mapping.
    pub reversible: bool,
}

/// Sanitized merge decision returned before a provider mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphMergeDecision {
    pub accepted: bool,
    pub reason_code: &'static str,
    pub alias_mapping_ref: Option<String>,
}

/// Strategy boundary for deterministic entity merge and conflict handling.
pub trait GraphMergeStrategy: Send + Sync {
    /// Evaluate a merge request without reading entity values.
    fn evaluate(&self, request: &GraphMergeRequest) -> GraphMergeDecision;
}

/// Conservative default merge Strategy with reversible alias support.
#[derive(Debug, Clone, Copy, Default)]
pub struct DeterministicGraphMergeStrategy;

impl GraphMergeStrategy for DeterministicGraphMergeStrategy {
    fn evaluate(&self, request: &GraphMergeRequest) -> GraphMergeDecision {
        if request.source_ref.trim().is_empty() || request.target_ref.trim().is_empty() {
            return GraphMergeDecision {
                accepted: false,
                reason_code: "merge_reference_missing",
                alias_mapping_ref: None,
            };
        }
        if request.source_ref == request.target_ref {
            return GraphMergeDecision {
                accepted: false,
                reason_code: "merge_self_conflict",
                alias_mapping_ref: None,
            };
        }
        if request.conflict_policy.trim().is_empty() {
            return GraphMergeDecision {
                accepted: false,
                reason_code: "merge_conflict_policy_missing",
                alias_mapping_ref: None,
            };
        }
        GraphMergeDecision {
            accepted: true,
            reason_code: "accepted",
            alias_mapping_ref: request
                .reversible
                .then(|| format!("alias:{}->{}", request.source_ref, request.target_ref)),
        }
    }
}
