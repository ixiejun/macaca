//! Context report value objects.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use macaca_proto::ApplicationId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::budget::ContextBudget;

/// Canonical source categories used in context accounting reports.
///
/// Engines classify every included source into one of these buckets so upper
/// layers can understand how prompt budget is spent without inspecting raw text.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContextSourceKind {
    SystemPrompt,
    DynamicPrompt,
    History,
    ToolSchema,
    ToolResult,
    Skill,
    Memory,
    WikiDigest,
    Trace,
    Workspace,
    FileRead,
    CommandOutput,
    SearchResult,
    CompactionSummary,
    External,
    Unknown,
}

/// Structured accounting record for one context source included in a request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextSourceReport {
    pub id: String,
    pub kind: ContextSourceKind,
    pub label: String,
    pub estimated_tokens: u32,
    pub byte_size: usize,
    pub included: bool,
    #[serde(default)]
    pub pruned_tokens: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trust_level: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance_provider_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance_source_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence_score: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub privacy_tier: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_only: Option<bool>,
}

impl ContextSourceReport {
    /// Create a report entry for an included source.
    pub fn included(
        id: impl Into<String>,
        kind: ContextSourceKind,
        label: impl Into<String>,
        estimated_tokens: u32,
        byte_size: usize,
    ) -> Self {
        Self {
            id: id.into(),
            kind,
            label: label.into(),
            estimated_tokens,
            byte_size,
            included: true,
            pruned_tokens: 0,
            render_mode: None,
            trust_level: None,
            source_ref: None,
            provenance_provider_id: None,
            provenance_source_id: None,
            confidence_score: None,
            privacy_tier: None,
            request_only: None,
        }
    }

    /// Attach rendering/pruning metadata to a source report.
    pub fn with_rendering(
        mut self,
        render_mode: impl Into<String>,
        trust_level: impl Into<String>,
        source_ref: Option<String>,
        pruned_tokens: u32,
    ) -> Self {
        self.render_mode = Some(render_mode.into());
        self.trust_level = Some(trust_level.into());
        self.source_ref = source_ref;
        self.pruned_tokens = pruned_tokens;
        self
    }

    /// Attach bounded recall metadata for dynamic/request-only sources.
    pub fn with_recall_metadata(
        mut self,
        provider_id: impl Into<String>,
        source_id: impl Into<String>,
        confidence_score: u8,
        privacy_tier: impl Into<String>,
        request_only: bool,
    ) -> Self {
        self.provenance_provider_id = Some(provider_id.into());
        self.provenance_source_id = Some(source_id.into());
        self.confidence_score = Some(confidence_score);
        self.privacy_tier = Some(privacy_tier.into());
        self.request_only = Some(request_only);
        self
    }
}

/// Bounded active recall diagnostics stored in context reports.
///
/// The report intentionally excludes full memory text. It keeps source ids,
/// labels, token/byte estimates, decisions, and latency so operators can debug
/// recall behavior without persisting sensitive memory contents by default.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActiveRecallDiagnostics {
    pub provider_id: String,
    pub total_candidates: usize,
    pub selected_candidates: usize,
    pub latency_ms: u64,
    pub source_breakdown: Vec<ContextSourceReport>,
    pub decisions: Vec<ContextDecisionReport>,
}

/// Bounded governed wiki/digest diagnostics stored in context reports.
///
/// Unlike `sources`, this structure is strictly diagnostic: it keeps the
/// selected governed digest claims, their provider provenance, and bounded
/// metadata without altering top-level token accounting that already reflects
/// the merged prompt assembly path.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KnowledgeDigestDiagnostics {
    pub provider_id: String,
    pub total_candidates: usize,
    pub selected_candidates: usize,
    pub source_breakdown: Vec<ContextSourceReport>,
}

/// Severity levels for context-engine decisions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContextDecisionSeverity {
    Info,
    Warning,
    Error,
}

/// One human/machine-readable decision emitted during context assembly.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextDecisionReport {
    pub code: String,
    pub severity: ContextDecisionSeverity,
    pub message: String,
}

impl ContextDecisionReport {
    /// Convenience constructor for informational decisions.
    pub fn info(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            severity: ContextDecisionSeverity::Info,
            message: message.into(),
        }
    }
}

/// Composer plan summary stored on `ContextReport` (inspectable without full prompt text).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComposerPlanSummary {
    pub plan_id: String,
    pub selected_source_ids: Vec<String>,
    pub skipped: Vec<ComposerSkipRecord>,
    /// SHA-256 hex over **stable** composer candidates (post selection, pre engine).
    #[serde(default)]
    pub stable_candidate_fingerprint: String,
    /// SHA-256 hex over **dynamic/unknown** composer candidates.
    #[serde(default)]
    pub dynamic_candidate_fingerprint: String,
}

