//! Governed knowledge digest integration for the composer pipeline.
//!
//! ## Patterns
//! - **Adapter** — [`KnowledgeDigestContextProvider`](`context_provider::KnowledgeDigestContextProvider`)
//!   translates neutral [`crate::memory::KnowledgeDigestItem`] rows into [`crate::composer::ContextCandidate`]
//!   values without leaking memory crate internals into generic OS code.
//! - **Strategy** — [`selection::apply_digest_vs_raw_selection`] swaps digest-vs-raw arbitration rules based on
//!   [`macaca_proto::config::KnowledgeDigestContextConfig`] thresholds (confidence, freshness, evidence depth).

pub mod context_provider;
pub mod selection;
pub mod tombstone_filter;

#[cfg(test)]
mod composition_tests;

pub use context_provider::{knowledge_digest_provider_arc, KnowledgeDigestContextProvider};
pub use selection::{
    apply_digest_vs_raw_selection, digest_candidate_id, ACTIVE_VECTOR_RECALL_SOURCE_ID,
};
pub use tombstone_filter::filter_digest_items_by_tombstones;
