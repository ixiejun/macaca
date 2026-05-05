//! Pluggable context engineering contracts for Macaca Agent OS.
//!
//! This crate owns the provider-neutral context boundary. Runtime, framework,
//! web, memory, and skill crates should depend on these abstractions instead
//! of coupling to a concrete context management implementation.

pub mod adapter;
pub mod budget;
pub mod compaction;
pub mod engine;
pub mod estimate;
pub mod memory;
pub mod preflight;
pub mod prompt;
pub mod report;
pub mod source;

pub use adapter::{
    validate_external_result, ContextAdapterSafetyPolicy, ContextEngineConformance,
    ContextFallbackPolicy, ExternalContextAdapter, ExternalContextAdapterInfo,
};
pub use budget::ContextBudget;
pub use compaction::{
    CompactionDecision, CompactionHookInput, CompactionLifecycleHook, CompactionPolicy,
    CompactionSummaryEnvelope, CompactionTrigger, LineageKind, NoopCompactionLifecycleHook,
    SessionLineage, ThresholdCompactionPolicy, TranscriptSegment,
};
pub use engine::{
    ContextAfterTurnInput, ContextAssembleInput, ContextAssembleResult, ContextEngine,
    ContextEngineInfo, ContextEngineRegistry, ContextEngineSelection, ContextManagerFacade,
    ContextOptionsPatch, ContextRuntimeFacade, LegacyContextEngine, PruningContextEngine,
    SummaryContextEngine, WindowedContextEngine,
};
pub use estimate::estimate_text_tokens;
pub use memory::{
    memory_source, wiki_digest_source, ConfidenceScore, ContextSourceProvenance,
    MemoryProviderHookInput, MemoryRecallItem, MemoryRecallQuery, MemorySourceProvider,
    PrivacyTier, WikiDigestSourceProvider,
};
pub use preflight::{
    read_only_recall_tool_name, ContextPreflightRecallConfig, ContextPreflightRecallInput,
    ContextPreflightRecallOutput,
};
pub use prompt::{
    CompiledPrompt, PromptComposer, PromptSection, PromptSectionBuilder, PromptStability,
    TrustLevel,
};
pub use report::{
    ContextDecisionReport, ContextDecisionSeverity, ContextReport, ContextReportBuilder,
    ContextSourceKind, ContextSourceReport,
};
pub use source::{
    decision_for_snippet, BudgetPolicy, ContextRenderInput, ContextRenderMode, ContextRenderable,
    ContextSnippet, ContextSourceReference, DefaultBudgetPolicy, DefaultPruningPolicy,
    DefaultSourceRenderer, PruningDecision, PruningPolicy,
};
