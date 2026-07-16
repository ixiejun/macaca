//! Structural portfolio-contract validation.
//!
//! These Specification-style checks admit only bounded, reference-only evidence
//! into the generic service boundary.  Provider execution, consent, entitlement,
//! policy, metering, cancellation delivery, and audit emission remain runtime
//! responsibilities.

use super::finance_common::{FinanceCommandEnvelope, FinancePage};
use super::finance_portfolio::{
    AllocationBucket, PortfolioArtifactHandle, PortfolioAttribution, PortfolioFreshness,
    PortfolioMethodology, PortfolioPerformance, PortfolioReport, PortfolioReportJob,
    PortfolioReportRequest, PortfolioRiskSummary, PortfolioScenarioAnalysis, PortfolioScope,
    RebalanceIntent, RebalanceIntentPlan,
};

fn bounded_portfolio_reference(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && !value.chars().any(char::is_control)
        && !value.contains("://")
        && !value.contains("token=")
        && !value.contains("account=")
}

impl PortfolioScope {
    /// Validate the declared portfolio capability scope before runtime decorators
    /// resolve consent, entitlement, policy, resources, and a concrete provider.
    pub fn is_valid_declaration(&self) -> bool {
        bounded_portfolio_reference(&self.tenant_scope, 160)
            && bounded_portfolio_reference(&self.household_ref, 160)
            && bounded_portfolio_reference(&self.consent_ref, 160)
            && matches!(
                self.permission_scope.as_str(),
                "finance.portfolio.read"
                    | "finance.portfolio.analytics"
                    | "finance.portfolio.report"
                    | "finance.portfolio.intent.write"
            )
    }
}

impl FinanceCommandEnvelope {
    /// Bound a portfolio command envelope without interpreting analytics options.
    pub fn has_bounded_portfolio_request(&self, max_parameters: usize, max_page_size: u32) -> bool {
        bounded_portfolio_reference(&self.subject_ref, 160)
            && self.parameters.len() <= max_parameters
            && self.parameters.iter().all(|(key, value)| {
                bounded_portfolio_reference(key, 96) && bounded_portfolio_reference(value, 256)
            })
            && self
                .cursor
                .as_deref()
                .is_none_or(|cursor| bounded_portfolio_reference(cursor, 256))
            && self
                .page_size
                .is_none_or(|size| size > 0 && size <= max_page_size)
            && self
                .idempotency_key
                .as_deref()
                .is_none_or(|key| bounded_portfolio_reference(key, 160))
    }
}

impl<T> FinancePage<T> {
    /// Keep a page bounded and retain only a reference cursor for subsequent reads.
    pub fn has_bounded_portfolio_page(&self, max_items: usize) -> bool {
        self.items.len() <= max_items
            && self
                .next_cursor
                .as_deref()
                .is_none_or(|cursor| bounded_portfolio_reference(cursor, 256))
    }
}

impl PortfolioFreshness {
    /// Retain an explicit stale classification rather than silently treating old valuations as current.
    pub fn is_valid_evidence(&self) -> bool {
        self.source_timestamp_epoch_ms > 0
            && self
                .valuation_timestamp_epoch_ms
                .is_none_or(|value| value >= self.source_timestamp_epoch_ms)
            && matches!(
                self.freshness_class.as_str(),
                "current" | "stale" | "unknown"
            )
    }
}

impl PortfolioAttribution {
    /// Require a source and license class; display evidence is optional only for unrestricted data.
    pub fn is_valid_evidence(&self) -> bool {
        bounded_portfolio_reference(&self.source_ref, 160)
            && matches!(
                self.license_class.as_str(),
                "public" | "licensed" | "restricted"
            )
            && self
                .required_display_ref
                .as_deref()
                .is_none_or(|value| bounded_portfolio_reference(value, 160))
            && (self.license_class == "public" || self.required_display_ref.is_some())
    }
}

impl PortfolioMethodology {
    /// Preserve reproducible analytics methodology as a bounded reference and assumption hash.
    pub fn is_valid_evidence(&self) -> bool {
        bounded_portfolio_reference(&self.methodology_ref, 160)
            && matches!(
                self.calculation_class.as_str(),
                "allocation" | "exposure" | "performance" | "risk" | "scenario"
            )
            && bounded_portfolio_reference(&self.assumption_hash, 256)
    }
}

impl AllocationBucket {
    /// Allocation is descriptive analytics only and must include no-advice evidence.
    pub fn has_no_advice_metadata(&self) -> bool {
        bounded_portfolio_reference(&self.bucket_ref, 160)
            && bounded_portfolio_reference(&self.classification_ref, 160)
            && (-10_000..=10_000).contains(&self.weight_basis_points)
            && bounded_portfolio_reference(&self.no_advice_disclaimer_ref, 160)
    }
}

impl PortfolioPerformance {
    /// Performance output is admissible only with a reproducible method and no-advice label.
    pub fn has_no_advice_metadata(&self, max_returns: usize) -> bool {
        bounded_portfolio_reference(&self.performance_ref, 160)
            && bounded_portfolio_reference(&self.benchmark.benchmark_ref, 160)
            && self.returns.len() <= max_returns
            && self.returns.iter().all(|point| {
                bounded_portfolio_reference(&point.period_ref, 96)
                    && bounded_portfolio_reference(&point.cash_flow_treatment, 96)
            })
            && self.methodology.is_valid_evidence()
            && bounded_portfolio_reference(&self.no_advice_disclaimer_ref, 160)
    }
}