/// Audit row for a dropped composer candidate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComposerSkipRecord {
    pub source_id: String,
    pub reason_code: String,
    pub message: String,
}

/// Top-level report emitted for every assembled prompt.
///
/// This object is the main observability artifact of the context-engine
/// runtime. It captures:
/// - which engine ran
/// - prompt/token budget accounting
/// - stable/full prompt hashes
/// - per-source breakdown
/// - warnings/errors/fallback decisions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextReport {
    pub request_id: String,
    pub app_id: Option<ApplicationId>,
    pub session_id: Option<String>,
    pub agent_name: String,
    /// Engine that actually assembled context (after composite/fallback policy).
    pub engine_id: String,
    /// Engine id requested by configuration before fallback.
    #[serde(default)]
    pub requested_engine_id: String,
    /// True when the primary engine failed and fallback engine produced this report.
    #[serde(default)]
    pub engine_fallback_applied: bool,
    /// Count of compaction-successor nodes under the session root lineage (diagnostic).
    #[serde(default)]
    pub lineage_compaction_count: u32,
    pub model: String,
    pub created_at: DateTime<Utc>,
    pub estimated_total_tokens: u32,
    pub token_budget: u32,
    pub stable_prompt_tokens: u32,
    pub dynamic_prompt_tokens: u32,
    pub history_tokens: u32,
    pub tool_schema_tokens: u32,
    pub skill_tokens: u32,
    pub memory_tokens: u32,
    pub trace_tokens: u32,
    pub pruned_tokens: u32,
    pub stable_prompt_hash: String,
    pub prompt_hash: String,
    pub sources: Vec<ContextSourceReport>,
    pub decisions: Vec<ContextDecisionReport>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub active_recall: Vec<ActiveRecallDiagnostics>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub knowledge_digest: Vec<KnowledgeDigestDiagnostics>,
    /// Context composer plan when the composer pipeline ran for this assembly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub composer: Option<ComposerPlanSummary>,
    /// Provider-runtime governance summary (timeouts, drops, policy fingerprint) — no raw prompt text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_runtime: Option<ProviderRuntimeSummary>,
    /// Compact tool capability planning summaries.
    ///
    /// These rows are safe to persist because they carry aggregate counts and
    /// stable reason codes only.  They intentionally exclude raw tool schemas,
    /// raw provider payloads, secrets, credentials, and model-visible prompt
    /// bodies.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_capabilities: Vec<ToolCapabilityReport>,
}

/// Aggregate report for the service-owned Tool Capability Plane context index.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCapabilityReport {
    pub provider_id: String,
    pub visible_count: usize,
    pub hidden_count: usize,
    pub skipped_count: usize,
    pub conflict_count: usize,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub reason_counts: BTreeMap<String, usize>,
}

/// Redacted observability surface for how [`crate::governance::pipeline`] executed providers.
///
/// This structure is safe to persist or show in admin UIs: it records timing, ids, counts,
/// and policy metadata — never verbatim candidate bodies or user secrets by default.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderRuntimeSummary {
    /// SHA-256 hex over a canonical JSON snapshot of the governance knobs that were active
    /// for this request (stable ordering inside the fingerprint helper).
    pub policy_fingerprint: String,
    /// Optional operator-supplied label mirrored from runtime configuration for auditing.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub policy_label: String,
    /// Per-provider invocation records in pipeline order.
    pub invocations: Vec<ProviderInvocationSummary>,
}

/// One provider pass through the governed pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderInvocationSummary {
    /// `ContextProvider::provider_id`.
    pub provider_id: String,
    /// `ok`, `timeout`, or `error`.
    pub outcome: String,
    pub latency_ms: u64,
    /// Candidates accepted after governance filters for this invocation.
    pub candidates_accepted: usize,
    /// Candidates dropped (deny-prefix, invalid, or token budget trim attributable to this row).
    pub candidates_dropped: usize,
    /// Optional semver attached by [`crate::catalog::VersionedContextProvider`] or custom providers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implementation_version: Option<String>,
}

/// Incremental builder that keeps token accounting logic in one place.
#[derive(Debug, Clone)]
pub struct ContextReportBuilder {
    report: ContextReport,
}

impl ContextReportBuilder {
    /// Create a new report builder with default counters and a fresh request id.
    pub fn new(engine_id: impl Into<String>) -> Self {
        Self {
            report: ContextReport {
                request_id: Uuid::new_v4().to_string(),
                app_id: None,
                session_id: None,
                agent_name: String::new(),
                engine_id: engine_id.into(),
                requested_engine_id: String::new(),
                engine_fallback_applied: false,
                lineage_compaction_count: 0,
                model: String::new(),
                created_at: Utc::now(),
                estimated_total_tokens: 0,
                token_budget: ContextBudget::default().input_budget(),
                stable_prompt_tokens: 0,
                dynamic_prompt_tokens: 0,
                history_tokens: 0,
                tool_schema_tokens: 0,
                skill_tokens: 0,
                memory_tokens: 0,
                trace_tokens: 0,
                pruned_tokens: 0,
                stable_prompt_hash: String::new(),
                prompt_hash: String::new(),
                sources: Vec::new(),
                decisions: Vec::new(),
                active_recall: Vec::new(),
                knowledge_digest: Vec::new(),
                composer: None,
                provider_runtime: None,
                tool_capabilities: Vec::new(),
            },
        }
    }

