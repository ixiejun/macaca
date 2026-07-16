//! Provider-neutral Strategy selection for knowledge summarization commands.
//!
//! Strategies identify an execution shape and bounded artifact references only.
//! Model calls, source retrieval, prompts, and generated summary content remain
//! behind replaceable provider adapters selected by a composition root.

use serde::Serialize;

/// Stable summary execution shapes exposed by the runtime-host adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SummarizationStrategyKind {
    Extractive,
    Abstractive,
    Hybrid,
    LongDocumentSynthesis,
    RollingSummary,
    ContextCompression,
    Refinement,
    Comparison,
    Evaluation,
    EvidenceInspection,
    ProviderInspection,
}

impl SummarizationStrategyKind {
    /// Return the strategy assigned to a descriptor-owned command.
    pub fn for_command(command: &str) -> Option<Self> {
        Some(match command {
            "summarization.plan" | "summarization.validate_request" => Self::LongDocumentSynthesis,
            "summarization.summarize" => Self::Extractive,
            "summarization.summarize_with_citations" => Self::Hybrid,
            "summarization.summarize_many" => Self::LongDocumentSynthesis,
            "summarization.summarize_conversation" => Self::RollingSummary,
            "summarization.compress_context" => Self::ContextCompression,
            "summarization.refine_summary" => Self::Refinement,
            "summarization.compare_summaries" => Self::Comparison,
            "summarization.evaluate_summary" => Self::Evaluation,
            "summarization.inspect_summary_evidence" => Self::EvidenceInspection,
            "summarization.inspect_provider" => Self::ProviderInspection,
            _ => return None,
        })
    }

    /// Select a summary-generation Strategy from a bounded provider-neutral mode.
    pub fn for_command_and_mode(command: &str, mode: Option<&str>) -> Option<Self> {
        if command == "summarization.summarize" {
            return Some(match mode {
                Some("abstractive") => Self::Abstractive,
                Some("hybrid") => Self::Hybrid,
                _ => Self::Extractive,
            });
        }
        Self::for_command(command)
    }

    /// Return a trace-safe label suitable for diagnostics and replay metadata.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Extractive => "extractive",
            Self::Abstractive => "abstractive",
            Self::Hybrid => "hybrid",
            Self::LongDocumentSynthesis => "long_document_synthesis",
            Self::RollingSummary => "rolling_summary",
            Self::ContextCompression => "context_compression",
            Self::Refinement => "refinement",
            Self::Comparison => "comparison",
            Self::Evaluation => "evaluation",
            Self::EvidenceInspection => "evidence_inspection",
            Self::ProviderInspection => "provider_inspection",
        }
    }
}

/// Build an opaque checkpoint reference when a strategy needs resumable work.
pub fn checkpoint_ref(strategy: SummarizationStrategyKind, trace_id: &str) -> Option<String> {
    matches!(
        strategy,
        SummarizationStrategyKind::LongDocumentSynthesis
            | SummarizationStrategyKind::RollingSummary
            | SummarizationStrategyKind::ContextCompression
    )
    .then(|| format!("summarization:checkpoint:{}:{trace_id}", strategy.label()))
}

/// Bounded MapReduce execution memento for long-document work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LongDocumentExecutionPlan {
    pub chunk_refs: Vec<String>,
    pub map_summary_refs: Vec<String>,
    pub reduce_summary_ref: String,
    pub overlap_policy: &'static str,
    pub partial_failure_policy: &'static str,
    pub checkpoint_ref: String,
}

impl LongDocumentExecutionPlan {
    /// Build deterministic opaque references for resumable MapReduce orchestration.
    pub fn for_trace(trace_id: &str, chunk_count: usize) -> Self {
        let count = chunk_count.clamp(1, 32);
        let chunk_refs = (0..count)
            .map(|index| format!("summarization:chunk:{trace_id}:{index}"))
            .collect::<Vec<_>>();
        let map_summary_refs = (0..count)
            .map(|index| format!("summarization:map:{trace_id}:{index}"))
            .collect::<Vec<_>>();
        Self {
            chunk_refs,
            map_summary_refs,
            reduce_summary_ref: format!("summarization:reduce:{trace_id}"),
            overlap_policy: "bounded_reference_overlap",
            partial_failure_policy: "retain_successful_maps_and_resume_failed_maps",
            checkpoint_ref: format!("summarization:checkpoint:long_document_synthesis:{trace_id}"),
        }
    }
}
