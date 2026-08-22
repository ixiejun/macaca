use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::knowledge_common::{
    define_knowledge_command_wrappers, knowledge_pack_definition, knowledge_stable_hash,
    KnowledgeCommandEnvelope, KnowledgeError, KnowledgePackDescriptor, KnowledgePage,
    KnowledgeProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const KNOWLEDGE_GRAPH_PACK_ID: &str = "pack.knowledge.graph.v1";
pub const KNOWLEDGE_GRAPH_SERVICE_ID: &str = "service.knowledge.graph";

/// Canonical command names described by `pack.knowledge.graph.v1`.
pub const KNOWLEDGE_GRAPH_COMMANDS: &[&str] = &[
    "graph.register_store",
    "graph.inspect_store",
    "graph.upsert_schema",
    "graph.validate_schema",
    "graph.upsert_node",
    "graph.upsert_edge",
    "graph.delete_graph_items",
    "graph.upsert_triple",
    "graph.delete_triples",
    "graph.query",
    "graph.validate_query",
    "graph.traverse",
    "graph.find_path",
    "graph.merge_entities",
    "graph.import_subgraph",
    "graph.export_subgraph",
    "graph.inspect_provenance",
    "graph.inspect_provider",
];

pub(crate) const GRAPH_PERMISSION_SCOPES: &[&str] = &[
    "graph.store.read",
    "graph.store.manage",
    "graph.schema.read",
    "graph.schema.write",
    "graph.node.read",
    "graph.node.write",
    "graph.edge.read",
    "graph.edge.write",
    "graph.rdf.read",
    "graph.rdf.write",
    "graph.query",
    "graph.traverse",
    "graph.path",
    "graph.merge",
    "graph.import",
    "graph.export",
    "graph.provenance.read",
    "graph.provider.inspect",
];

const PROPERTY_GRAPH_METADATA: &[(&str, &str)] = &[
    ("property_graph", "true"),
    ("rdf", "false"),
    ("traversal", "true"),
    ("import_export", "true"),
];
const RDF_GRAPH_METADATA: &[(&str, &str)] = &[
    ("property_graph", "false"),
    ("rdf", "true"),
    ("sparql_like", "true"),
    ("import_export", "true"),
];
const MULTI_MODEL_GRAPH_METADATA: &[(&str, &str)] = &[
    ("property_graph", "true"),
    ("rdf", "true"),
    ("traversal", "true"),
    ("merge", "true"),
];
const GRAPH_MOCK_METADATA: &[(&str, &str)] = &[
    ("property_graph", "true"),
    ("rdf", "true"),
    ("traversal", "true"),
    ("import_export", "true"),
];
const GRAPH_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("property_graph", "false"),
    ("rdf", "false"),
    ("traversal", "false"),
    ("import_export", "false"),
];

const GRAPH_PROVIDER_CLASSES: &[KnowledgeProviderClass<'_>] = &[
    KnowledgeProviderClass {
        provider_class: "property-graph",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: PROPERTY_GRAPH_METADATA,
    },
    KnowledgeProviderClass {
        provider_class: "rdf-graph",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: RDF_GRAPH_METADATA,
    },
    KnowledgeProviderClass {
        provider_class: "multi-model-graph",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: MULTI_MODEL_GRAPH_METADATA,
    },
    KnowledgeProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: GRAPH_MOCK_METADATA,
    },
    KnowledgeProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: GRAPH_UNAVAILABLE_METADATA,
    },
];