    /// Attach request identity fields.
    pub fn identity(
        mut self,
        app_id: Option<ApplicationId>,
        session_id: Option<String>,
        agent_name: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        self.report.app_id = app_id;
        self.report.session_id = session_id;
        self.report.agent_name = agent_name.into();
        self.report.model = model.into();
        self
    }

    /// Override the engine id recorded in the report.
    pub fn engine(mut self, engine_id: impl Into<String>) -> Self {
        self.report.engine_id = engine_id.into();
        self
    }

    /// Record requested-engine metadata and whether fallback was used.
    pub fn engine_selection(
        mut self,
        requested_engine_id: impl Into<String>,
        fallback_applied: bool,
    ) -> Self {
        self.report.requested_engine_id = requested_engine_id.into();
        self.report.engine_fallback_applied = fallback_applied;
        self
    }

    /// Record lineage compaction count diagnostics.
    pub fn lineage_compactions(mut self, count: u32) -> Self {
        self.report.lineage_compaction_count = count;
        self
    }

    /// Apply the token budget used for this assembly pass.
    pub fn budget(mut self, budget: ContextBudget) -> Self {
        self.report.token_budget = budget.input_budget();
        self
    }

    /// Store stable/full prompt hashes computed by the prompt composer.
    pub fn hashes(mut self, stable: impl Into<String>, full: impl Into<String>) -> Self {
        self.report.stable_prompt_hash = stable.into();
        self.report.prompt_hash = full.into();
        self
    }

    /// Add one source report and update aggregate counters.
    ///
    /// This is the central accounting function of the reporting model: every
    /// source classification contributes to both per-source detail and top-level
    /// token totals so UIs and diagnostics can inspect either view.
    pub fn source(mut self, source: ContextSourceReport) -> Self {
        match source.kind {
            ContextSourceKind::SystemPrompt => {
                self.report.stable_prompt_tokens += source.estimated_tokens;
            }
            ContextSourceKind::DynamicPrompt => {
                self.report.dynamic_prompt_tokens += source.estimated_tokens;
            }
            ContextSourceKind::History => {
                self.report.history_tokens += source.estimated_tokens;
            }
            ContextSourceKind::ToolSchema => {
                self.report.tool_schema_tokens += source.estimated_tokens;
            }
            ContextSourceKind::ToolResult => {
                self.report.trace_tokens += source.estimated_tokens;
            }
            ContextSourceKind::Skill => self.report.skill_tokens += source.estimated_tokens,
            ContextSourceKind::Memory | ContextSourceKind::WikiDigest => {
                self.report.memory_tokens += source.estimated_tokens;
            }
            ContextSourceKind::Trace
            | ContextSourceKind::FileRead
            | ContextSourceKind::CommandOutput
            | ContextSourceKind::SearchResult => {
                self.report.trace_tokens += source.estimated_tokens;
            }
            _ => {}
        }
        self.report.estimated_total_tokens += source.estimated_tokens;
        self.report.pruned_tokens += source.pruned_tokens;
        self.report.sources.push(source);
        self
    }

    /// Append one decision emitted during assembly.
    pub fn decision(mut self, decision: ContextDecisionReport) -> Self {
        self.report.decisions.push(decision);
        self
    }

    /// Append bounded active recall diagnostics to the report.
    pub fn active_recall(mut self, diagnostics: ActiveRecallDiagnostics) -> Self {
        for source in diagnostics.source_breakdown.iter().cloned() {
            self = self.source(source);
        }
        for decision in diagnostics.decisions.iter().cloned() {
            self = self.decision(decision);
        }
        self.report.active_recall.push(diagnostics);
        self
    }

    /// Attach provider runtime summary produced by the governance pipeline.
    pub fn provider_runtime(mut self, summary: ProviderRuntimeSummary) -> Self {
        self.report.provider_runtime = Some(summary);
        self
    }

    /// Append one compact tool capability planning summary.
    ///
    /// The builder accepts only counts and reason codes so callers cannot
    /// accidentally smuggle raw descriptor schemas or provider diagnostics into
    /// the report.  Detailed operator views belong to the Tool service audit
    /// surface, not the default context report.
    pub fn tool_capabilities(mut self, report: ToolCapabilityReport) -> Self {
        self.report.tool_capabilities.push(report);
        self
    }

    /// Finalize the report.
    pub fn build(self) -> ContextReport {
        self.report
    }
}
