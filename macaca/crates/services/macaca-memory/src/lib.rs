//! aos-memory: three-layer memory system for Agent OS.
//!
//! The crate exposes two stable layers of API side by side:
//! - concrete managers/stores (`manager`, `isolated`, `store`, `vector`, ...)
//! - the memory fabric core (`core`) introduced as a provider-neutral boundary
//!
//! The fabric layer wraps builtin managers behind scope-aware traits so callers
//! use a generic memory contract while concrete storage backends continue to
//! evolve independently.

pub mod artifacts;
pub mod backend;
pub mod cache;
pub mod core;
pub mod embedding;
pub mod embedding_decorators;
pub mod embedding_registry;
pub mod facade;
pub mod file;
pub mod governance;
pub mod isolated;
pub mod manager;
pub mod providers;
pub mod query;
pub mod query_pipeline;
pub mod runtime;
pub mod service_adapter;
pub mod service_contract;
pub mod session;
pub mod snapshot;
pub mod store;
pub mod tombstone_index;
pub mod vector;
pub mod vector_backend;
pub mod vector_conformance;
pub mod vector_topology;

pub use backend::{
    ConfiguredMemoryManager, MemoryBackendConfig, MemoryBackendFactory, MemoryBackendProfile,
};
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
pub use embedding_decorators::{
    EmbeddingMetrics, MetricsEmbeddingProvider, RetryEmbeddingProvider, TimeoutEmbeddingProvider,
};
pub use embedding_registry::{EmbeddingProviderFactory, EmbeddingProviderRegistry};
pub use facade::{ForgetMemory, RecallQuery, RecallResult, RememberText};
pub use file::FileMemory;
pub use governance::{
    compiled_digest_candidates, ArtifactContent, ArtifactKind, CandidateCaptureResult,
    CandidateDecision, CandidatePromotionResult, CandidateSource, CitationArtifact,
    ClaimConfidence, ClaimEvidence, ClaimFreshness, ClaimGroup, ContradictionStrategy,
    DefaultMemoryCandidateCapturePolicy, DeterministicContradictionStrategy,
    DisabledMemoryCompactionStrategy, FacadeDeletePropagator, GovernedMemoryFacade,
    InMemoryGovernanceJournal, KnowledgeClaim, KnowledgeCompileCapability, KnowledgeCompileRequest,
    KnowledgeCompileResult, KnowledgeCompiler, KnowledgeContextCandidate,
    KnowledgeContextSourceKind, MemoryArtifact, MemoryArtifactList, MemoryArtifactScope,
    MemoryAuditEvent, MemoryAuditEventKind, MemoryCandidate, MemoryCandidateCapture,
    MemoryCandidateCapturePolicy, MemoryCandidateStore, MemoryCompactionResult,
    MemoryCompactionStrategy, MemoryDeletePropagation, MemoryDeletePropagationStep,
    MemoryDeletePropagator, MemoryGovernanceJournal, MemoryPromotionDecision,
    MemoryPromotionPolicy, MemoryPromotionPolicyId, MemoryPromotionTarget,
    MemoryProviderMigrationPlan, MemoryProviderMigrationPort, MemoryProviderMigrationResult,
    MemoryProviderMigrationRuntime, MemoryProviderMigrationStatus, MemoryTombstone,
    ProjectDecisionLogArtifact, WikiDigestArtifact,
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
pub use query_pipeline::{
    DefaultMemoryQueryPipeline, MemoryQueryMode, MemoryQueryPipeline, MemoryQueryPipelineResult,
};
pub use runtime::{
    ComposedMemoryRuntime, FabricMemoryRuntime, MemoryRuntimeBuilder, MemoryRuntimeFacade,
    MemoryRuntimeStatus,
};
pub use service_adapter::memory_service_descriptor;
pub use service_contract::{
    topology_labels_for_scope, MemoryForgetCommand, MemoryGetCommand, MemoryGetResult,
    MemoryPolicyHints, MemoryPrefetchCommand, MemoryRecallCommand, MemoryRecallResult,
    MemoryRememberCommand, MemoryRememberResult, MemoryServiceSnapshot,
    MemoryServiceSnapshotCommand, MemoryStatusCommand, MemoryTopologyLabels, MEMORY_FORGET_COMMAND,
    MEMORY_GET_COMMAND, MEMORY_PREFETCH_COMMAND, MEMORY_RECALL_COMMAND, MEMORY_REMEMBER_COMMAND,
    MEMORY_SERVICE_ID, MEMORY_SNAPSHOT_COMMAND, MEMORY_STATUS_COMMAND,
};
pub use session::SessionMemory;
pub use snapshot::{MemorySnapshot, MemorySnapshotStore};
pub use store::{
    DynamicEmbeddingProvider, DynamicVectorStore, EmbeddingProvider, MemoryRetriever, MemoryStore,
    VectorSearchResult, VectorStore,
};
pub use tombstone_index::{
    EmptyTombstoneIndex, GovernanceFacadeTombstones, MergedTombstoneIndex, SharedTombstoneRegistry,
    TombstoneIndex,
};
pub use vector::{InMemoryVectorStore, MilvusStore};
pub use vector_backend::{
    MilvusCollectionStoreFactory, TopologyVectorMemoryBackend, VectorBackendStatus,
    VectorCollectionSchema, VectorCollectionStoreFactory, VectorMemoryBackend, VectorMemoryHit,
    VectorMemoryRecord,
};
pub use vector_conformance::{
    assert_private_collections_are_isolated, assert_shared_collection_is_explicit,
    assert_status_reports_topology, assert_vector_backend_conformance,
};
pub use vector_topology::{
    sanitize_identifier, DefaultVectorTopologyResolver, VectorMemoryTopology, VectorTopologyKey,
    VectorTopologyResolver,
};
