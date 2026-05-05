//! Pluggable context engineering contracts for Macaca Agent OS.
//!
//! This crate owns the provider-neutral context boundary. Runtime, framework,
//! web, memory, and skill crates should depend on these abstractions instead
//! of coupling to a concrete context management implementation.

pub mod budget;
pub mod engine;
pub mod estimate;
pub mod prompt;
pub mod report;

pub use budget::ContextBudget;
pub use engine::{
    ContextAfterTurnInput, ContextAssembleInput, ContextAssembleResult, ContextEngine,
    ContextEngineInfo, ContextEngineRegistry, ContextManagerFacade, ContextOptionsPatch,
    LegacyContextEngine,
};
pub use estimate::estimate_text_tokens;
pub use prompt::{
    CompiledPrompt, PromptComposer, PromptSection, PromptSectionBuilder, PromptStability,
    TrustLevel,
};
pub use report::{
    ContextDecisionReport, ContextDecisionSeverity, ContextReport, ContextReportBuilder,
    ContextSourceKind, ContextSourceReport,
};
