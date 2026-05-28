//! Focused tests for `service.review`.
//!
//! Review tests assert structured findings and trace refs without encoding any
//! coding-product workflow in the service provider.

use macaca_kernel::SystemService;
use macaca_proto::workbench::review::*;
use macaca_proto::{
    BoundedSummary, ServiceCommand, ServiceCommandName, TraceContext, WorkbenchCommandResult,
    WorkbenchCommandStatus,
};

use crate::ReviewSystemServiceProvider;

fn command<T: serde::Serialize>(name: &str, payload: T) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        serde_json::to_value(payload).unwrap(),
        TraceContext::new(format!("trace-{name}")),
    )
}

fn decode(value: serde_json::Value) -> WorkbenchCommandResult<ReviewServiceResponse> {
    serde_json::from_value(value).unwrap()
}

#[tokio::test]
async fn review_result_returns_structured_findings_with_trace_refs() {
    let provider = ReviewSystemServiceProvider::mock();
    let started = decode(
        provider
            .call(command(
                START_COMMAND,
                ReviewStartRequest {
                    subject_ref: "change://generic".into(),
                    changed_paths: vec!["src/lib.rs".into()],
                    evidence_refs: vec!["audit://git/example".into()],
                    rationale: Some(BoundedSummary {
                        text: "risk: shared interface changed".into(),
                        truncated: false,
                        original_byte_len: None,
                        artifact_ref: None,
                    }),
                },
            ))
            .await
            .unwrap()
            .output,
    );
    let Some(ReviewServiceResponse::Started(run)) = started.output else {
        panic!("expected review start");
    };

    let result = decode(
        provider
            .call(command(
                RESULT_COMMAND,
                ReviewRefRequest {
                    review_ref: run.review_ref,
                },
            ))
            .await
            .unwrap()
            .output,
    );

    let Some(ReviewServiceResponse::Result(result)) = result.output else {
        panic!("expected review result");
    };
    assert!(matches!(result.run.status, ReviewRunStatus::Complete));
    assert_eq!(result.findings.len(), 1);
    assert!(result.artifact_ref.is_some());
    assert!(matches!(
        result.findings[0].severity,
        ReviewSeverity::Medium
    ));
    assert_eq!(result.findings[0].trace_refs, vec!["trace-review.result"]);
}

#[tokio::test]
async fn unavailable_review_provider_returns_structured_unavailable() {
    let provider = ReviewSystemServiceProvider::unavailable("review disabled");
    let result = decode(
        provider
            .call(command(
                FINDINGS_LIST_COMMAND,
                ReviewRefRequest {
                    review_ref: "review-missing".into(),
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
}
