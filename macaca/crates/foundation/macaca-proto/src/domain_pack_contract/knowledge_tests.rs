use std::collections::{BTreeMap, BTreeSet};

use super::*;

// Knowledge pack tests validate descriptor and DTO contracts only. They do not
// open network connections, parse private documents, query databases, call LLMs,
// construct provider adapters, or load provider-native search/vector/graph APIs.

#[test]
fn knowledge_descriptors_are_discoverable_and_not_callable() {
    let cases = [
        (
            knowledge_search_pack_definition(),
            KNOWLEDGE_SEARCH_PACK_ID,
            KNOWLEDGE_SEARCH_SERVICE_ID,
            KNOWLEDGE_SEARCH_COMMANDS,
            "knowledge_search_provider_not_installed",
            "semantic-search",
            "search.search",
        ),
        (
            knowledge_retrieval_pack_definition(),
            KNOWLEDGE_RETRIEVAL_PACK_ID,
            KNOWLEDGE_RETRIEVAL_SERVICE_ID,
            KNOWLEDGE_RETRIEVAL_COMMANDS,
            "knowledge_retrieval_provider_not_installed",
            "hybrid-retrieval",
            "retrieval.retrieve",
        ),
        (
            knowledge_document_parsing_pack_definition(),
            KNOWLEDGE_DOCUMENT_PARSING_PACK_ID,
            KNOWLEDGE_DOCUMENT_PARSING_SERVICE_ID,
            KNOWLEDGE_DOCUMENT_PARSING_COMMANDS,
            "knowledge_document_parsing_provider_not_installed",
            "structured-parser",
            "document_parsing.parse_document",
        ),
        (
            knowledge_citations_pack_definition(),
            KNOWLEDGE_CITATIONS_PACK_ID,
            KNOWLEDGE_CITATIONS_SERVICE_ID,
            KNOWLEDGE_CITATIONS_COMMANDS,
            "knowledge_citations_provider_not_installed",
            "identifier-resolver",
            "citations.create_citation",
        ),
        (
            knowledge_graph_pack_definition(),
            KNOWLEDGE_GRAPH_PACK_ID,
            KNOWLEDGE_GRAPH_SERVICE_ID,
            KNOWLEDGE_GRAPH_COMMANDS,
            "knowledge_graph_provider_not_installed",
            "multi-model-graph",
            "graph.query",
        ),
        (
            knowledge_summarization_pack_definition(),
            KNOWLEDGE_SUMMARIZATION_PACK_ID,
            KNOWLEDGE_SUMMARIZATION_SERVICE_ID,
            KNOWLEDGE_SUMMARIZATION_COMMANDS,
            "knowledge_summarization_provider_not_installed",
            "abstractive-summary",
            "summarization.summarize",
        ),
    ];

    for (definition, pack_id, service_id, commands, unavailable_reason, provider_class, command) in
        cases
    {
        assert_eq!(definition.pack_id, pack_id);
        assert!(!definition.is_callable());
        assert!(DomainPackDefinitionSpec.validate(&definition).is_ok());
        assert_eq!(
            definition.metadata.parent_pack_id.as_deref(),
            Some("pack.knowledge.v1")
        );
        assert_eq!(
            definition.metadata.diagnostics.unavailable_reason,
            unavailable_reason
        );
        assert!(definition
            .metadata
            .sdk
            .docs_url
            .contains("developer-packs/knowledge"));
        assert!(definition
            .metadata
            .provider_descriptors
            .contains_key(provider_class));
        assert!(definition
            .metadata
            .service_command_schemas
            .get(service_id)
            .is_some_and(|schemas| schemas.contains(command)));

        let descriptor_commands = definition
            .metadata
            .service_command_schemas
            .get(service_id)
            .expect("knowledge descriptor exposes command schemas");
        for expected in commands {
            assert!(
                descriptor_commands.contains(*expected),
                "missing command {expected}"
            );
        }
    }
}

#[test]
fn industrial_catalog_uses_specialized_knowledge_descriptors() {
    let definitions = industrial_reference_domain_pack_definitions();
    let search = definitions
        .iter()
        .find(|definition| definition.pack_id == KNOWLEDGE_SEARCH_PACK_ID)
        .expect("industrial catalog includes knowledge search");
    let graph = definitions
        .iter()
        .find(|definition| definition.pack_id == KNOWLEDGE_GRAPH_PACK_ID)
        .expect("industrial catalog includes knowledge graph");

    assert_eq!(
        search.metadata.diagnostics.unavailable_reason,
        "knowledge_search_provider_not_installed"
    );
    assert!(search
        .metadata
        .service_command_schemas
        .get(KNOWLEDGE_SEARCH_SERVICE_ID)
        .is_some_and(|commands| commands.contains("search.search")));
    assert_eq!(
        graph
            .metadata
            .provider_descriptors
            .get("multi-model-graph")
            .and_then(|descriptor| descriptor.metadata.get("rdf"))
            .map(String::as_str),
        Some("true")
    );
}

