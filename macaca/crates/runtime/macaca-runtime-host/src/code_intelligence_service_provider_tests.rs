//! Focused tests for `service.code_intelligence`.
//!
//! These tests keep analyzer behavior provider-neutral: local search is only a
//! built-in adapter, while unavailable providers still return structured
//! results instead of panicking or leaking raw analyzer payloads.

use macaca_kernel::SystemService;
use macaca_proto::workbench::code_intelligence::*;
use macaca_proto::{
    ServiceCommand, ServiceCommandName, TraceContext, WorkbenchCommandResult,
    WorkbenchCommandStatus,
};

use crate::CodeIntelligenceSystemServiceProvider;

fn command<T: serde::Serialize>(name: &str, payload: T) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        serde_json::to_value(payload).unwrap(),
        TraceContext::new(format!("trace-{name}")),
    )
}

fn decode(value: serde_json::Value) -> WorkbenchCommandResult<CodeIntelligenceResponse> {
    serde_json::from_value(value).unwrap()
}

#[tokio::test]
async fn local_search_returns_bounded_snippets_and_health() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("lib.rs"), "fn target_symbol() {}\n").unwrap();
    let provider = CodeIntelligenceSystemServiceProvider::mock();

    let result = decode(
        provider
            .call(command(
                SEARCH_COMMAND,
                CodeSearchRequest {
                    workspace_root: dir.path().to_string_lossy().to_string(),
                    query: "target_symbol".into(),
                    include_globs: Vec::new(),
                    max_results: 5,
                    snippet_budget_bytes: 8,
                },
            ))
            .await
            .unwrap()
            .output,
    );

    let Some(CodeIntelligenceResponse::Search {
        results,
        provider_health,
        ..
    }) = result.output
    else {
        panic!("expected search response");
    };
    assert!(provider_health.healthy);
    assert_eq!(results.len(), 1);
    assert!(results[0].snippet.truncated);
}

#[tokio::test]
async fn unavailable_analyzer_returns_structured_unavailable() {
    let provider = CodeIntelligenceSystemServiceProvider::unavailable("index disabled");

    let result = decode(
        provider
            .call(command(
                DIAGNOSTICS_COMMAND,
                AnalyzerDiagnosticsRequest {
                    workspace_root: ".".into(),
                    analyzer_ref: None,
                    changed_paths: Vec::new(),
                    max_diagnostics: 5,
                },
            ))
            .await
            .unwrap()
            .output,
    );

    assert!(matches!(
        result.status,
        WorkbenchCommandStatus::Unavailable { .. }
    ));
    assert!(matches!(
        result.output,
        Some(CodeIntelligenceResponse::Diagnostics {
            provider_health,
            ..
        }) if !provider_health.healthy
    ));
}
