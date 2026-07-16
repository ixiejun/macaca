use super::finance_common::{FinanceCommandEnvelope, FinancePage};
use super::finance_portfolio::{
    AllocationBucket, BenchmarkReference, PortfolioArtifactHandle, PortfolioAttribution,
    PortfolioFreshness, PortfolioMethodology, PortfolioPerformance, PortfolioRedactionPolicy,
    PortfolioReport, PortfolioReportJob, PortfolioReportRequest, PortfolioRiskSummary,
    PortfolioScenarioAnalysis, PortfolioScope, RebalanceIntent, RebalanceIntentPlan,
};

#[test]
fn portfolio_preflight_bounds_page_report_and_async_job_evidence() {
    let scope = PortfolioScope {
        tenant_scope: "tenant-ref".into(),
        household_ref: "household-ref".into(),
        consent_ref: "consent-ref".into(),
        permission_scope: "finance.portfolio.read".into(),
    };
    assert!(scope.is_valid_declaration());
    assert!(!PortfolioScope {
        permission_scope: "finance.portfolio.trade".into(),
        ..scope.clone()
    }
    .is_valid_declaration());
    let command = FinanceCommandEnvelope {
        subject_ref: "portfolio-subject".into(),
        cursor: Some("cursor-ref".into()),
        page_size: Some(50),
        ..Default::default()
    };
    assert!(command.has_bounded_portfolio_request(4, 100));
    assert!(FinancePage::<String> {
        items: vec!["item".into()],
        next_cursor: Some("next-ref".into()),
        truncated: false
    }
    .has_bounded_portfolio_page(2));

    let request = PortfolioReportRequest {
        request_ref: "report-request".into(),
        report_kind: "performance".into(),
        period_range: "period-ref".into(),
        currency: "USD".into(),
        redaction: PortfolioRedactionPolicy {
            policy_ref: "policy-ref".into(),
            export_profile: "masked".into(),
            ..Default::default()
        },
    };
    assert!(request.has_safe_preconditions());
    let artifact = PortfolioArtifactHandle {
        artifact_id: "artifact-ref".into(),
        export_format: "artifact".into(),
        checksum: "checksum".into(),
        expires_at_epoch_ms: 200,
        retention_policy: "short".into(),
        access_policy: "policy-ref".into(),
    };
    let report = PortfolioReport {
        report_ref: "report-ref".into(),
        sections: ["performance".into()].into_iter().collect(),
        artifact: Some(artifact),
        freshness: PortfolioFreshness {
            source_timestamp_epoch_ms: 10,
            valuation_timestamp_epoch_ms: Some(11),
            freshness_class: "current".into(),
        },
    };
    assert!(report.has_safe_projection(100, 4));
    assert!(PortfolioReportJob {
        job_ref: "job-ref".into(),
        request_ref: "report-request".into(),
        state: "completed".into(),
        timeout_ms: 30_000,
        cancellation_ref: Some("cancel-ref".into()),
        report_ref: Some("report-ref".into()),
        replay_pointer: "replay-ref".into()
    }
    .is_valid_lifecycle());
}

#[test]
fn portfolio_preflight_requires_methodology_attribution_and_bounded_completion() {
    assert!(PortfolioMethodology {
        methodology_ref: "methodology-ref".into(),
        calculation_class: "risk".into(),
        assumption_hash: "assumption-hash".into()
    }
    .is_valid_evidence());
    assert!(PortfolioAttribution {
        source_ref: "source-ref".into(),
        license_class: "licensed".into(),
        required_display_ref: Some("display-ref".into())
    }
    .is_valid_evidence());
    assert!(!PortfolioReportJob {
        job_ref: "job-ref".into(),
        request_ref: "request-ref".into(),
        state: "completed".into(),
        timeout_ms: 30_000,
        replay_pointer: "replay-ref".into(),
        ..Default::default()
    }
    .is_valid_lifecycle());
    assert!(!PortfolioReportRequest {
        currency: "usd".into(),
        ..Default::default()
    }
    .has_safe_preconditions());
}

#[test]
fn portfolio_preflight_requires_no_advice_and_non_execution_metadata() {
    assert!(AllocationBucket {
        bucket_ref: "bucket-ref".into(),
        classification_ref: "class-ref".into(),
        value_micros: 1,
        weight_basis_points: 5_000,
        no_advice_disclaimer_ref: "no-advice-ref".into()
    }
    .has_no_advice_metadata());
    let methodology = PortfolioMethodology {
        methodology_ref: "methodology-ref".into(),
        calculation_class: "performance".into(),
        assumption_hash: "assumption-hash".into(),
    };
    assert!(PortfolioPerformance {
        performance_ref: "performance-ref".into(),
        benchmark: BenchmarkReference {
            benchmark_ref: "benchmark-ref".into(),
            currency: "USD".into(),
            source_ref: "source-ref".into(),
        },
        returns: vec![],
        methodology,
        no_advice_disclaimer_ref: "no-advice-ref".into()
    }
    .has_no_advice_metadata(4));
    assert!(PortfolioRiskSummary {
        risk_ref: "risk-ref".into(),
        confidence_class: "estimated".into(),
        no_advice_disclaimer_ref: "no-advice-ref".into(),
        ..Default::default()
    }
    .has_no_advice_metadata());
    assert!(PortfolioScenarioAnalysis {
        scenario_ref: "scenario-ref".into(),
        assumption_hash: "assumption-hash".into(),
        confidence_class: "estimated".into(),
        no_advice_disclaimer_ref: "no-advice-ref".into(),
        ..Default::default()
    }
    .has_no_advice_metadata());
    assert!(RebalanceIntentPlan {
        plan_ref: "plan-ref".into(),
        approval_state: "planned".into(),
        no_advice_disclaimer_ref: "no-advice-ref".into(),
        intents: vec![RebalanceIntent {
            intent_ref: "intent-ref".into(),
            target_ref: "target-ref".into(),
            tolerance_basis_points: 100,
            non_execution_disclaimer_ref: "non-execution-ref".into(),
            ..Default::default()
        }],
        ..Default::default()
    }
    .has_no_advice_metadata(4));
}