pub fn knowledge_graph_pack_definition() -> DomainPackDefinition {
    knowledge_pack_definition(KnowledgePackDescriptor {
        pack_id: KNOWLEDGE_GRAPH_PACK_ID,
        child_change_id: "openspec:add-pack-knowledge-graph",
        docs_slug: "graph",
        service_id: KNOWLEDGE_GRAPH_SERVICE_ID,
        commands: KNOWLEDGE_GRAPH_COMMANDS,
        permission_scopes: GRAPH_PERMISSION_SCOPES,
        provider_classes: GRAPH_PROVIDER_CLASSES,
        health_probe: "graph.inspect_provider",
        unavailable_reason: "knowledge_graph_provider_not_installed",
        replay_schema: "knowledge.graph.replay.v1",
        data_classification: "knowledge_graph_metadata",
        retention_policy: "graph_values_and_source_documents_by_reference_with_bounded_results",
        redaction_policy: "credentials_provider_payloads_private_values_queries_and_execution_plans_redacted",
        examples: &[
            "Declare `pack.knowledge.graph.v1` as optional until a graph provider is installed.",
            "Use node, edge, statement, and provenance handles instead of raw source documents.",
        ],
        migration_notes: &[
            "Graph becomes callable only after an approved graph service provider registers command schemas.",
            "Provider-native query dialects, execution plans, and graph database payloads must stay behind provider adapters.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphStore {
    pub store_id: String,
    pub model_support: BTreeSet<String>,
    pub namespace: String,
    pub provenance_policy: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphSchema {
    pub schema_id: String,
    pub version_hash: String,
    pub node_labels: BTreeSet<String>,
    pub edge_labels: BTreeSet<String>,
    pub constraints: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphNode {
    pub node_id: String,
    pub labels: BTreeSet<String>,
    pub properties: Vec<GraphProperty>,
    pub provenance: Option<GraphProvenance>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphEdge {
    pub edge_id: String,
    pub source_node_id: String,
    pub target_node_id: String,
    pub label: String,
    pub properties: Vec<GraphProperty>,
    pub provenance: Option<GraphProvenance>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RdfTerm {
    pub term_kind: String,
    pub value_ref: String,
    pub datatype: Option<String>,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RdfStatement {
    pub subject: RdfTerm,
    pub predicate: RdfTerm,
    pub object: RdfTerm,
    pub graph_ref: Option<String>,
    pub provenance: Option<GraphProvenance>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphProperty {
    pub name: String,
    pub value_ref: String,
    pub value_kind: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphQuery {
    pub query_ref: String,
    pub dialect: String,
    pub max_rows: u32,
    pub redaction_profile: String,
}

impl GraphQuery {
    /// Validate query envelope limits without parsing provider-native dialects.
    pub fn is_bounded(&self, max_rows: u32) -> bool {
        !self.query_ref.trim().is_empty() && self.max_rows > 0 && self.max_rows <= max_rows
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphQueryResult {
    pub result_id: String,
    pub rows_ref: String,
    pub row_count: u32,
    pub truncated: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphTraversal {
    pub start_node_id: String,
    pub edge_labels: BTreeSet<String>,
    pub max_depth: u32,
    pub max_fanout: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphPath {
    pub path_id: String,
    pub node_ids: Vec<String>,
    pub edge_ids: Vec<String>,
    pub cost_micros: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphImportPlan {
    pub import_ref: String,
    pub format: String,
    pub dry_run: bool,
    pub batch_size: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphExportPlan {
    pub export_ref: String,
    pub format: String,
    pub max_items: u32,
    pub redaction_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphProvenance {
    pub source_ref: String,
    pub confidence_micros: u32,
    pub valid_from_epoch_ms: Option<u64>,
    pub valid_to_epoch_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphProviderCapability {
    pub provider_class: String,
    pub graph_models: BTreeSet<String>,
    pub query_dialects: BTreeSet<String>,
    pub import_export_formats: BTreeSet<String>,
    pub max_depth: u32,
    pub state: DomainPackProviderCapabilityState,
}

define_knowledge_command_wrappers!(
    GraphRegisterStoreCommand,
    GraphInspectStoreCommand,
    GraphUpsertSchemaCommand,
    GraphValidateSchemaCommand,
    GraphUpsertNodeCommand,
    GraphUpsertEdgeCommand,
    GraphDeleteGraphItemsCommand,
    GraphUpsertTripleCommand,
    GraphDeleteTriplesCommand,
    GraphQueryCommand,
    GraphValidateQueryCommand,
    GraphTraverseCommand,
    GraphFindPathCommand,
    GraphMergeEntitiesCommand,
    GraphImportSubgraphCommand,
    GraphExportSubgraphCommand,
    GraphInspectProvenanceCommand,
    GraphInspectProviderCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphResultStatus {
    Success,
    Page,
    PartialResult,
    DryRun,
    ValidationIssue,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    Quota,
    Timeout,
    Cancellation,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphResultEnvelope<T> {
    pub status: GraphResultStatus,
    pub data: Option<T>,
    pub page: Option<KnowledgePage<T>>,
    pub error: Option<KnowledgeError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_schema_hash: String,
    pub schema_version_hash: String,
    pub unavailable_schema_hash: String,
}

pub fn knowledge_graph_descriptor_hashes() -> GraphDescriptorHashes {
    let schema = GraphSchema {
        schema_id: "schema".into(),
        version_hash: "v1".into(),
        node_labels: BTreeSet::from(["Entity".into()]),
        edge_labels: BTreeSet::from(["RELATED_TO".into()]),
        constraints: BTreeSet::from(["entity_id_unique".into()]),
    };
    GraphDescriptorHashes {
        command_schema_hash: graph_stable_hash(&KNOWLEDGE_GRAPH_COMMANDS),
        result_schema_hash: graph_stable_hash(&GraphResultStatus::Success),
        descriptor_hash: graph_stable_hash(&knowledge_graph_pack_definition()),
        provider_capability_schema_hash: graph_stable_hash(&GraphProviderCapability {
            provider_class: "mock".into(),
            graph_models: BTreeSet::from(["property_graph".into(), "rdf".into()]),
            query_dialects: BTreeSet::from(["portable".into(), "sparql_like".into()]),
            import_export_formats: BTreeSet::from(["graph_bundle".into(), "rdf_dataset".into()]),
            max_depth: 5,
            state: DomainPackProviderCapabilityState::Preview,
        }),
        schema_version_hash: graph_stable_hash(&schema),
        unavailable_schema_hash: graph_stable_hash(&KnowledgeError {
            code: "unavailable".into(),
            message: "knowledge graph provider is not installed".into(),
            retryable: false,
            trace_safe_detail: Some("knowledge_graph_provider_not_installed".into()),
        }),
    }
}

pub fn graph_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    knowledge_stable_hash(value)
}
