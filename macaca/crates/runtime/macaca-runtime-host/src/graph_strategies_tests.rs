//! Contract tests for graph query, format, and merge Strategies.

use macaca_proto::{GraphImportPlan, GraphQuery};

use super::graph_strategies::{
    default_graph_import_export_strategies, default_graph_query_strategies,
    BoundedGraphFormatStrategy, BoundedGraphQueryStrategy, DeterministicGraphMergeStrategy,
    GraphImportExportStrategy, GraphMergeRequest, GraphMergeStrategy, GraphQueryValidationStrategy,
};

#[test]
fn query_strategy_covers_declared_modes_without_provider_routing() {
    let strategies = default_graph_query_strategies();
    for mode in [
        "portable",
        "cypher_like",
        "sparql_like",
        "gremlin_like",
        "gsql_like",
        "provider_declared",
    ] {
        let strategy = strategies
            .iter()
            .find(|strategy| strategy.mode() == mode)
            .unwrap();
        assert!(
            strategy
                .validate(&GraphQuery {
                    query_ref: "query-ref".into(),
                    dialect: mode.into(),
                    max_rows: 100,
                    redaction_profile: "bounded".into(),
                })
                .accepted
        );
    }
}

#[test]
fn query_strategy_rejects_unbounded_or_wrong_mode_envelopes() {
    let strategy = BoundedGraphQueryStrategy::new("portable", 100);
    assert!(
        !strategy
            .validate(&GraphQuery {
                query_ref: "query-ref".into(),
                dialect: "sparql_like".into(),
                max_rows: 10,
                redaction_profile: "bounded".into(),
            })
            .accepted
    );
    assert!(
        !strategy
            .validate(&GraphQuery {
                query_ref: "query-ref".into(),
                dialect: "portable".into(),
                max_rows: 101,
                redaction_profile: "bounded".into(),
            })
            .accepted
    );
}

#[test]
fn import_export_strategies_bound_formats_and_batches() {
    let strategies = default_graph_import_export_strategies();
    for format in ["graph_bundle", "rdf_dataset", "json_ld_like", "csv_like"] {
        let strategy = strategies
            .iter()
            .find(|strategy| strategy.format() == format)
            .unwrap();
        assert!(
            strategy
                .validate_import(&GraphImportPlan {
                    import_ref: "import-ref".into(),
                    format: format.into(),
                    dry_run: true,
                    batch_size: 100,
                })
                .accepted
        );
        assert!(strategy.validate_export(format, 100).accepted);
    }
    let strategy = BoundedGraphFormatStrategy::new("graph_bundle", 100);
    assert!(!strategy.validate_export("graph_bundle", 101).accepted);
}

#[test]
fn merge_strategy_is_deterministic_and_reversible_by_reference() {
    let strategy = DeterministicGraphMergeStrategy;
    let decision = strategy.evaluate(&GraphMergeRequest {
        source_ref: "source".into(),
        target_ref: "target".into(),
        conflict_policy: "prefer_target".into(),
        reversible: true,
    });
    assert!(decision.accepted);
    assert_eq!(
        decision.alias_mapping_ref.as_deref(),
        Some("alias:source->target")
    );
    assert!(
        !strategy
            .evaluate(&GraphMergeRequest {
                source_ref: "same".into(),
                target_ref: "same".into(),
                conflict_policy: "prefer_target".into(),
                reversible: false,
            })
            .accepted
    );
}