impl PortfolioRiskSummary {
    /// A risk projection cannot be used as advice without an explicit disclaimer reference.
    pub fn has_no_advice_metadata(&self) -> bool {
        bounded_portfolio_reference(&self.risk_ref, 160)
            && bounded_portfolio_reference(&self.confidence_class, 96)
            && bounded_portfolio_reference(&self.no_advice_disclaimer_ref, 160)
    }
}

impl PortfolioScenarioAnalysis {
    /// Scenario results remain reference-based analysis, not trade recommendations.
    pub fn has_no_advice_metadata(&self) -> bool {
        bounded_portfolio_reference(&self.scenario_ref, 160)
            && bounded_portfolio_reference(&self.assumption_hash, 256)
            && bounded_portfolio_reference(&self.confidence_class, 96)
            && bounded_portfolio_reference(&self.no_advice_disclaimer_ref, 160)
    }
}

impl RebalanceIntent {
    /// Preserve rebalance output as a non-executable intent rather than an order instruction.
    pub fn is_non_executable(&self) -> bool {
        bounded_portfolio_reference(&self.intent_ref, 160)
            && bounded_portfolio_reference(&self.target_ref, 160)
            && self.tolerance_basis_points >= 0
            && bounded_portfolio_reference(&self.non_execution_disclaimer_ref, 160)
    }
}

impl RebalanceIntentPlan {
    /// Require explicit no-advice and non-execution evidence for every rebalance intent.
    pub fn has_no_advice_metadata(&self, max_intents: usize) -> bool {
        self.is_bounded(max_intents)
            && matches!(
                self.approval_state.as_str(),
                "planned" | "approval_required" | "approved" | "rejected"
            )
            && bounded_portfolio_reference(&self.no_advice_disclaimer_ref, 160)
            && self.intents.iter().all(RebalanceIntent::is_non_executable)
            && self.constraints.iter().all(|constraint| {
                bounded_portfolio_reference(&constraint.constraint_ref, 160)
                    && bounded_portfolio_reference(&constraint.constraint_kind, 96)
                    && bounded_portfolio_reference(&constraint.limit_ref, 160)
            })
    }
}

impl PortfolioReportRequest {
    /// Validate a redacted report request before the runtime starts an asynchronous job.
    pub fn has_safe_preconditions(&self) -> bool {
        bounded_portfolio_reference(&self.request_ref, 160)
            && matches!(
                self.report_kind.as_str(),
                "summary" | "allocation" | "performance" | "risk" | "scenario"
            )
            && bounded_portfolio_reference(&self.period_range, 160)
            && self.currency.len() == 3
            && self.currency.bytes().all(|byte| byte.is_ascii_uppercase())
            && bounded_portfolio_reference(&self.redaction.policy_ref, 160)
            && bounded_portfolio_reference(&self.redaction.export_profile, 96)
            && self.redaction.redacted_fields.len() <= 64
    }
}

impl PortfolioArtifactHandle {
    /// Represent report output as an expiring artifact handle, never inline report content.
    pub fn is_safe_async_result(&self, now_epoch_ms: u64) -> bool {
        bounded_portfolio_reference(&self.artifact_id, 160)
            && matches!(
                self.export_format.as_str(),
                "json" | "csv" | "pdf" | "artifact"
            )
            && bounded_portfolio_reference(&self.checksum, 256)
            && self.expires_at_epoch_ms > now_epoch_ms
            && bounded_portfolio_reference(&self.retention_policy, 96)
            && bounded_portfolio_reference(&self.access_policy, 160)
    }
}

impl PortfolioReport {
    /// Validate a bounded report projection suitable for SDK, trace, and snapshot consumers.
    pub fn has_safe_projection(&self, now_epoch_ms: u64, max_sections: usize) -> bool {
        bounded_portfolio_reference(&self.report_ref, 160)
            && !self.sections.is_empty()
            && self.sections.len() <= max_sections
            && self
                .sections
                .iter()
                .all(|section| bounded_portfolio_reference(section, 96))
            && self
                .artifact
                .as_ref()
                .is_none_or(|artifact| artifact.is_safe_async_result(now_epoch_ms))
            && self.freshness.is_valid_evidence()
    }
}

impl PortfolioReportJob {
    /// Validate asynchronous lifecycle, timeout, cancellation, and replay evidence.
    pub fn is_valid_lifecycle(&self) -> bool {
        bounded_portfolio_reference(&self.job_ref, 160)
            && bounded_portfolio_reference(&self.request_ref, 160)
            && matches!(
                self.state.as_str(),
                "queued" | "running" | "completed" | "cancelled" | "failed" | "timed_out"
            )
            && (1_000..=300_000).contains(&self.timeout_ms)
            && self
                .cancellation_ref
                .as_deref()
                .is_none_or(|value| bounded_portfolio_reference(value, 160))
            && self
                .report_ref
                .as_deref()
                .is_none_or(|value| bounded_portfolio_reference(value, 160))
            && (self.state != "completed" || self.report_ref.is_some())
            && bounded_portfolio_reference(&self.replay_pointer, 256)
    }
}
