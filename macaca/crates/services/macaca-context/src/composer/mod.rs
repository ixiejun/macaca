//! Context Composer multi-source assembly (Chain of Responsibility + Builder + Composite).
//!
//! Relationship to the **context engine** (`engine` module):
//! - **Composer**: turns provider candidates into a plan plus optional synthesized user context.
//! - **Engine**: applies window/prune/summary policies to the resulting `Vec<LlmMessage>`.
//!
//! Call order is always **Composer → Engine**, coordinated by [`crate::composer::facade::ContextFacade`].

mod assembly_policy;
mod candidate;
mod candidate_fingerprint;
mod default_composer;
mod facade;
mod plan;
mod provider;

pub use assembly_policy::ContextFacadeAssemblyPolicy;
pub use candidate::{
    CapabilityCandidate, ContextCacheClass, ContextCandidate, ContextCandidateKind, ContextScope,
    ContextTarget, DigestStrengthSnapshot,
};
pub use default_composer::{ContextComposer, ContextPlanBuilder, DefaultContextComposer};
pub use facade::{merge_composer_into_messages, ContextFacade};
pub use plan::{ContextPlan, ContextPlanDecision};
pub use provider::{
    sort_providers, ContextComposeContext, ContextProvider, ContextProviderDiagnostics,
    ContextProviderOutcome, ContextProviderStage,
};
