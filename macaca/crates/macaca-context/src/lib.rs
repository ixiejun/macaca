//! Pluggable context engineering contracts for Macaca Agent OS.
//!
//! This crate owns the provider-neutral context boundary. Runtime, framework,
//! web, memory, and skill crates should depend on these abstractions instead
//! of coupling to a concrete context management implementation.

pub mod active_recall;
pub mod adapter;
pub mod budget;
pub mod capability;
pub mod catalog;
pub mod compaction;
pub mod composer;
pub mod engine;
pub mod estimate;
pub mod governance;
pub mod memory;
pub mod memory_active_recall_provider;
pub mod preflight;
pub mod profile;
pub mod prompt;
pub mod report;
pub mod source;

pub use active_recall::{
    active_recall_degraded, active_recall_diagnostics_from_prefetch, render_active_recall_fence,
    DefaultActiveRecallProvider,
};
pub use adapter::{
    validate_external_result, ContextAdapterSafetyPolicy, ContextEngineConformance,
    ContextFallbackPolicy, ExternalContextAdapter, ExternalContextAdapterInfo,
};
pub use budget::ContextBudget;
pub use capability::{
    mcp_capability_provider_arc, mcp_tool_collisions, runtime_tool_capability_provider_arc,
    skill_capability_provider_arc, CapabilityCollisionRecord, DeclaredCapabilityDependency,
    McpCapabilityCatalog, McpContextProvider, McpServerCapabilitySummary, RuntimeToolCapabilityCatalog,
    RuntimeToolCapabilityProvider, SkillCapabilityCatalog, SkillCapabilityRecord, SkillContextProvider,
    SkillFilterDiagnostic,
};
pub use compaction::{
    CompactionDecision, CompactionHookInput, CompactionLifecycleHook, CompactionPolicy,
    CompactionSummaryEnvelope, CompactionTrigger, LineageKind, NoopCompactionLifecycleHook,
    SessionLineage, ThresholdCompactionPolicy, TranscriptSegment,
};
pub use catalog::{
    assemble_context_providers, list_builtin_family_descriptors, ProviderAssemblyEnvironment,
    ProviderHealthLedger, ProviderHealthSnapshot, VersionedContextProvider,
};
pub use composer::{
    merge_composer_into_messages, sort_providers, ContextCacheClass, ContextCandidate,
    ContextCandidateKind, ContextComposeContext, ContextComposer, ContextFacade,
    ContextFacadeAssemblyPolicy, ContextPlan,
    ContextPlanBuilder, ContextPlanDecision, ContextProvider, ContextProviderDiagnostics,
    ContextProviderOutcome, ContextProviderStage, ContextScope, ContextTarget,
    DefaultContextComposer,
};
pub use engine::{
    ContextAfterTurnInput, ContextAssembleInput, ContextAssembleResult, ContextEngine,
    ContextEngineInfo, ContextEngineRegistry, ContextEngineSelection, ContextManagerFacade,
    ContextOptionsPatch, ContextRuntimeFacade, LegacyContextEngine, PruningContextEngine,
    SummaryContextEngine, WindowedContextEngine,
};
pub use estimate::estimate_text_tokens;
pub use memory::{
    memory_source, wiki_digest_source, ActiveRecallBudget, ActiveRecallCapability,
    ActiveRecallPolicy, ActiveRecallReport, ConfidenceScore, ContextSourceProvenance,
    MemoryPrefetchResult, MemoryProviderHookInput, MemoryRecallItem, MemoryRecallQuery,
    MemorySourceProvider, PrivacyTier, RecallCandidate, RecallDecision, WikiDigestSourceProvider,
};
pub use memory_active_recall_provider::{
    memory_active_recall_provider_arc, MemoryActiveRecallContextProvider,
};
pub use preflight::{
    read_only_recall_tool_name, ContextPreflightRecallConfig, ContextPreflightRecallInput,
    ContextPreflightRecallOutput,
};
pub use profile::{
    canonical_root, default_policy_for, load_profile_file, load_profile_file_at_canonical_root,
    profile_provider_arc, AgentProfileFileKind, ProfileFileContextProvider, ProfileKindPolicy,
    ProfileLoadOutput, ProfileSkipReason,
};
pub use prompt::{
    CompiledPrompt, PromptComposer, PromptSection, PromptSectionBuilder, PromptStability,
    TrustLevel,
};
pub use governance::{
    apply_trust_policies_to_candidates, governance_config_fingerprint,
    run_governed_provider_chain, run_ungoverned_provider_chain, validate_opaque_external_payload,
    ContextProviderFactory, ContextProviderRegistry, ExternalCandidateLimits, GovernedCollection,
    OpaqueExternalPayload, ProviderFactoryInput, ProviderFamilyDescriptor,
};
pub use report::{
    ActiveRecallDiagnostics, ComposerPlanSummary, ComposerSkipRecord, ContextDecisionReport,
    ContextDecisionSeverity, ContextReport, ContextReportBuilder, ContextSourceKind,
    ContextSourceReport, ProviderInvocationSummary, ProviderRuntimeSummary,
};
pub use source::{
    decision_for_snippet, BudgetPolicy, ContextRenderInput, ContextRenderMode, ContextRenderable,
    ContextSnippet, ContextSourceReference, DefaultBudgetPolicy, DefaultPruningPolicy,
    DefaultSourceRenderer, PruningDecision, PruningPolicy,
};
