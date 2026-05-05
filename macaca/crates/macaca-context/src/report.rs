//! Context report value objects.

use chrono::{DateTime, Utc};
use macaca_proto::ApplicationId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::budget::ContextBudget;

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
}

impl ContextSourceReport {
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
        }
    }

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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContextDecisionSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextDecisionReport {
    pub code: String,
    pub severity: ContextDecisionSeverity,
    pub message: String,
}

impl ContextDecisionReport {
    pub fn info(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            severity: ContextDecisionSeverity::Info,
            message: message.into(),
        }
    }
}

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
}

#[derive(Debug, Clone)]
pub struct ContextReportBuilder {
    report: ContextReport,
}

impl ContextReportBuilder {
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
            },
        }
    }

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

    pub fn engine(mut self, engine_id: impl Into<String>) -> Self {
        self.report.engine_id = engine_id.into();
        self
    }

    pub fn engine_selection(
        mut self,
        requested_engine_id: impl Into<String>,
        fallback_applied: bool,
    ) -> Self {
        self.report.requested_engine_id = requested_engine_id.into();
        self.report.engine_fallback_applied = fallback_applied;
        self
    }

    pub fn lineage_compactions(mut self, count: u32) -> Self {
        self.report.lineage_compaction_count = count;
        self
    }

    pub fn budget(mut self, budget: ContextBudget) -> Self {
        self.report.token_budget = budget.input_budget();
        self
    }

    pub fn hashes(mut self, stable: impl Into<String>, full: impl Into<String>) -> Self {
        self.report.stable_prompt_hash = stable.into();
        self.report.prompt_hash = full.into();
        self
    }

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

    pub fn decision(mut self, decision: ContextDecisionReport) -> Self {
        self.report.decisions.push(decision);
        self
    }

    pub fn build(self) -> ContextReport {
        self.report
    }
}
