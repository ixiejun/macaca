//! aos-memory: three-layer memory system for Agent OS.
//!
//! The crate now exposes two layers of API side by side:
//! - legacy managers/stores (`manager`, `isolated`, `store`, `vector`, ...)
//! - the new memory fabric core (`core`) introduced as an additive boundary
//!
//! The fabric layer does not replace the old implementation immediately.
//! Instead, it wraps builtin managers behind scope-aware traits so callers can
//! migrate to a generic memory contract without losing backward compatibility.

pub mod artifacts;
pub mod backend;
pub mod cache;
pub mod core;
pub mod embedding;
pub mod facade;
pub mod file;
pub mod governance;
pub mod isolated;
pub mod manager;
pub mod providers;
pub mod query;
pub mod session;
pub mod snapshot;
pub mod store;
pub mod vector;
pub mod vector_backend;
pub mod vector_topology;

pub use backend::{MemoryBackendConfig, MemoryBackendFactory};
pub use cache::{CachedEmbeddingProvider, EmbeddingCache};
pub use core::{
    ActiveRecallBudget, ActiveRecallCandidate, ActiveRecallCapability, ActiveRecallDecision,
    ActiveRecallRequest, ActiveRecallResult, BuiltinAgentPrivateMemory, BuiltinSessionSharedMemory,
    DefaultActiveRecallStrategy, DefaultMemoryRouter, MemoryArtifactCapability,
    MemoryCapabilitySet, MemoryDeleteRequest, MemoryFabricFacade, MemoryFacade,
    MemoryFlushCapability, MemoryGetRequest, MemoryGovernanceCapability, MemoryIdentity,
    MemoryKnowledgeCapability, MemoryLifecycleCapability, MemoryLifecycleEvent,
    MemoryLifecycleEventKind, MemoryPrefetchRequest, MemoryPromptCapability, MemoryProvider,
    MemoryProviderDescriptor, MemoryRoute, MemoryRouter, MemoryScope, MemorySearchCapability,
    MemorySearchRequest, MemoryStatusReport, MemoryStoreCapability, MemoryVisibility,
    MemoryWriteRequest,
};
pub use embedding::{DashScopeEmbedding, MockEmbedding};
pub use facade::{ForgetMemory, RecallQuery, RecallResult, RememberText};
pub use file::FileMemory;
pub use governance::{
    compiled_digest_candidates, ArtifactContent, ArtifactKind, CandidateCaptureResult,
    CandidateDecision, CandidatePromotionResult, CandidateSource, ClaimConfidence, ClaimEvidence,
    ClaimFreshness, ClaimGroup, FacadeDeletePropagator, GovernedMemoryFacade, KnowledgeClaim,
    KnowledgeCompileCapability, KnowledgeCompileRequest, KnowledgeCompileResult, KnowledgeCompiler,
    KnowledgeContextCandidate, KnowledgeContextSourceKind, MemoryArtifact, MemoryArtifactList,
    MemoryArtifactScope, MemoryAuditEvent, MemoryAuditEventKind, MemoryCandidate,
    MemoryCandidateCapture, MemoryCandidateStore, MemoryDeletePropagation,
    MemoryDeletePropagationStep, MemoryDeletePropagator, MemoryPromotionDecision,
    MemoryPromotionPolicy, MemoryPromotionPolicyId, MemoryPromotionTarget, MemoryTombstone,
};
pub use isolated::{IsolatedMemoryManager, TestIsolatedMemoryManager};
pub use manager::{MemoryManager, TestMemoryManager};
pub use providers::{
    BuiltinMemoryProviderFactory, MemoryProviderConfig, MemoryProviderEndpointConfig,
    MemoryProviderFactory, MemoryProviderFactoryResult, MemoryProviderProfileConfig,
    MemoryProviderProfilesConfig, MemoryProviderRegistry, MemoryProviderRegistryError,
    MemoryProviderRemoteEnvelope, MemoryProviderRemoteResult, MemoryProviderResilienceConfig,
    MemoryProviderRuntime, MemoryProviderRuntimeConfig, MemoryProviderRuntimeStatus,
    MemoryProviderSlotBinding, MemoryProviderSlotKind, MemoryProviderSlotOverride,
    MemoryProviderSlotScope, MemoryProviderToolConfig, MemoryProviderTransportKind,
};
pub use query::{SimilarityVectorQueryStrategy, VectorQuery, VectorQueryStrategy};
pub use session::SessionMemory;
pub use snapshot::{MemorySnapshot, MemorySnapshotStore};
pub use store::{EmbeddingProvider, MemoryRetriever, MemoryStore, VectorSearchResult, VectorStore};
pub use vector::{InMemoryVectorStore, MilvusStore};
pub use vector_backend::{
    MilvusCollectionStoreFactory, TopologyVectorMemoryBackend, VectorBackendStatus,
    VectorCollectionSchema, VectorCollectionStoreFactory, VectorMemoryBackend, VectorMemoryHit,
    VectorMemoryRecord,
};
pub use vector_topology::{
    sanitize_identifier, DefaultVectorTopologyResolver, VectorMemoryTopology, VectorTopologyKey,
    VectorTopologyResolver,
};
