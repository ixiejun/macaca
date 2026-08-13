use macaca_proto::{
    compose_installed_domain_pack_catalog, reference_domain_pack_definitions,
    KNOWLEDGE_CITATIONS_PACK_ID, KNOWLEDGE_CITATIONS_SERVICE_ID,
    KNOWLEDGE_DOCUMENT_PARSING_PACK_ID, KNOWLEDGE_DOCUMENT_PARSING_SERVICE_ID,
    KNOWLEDGE_GRAPH_PACK_ID, KNOWLEDGE_GRAPH_SERVICE_ID, KNOWLEDGE_RETRIEVAL_PACK_ID,
    KNOWLEDGE_RETRIEVAL_SERVICE_ID, KNOWLEDGE_SEARCH_PACK_ID, KNOWLEDGE_SEARCH_SERVICE_ID,
    KNOWLEDGE_SUMMARIZATION_PACK_ID, KNOWLEDGE_SUMMARIZATION_SERVICE_ID,
};

use super::*;

// These tests keep the knowledge discovery surface provider-neutral. The SDK
// reads catalog metadata and never constructs search, vector, parser, citation,
// graph, summarization, model, database, or cloud-provider clients.

#[tokio::test]
async fn catalog_client_discovers_knowledge_contract_metadata() {
    let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
    let client = CatalogBackedDomainPackClient::new(catalog);

    let cases = [
        (
            KNOWLEDGE_SEARCH_PACK_ID,
            KNOWLEDGE_SEARCH_SERVICE_ID,
            "search.search",
            "knowledge_search_provider_not_installed",
            "semantic-search",
        ),
        (
            KNOWLEDGE_RETRIEVAL_PACK_ID,
            KNOWLEDGE_RETRIEVAL_SERVICE_ID,
            "retrieval.retrieve",
            "knowledge_retrieval_provider_not_installed",
            "hybrid-retrieval",
        ),
        (
            KNOWLEDGE_DOCUMENT_PARSING_PACK_ID,
            KNOWLEDGE_DOCUMENT_PARSING_SERVICE_ID,
            "document_parsing.parse_document",
            "knowledge_document_parsing_provider_not_installed",
            "structured-parser",
        ),
        (
            KNOWLEDGE_CITATIONS_PACK_ID,
            KNOWLEDGE_CITATIONS_SERVICE_ID,
            "citations.create_citation",
            "knowledge_citations_provider_not_installed",
            "identifier-resolver",
        ),
        (
            KNOWLEDGE_GRAPH_PACK_ID,
            KNOWLEDGE_GRAPH_SERVICE_ID,
            "graph.query",
            "knowledge_graph_provider_not_installed",
            "multi-model-graph",
        ),
        (
            KNOWLEDGE_SUMMARIZATION_PACK_ID,
            KNOWLEDGE_SUMMARIZATION_SERVICE_ID,
            "summarization.summarize",
            "knowledge_summarization_provider_not_installed",
            "abstractive-summary",
        ),
    ];

    for (pack_id, service_id, command, unavailable_reason, provider_class) in cases {
        let inspect = client
            .inspect_pack(&DomainPackInspectCommand::new(pack_id).expect("valid knowledge id"))
            .await
            .unwrap();
        let pack = inspect.pack.expect("knowledge descriptor exists");

        assert!(!pack.is_callable());
        assert_eq!(
            pack.metadata.diagnostics.unavailable_reason,
            unavailable_reason
        );
        assert!(pack
            .metadata
            .service_command_schemas
            .get(service_id)
            .is_some_and(|commands| commands.contains(command)));
        assert!(pack
            .metadata
            .provider_descriptors
            .contains_key(provider_class));
        assert!(pack
            .metadata
            .sdk
            .docs_url
            .contains("developer-packs/knowledge"));
    }
}

