//! Provider-runtime governance: registry/factory, pipelines, and policy transforms.
//!
//! Design themes (aligned with `docs/context-memory-integration-openclaw-hermes-research.md`):
//! - **Decorator**: governance layers wrap raw provider output before composer budgeting.
//! - **Strategy**: redaction, deny-prefix, and timeout policies are swappable via configuration.
//! - **Abstract factory + registry**: optional user-replaceable provider families keyed by neutral ids.
//! - **Anti-corruption**: all content passes [`crate::governance::candidate_ops::validate_governed_candidate`].

pub mod candidate_ops;
pub mod external_boundary;
pub mod fingerprint;
pub mod pipeline;
pub mod registry;
pub mod trust_policy;

pub use candidate_ops::{
    is_denied_by_prefix, redact_plain_substrings, validate_governed_candidate,
    with_rewritten_content,
};
pub use external_boundary::{
    validate_opaque_external_payload, ExternalCandidateLimits, OpaqueExternalPayload,
};
pub use fingerprint::governance_config_fingerprint;
pub use pipeline::{
    run_governed_provider_chain, run_ungoverned_provider_chain, GovernedCollection,
};
pub use registry::{
    ContextProviderFactory, ContextProviderRegistry, ProviderFactoryInput, ProviderFamilyDescriptor,
};
pub use trust_policy::apply_trust_policies_to_candidates;