#[test]
fn knowledge_command_dtos_are_serde_compatible() {
    let envelope = KnowledgeCommandEnvelope {
        subject_ref: "handle:subject".into(),
        parameters: BTreeMap::from([("mode".into(), "preview".into())]),
        cursor: None,
        page_size: Some(10),
        idempotency_key: Some("idem-knowledge".into()),
    };

    let values = [
        serde_json::to_value(SearchSearchCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(RetrievalRetrieveCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(DocumentParsingParseDocumentCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(CitationsCreateCitationCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(GraphQueryCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(SummarizationSummarizeCommand { request: envelope }).unwrap(),
    ];

    assert!(values.iter().all(|value| value.is_object()));
}

#[test]
fn knowledge_descriptor_hashes_are_stable_and_distinct() {
    let hash_groups = [
        knowledge_search_descriptor_hashes().into_hashes(),
        knowledge_retrieval_descriptor_hashes().into_hashes(),
        knowledge_document_parsing_descriptor_hashes().into_hashes(),
        knowledge_citations_descriptor_hashes().into_hashes(),
        knowledge_graph_descriptor_hashes().into_hashes(),
        knowledge_summarization_descriptor_hashes().into_hashes(),
    ];

    for hashes in hash_groups {
        let unique = hashes.into_iter().collect::<BTreeSet<_>>();
        assert_eq!(unique.len(), 5);
        assert!(unique.iter().all(|hash| !hash.is_empty()));
    }
}

#[test]
fn knowledge_validation_helpers_are_provider_neutral() {
    let query = SearchQuery {
        query_ref: "artifact:query".into(),
        ast_hash: "hash".into(),
        filters: vec![SearchFilter {
            field: "title".into(),
            operator: "contains".into(),
            value_ref: "artifact:value".into(),
        }],
        facets: Vec::new(),
        sort: Vec::new(),
        page_size: 25,
    };
    assert!(query.is_bounded(100, 4));

    let vector_a = RetrievalVectorSpace {
        vector_space_id: "a".into(),
        dimensions: 1536,
        metric: "cosine".into(),
        embedding_model_ref: "embedding:default".into(),
    };
    let vector_b = RetrievalVectorSpace {
        vector_space_id: "b".into(),
        ..vector_a.clone()
    };
    assert!(vector_a.is_compatible_with(&vector_b));

    let identifier = CitationIdentifier::normalize("DOI", " 10.0000/EXAMPLE ");
    assert_eq!(identifier.scheme, "doi");
    assert_eq!(identifier.normalized_value, "10.0000/example");

    let selector = CitationSelector {
        selector_kind: "text_position".into(),
        start_offset: 10,
        end_offset: 20,
        checksum: None,
    };
    assert!(selector.is_bounded(64));

    let graph_query = GraphQuery {
        query_ref: "artifact:query".into(),
        dialect: "portable".into(),
        max_rows: 10,
        redaction_profile: "rows_only".into(),
    };
    assert!(graph_query.is_bounded(100));

    let summary_request = SummaryRequest {
        request_id: "summary".into(),
        sources: vec![SummarySource {
            source_ref: "document:one".into(),
            source_kind: "document".into(),
            revision: "rev1".into(),
            sensitivity: "normal".into(),
        }],
        mode: "extractive".into(),
        target_tokens: 256,
        language: Some("en".into()),
    };
    assert!(summary_request.is_bounded(4, 1024));
}

trait DescriptorHashSet {
    fn into_hashes(self) -> [String; 5];
}

impl DescriptorHashSet for SearchDescriptorHashes {
    fn into_hashes(self) -> [String; 5] {
        [
            self.command_schema_hash,
            self.result_schema_hash,
            self.descriptor_hash,
            self.provider_capability_schema_hash,
            self.unavailable_schema_hash,
        ]
    }
}

impl DescriptorHashSet for RetrievalDescriptorHashes {
    fn into_hashes(self) -> [String; 5] {
        [
            self.command_schema_hash,
            self.result_schema_hash,
            self.descriptor_hash,
            self.provider_capability_schema_hash,
            self.unavailable_schema_hash,
        ]
    }
}

impl DescriptorHashSet for DocumentParsingDescriptorHashes {
    fn into_hashes(self) -> [String; 5] {
        [
            self.command_schema_hash,
            self.result_schema_hash,
            self.descriptor_hash,
            self.provider_capability_schema_hash,
            self.unavailable_schema_hash,
        ]
    }
}

impl DescriptorHashSet for CitationDescriptorHashes {
    fn into_hashes(self) -> [String; 5] {
        [
            self.command_schema_hash,
            self.result_schema_hash,
            self.descriptor_hash,
            self.provider_capability_schema_hash,
            self.unavailable_schema_hash,
        ]
    }
}

impl DescriptorHashSet for GraphDescriptorHashes {
    fn into_hashes(self) -> [String; 5] {
        [
            self.command_schema_hash,
            self.result_schema_hash,
            self.descriptor_hash,
            self.provider_capability_schema_hash,
            self.unavailable_schema_hash,
        ]
    }
}

impl DescriptorHashSet for SummarizationDescriptorHashes {
    fn into_hashes(self) -> [String; 5] {
        [
            self.command_schema_hash,
            self.result_schema_hash,
            self.descriptor_hash,
            self.provider_capability_schema_hash,
            self.unavailable_schema_hash,
        ]
    }
}