#[tokio::test]
async fn summarization_sdk_discovery_serializes_only_descriptor_metadata() {
    let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
    let client = CatalogBackedDomainPackClient::new(catalog);
    let inspect = client
        .inspect_pack(
            &DomainPackInspectCommand::new(KNOWLEDGE_SUMMARIZATION_PACK_ID)
                .expect("summary pack id must be valid"),
        )
        .await
        .unwrap();

    // SDK discovery receives catalog descriptors, not service-call payloads.
    // These markers model values that must remain confined to runtime-host
    // request handling and therefore must never enter developer diagnostics.
    let diagnostic = serde_json::to_string(&inspect).unwrap();
    for marker in [
        "credential=summary-secret",
        "private-source-content",
        "raw-provider-response",
        "private-conversation-turn",
    ] {
        assert!(
            !diagnostic.contains(marker),
            "SDK diagnostic leaked {marker}"
        );
    }
    assert!(diagnostic.contains("knowledge_summarization_provider_not_installed"));
    assert!(diagnostic.contains("redaction_policy"));
}

#[tokio::test]
async fn search_sdk_discovery_serializes_only_descriptor_metadata() {
    let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
    let client = CatalogBackedDomainPackClient::new(catalog);
    let inspect = client
        .inspect_pack(
            &DomainPackInspectCommand::new(KNOWLEDGE_SEARCH_PACK_ID)
                .expect("search pack id must be valid"),
        )
        .await
        .unwrap();

    // Discovery serializes an immutable descriptor, never request or provider data.
    let diagnostic = serde_json::to_string(&inspect).unwrap();
    for marker in [
        "credential=search-secret",
        "raw-provider-response",
        "private-corpus-content",
        "raw-query-token",
        "unbounded-snippet",
    ] {
        assert!(
            !diagnostic.contains(marker),
            "SDK diagnostic leaked {marker}"
        );
    }
    assert!(diagnostic.contains("knowledge_search_provider_not_installed"));
    assert!(diagnostic.contains("redaction_policy"));
}

#[tokio::test]
async fn citations_sdk_discovery_serializes_only_descriptor_metadata() {
    let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
    let client = CatalogBackedDomainPackClient::new(catalog);
    let inspect = client
        .inspect_pack(
            &DomainPackInspectCommand::new(KNOWLEDGE_CITATIONS_PACK_ID)
                .expect("citation pack id must be valid"),
        )
        .await
        .unwrap();

    // Discovery serializes the static descriptor rather than source or resolver data.
    let diagnostic = serde_json::to_string(&inspect).unwrap();
    for marker in [
        "credential=citation-secret",
        "raw-provider-response",
        "raw-source-document",
        "private-quote",
        "raw-style-file",
        "private-corpus-content",
    ] {
        assert!(
            !diagnostic.contains(marker),
            "SDK diagnostic leaked {marker}"
        );
    }
    assert!(diagnostic.contains("knowledge_citations_provider_not_installed"));
    assert!(diagnostic.contains("redaction_policy"));
}

#[tokio::test]
async fn document_parsing_sdk_discovery_serializes_only_descriptor_metadata() {
    let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
    let client = CatalogBackedDomainPackClient::new(catalog);
    let inspect = client
        .inspect_pack(
            &DomainPackInspectCommand::new(KNOWLEDGE_DOCUMENT_PARSING_PACK_ID)
                .expect("document parsing pack id must be valid"),
        )
        .await
        .unwrap();

    // Discovery uses descriptors and must never retain parsing inputs or outputs.
    let diagnostic = serde_json::to_string(&inspect).unwrap();
    for marker in [
        "credential=parser-secret",
        "raw-provider-response",
        "raw-document-bytes",
        "raw-ocr-image",
        "raw-embedded-file",
        "private-signature",
        "private-corpus-content",
    ] {
        assert!(
            !diagnostic.contains(marker),
            "SDK diagnostic leaked {marker}"
        );
    }
    assert!(diagnostic.contains("knowledge_document_parsing_provider_not_installed"));
    assert!(diagnostic.contains("redaction_policy"));
}
