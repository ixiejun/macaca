//! External context adapter safety boundary contracts.

use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::engine::{ContextAssembleInput, ContextAssembleResult};
use crate::report::{ContextDecisionReport, ContextDecisionSeverity};

/// Safety limits applied when an external context adapter participates in assembly.
///
/// External adapters are intentionally treated as less trusted than the builtin
/// engines. These limits bound execution time and payload size so a buggy or
/// hostile adapter cannot silently explode the request budget.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextAdapterSafetyPolicy {
    pub timeout_ms: u64,
    pub max_payload_bytes: usize,
    pub require_schema_validation: bool,
    pub require_budget_validation: bool,
    pub circuit_breaker_failures: u32,
}

impl Default for ContextAdapterSafetyPolicy {
    fn default() -> Self {
        Self {
            timeout_ms: 2_000,
            max_payload_bytes: 256 * 1024,
            require_schema_validation: true,
            require_budget_validation: true,
            circuit_breaker_failures: 3,
        }
    }
}

impl ContextAdapterSafetyPolicy {
    /// Convert the configured timeout into a runtime `Duration`.
    pub fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_ms)
    }
}

/// Fallback policy used when an external adapter contributes nothing or fails.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextFallbackPolicy {
    pub fallback_engine_id: String,
    pub empty_external_contribution: bool,
}

impl Default for ContextFallbackPolicy {
    fn default() -> Self {
        Self {
            fallback_engine_id: "legacy".into(),
            empty_external_contribution: true,
        }
    }
}

/// Minimal metadata identifying an external context adapter implementation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExternalContextAdapterInfo {
    pub id: String,
    pub version: String,
}

/// Contract for adapters that source context from an external runtime/process.
///
/// The trait mirrors the builtin `ContextEngine` boundary closely so upper
/// layers can switch between builtin and external assembly without changing the
/// request/response shape.
#[async_trait]
pub trait ExternalContextAdapter: Send + Sync {
    fn info(&self) -> ExternalContextAdapterInfo;

    async fn assemble(
        &self,
        input: ContextAssembleInput,
    ) -> macaca_proto::MacacaResult<ContextAssembleResult>;
}

/// Validate that an external adapter result respects the configured guardrails.
///
/// Today the checks focus on two invariants:
/// - the adapter must not exceed the caller's prompt budget
/// - the emitted report payload must stay below a bounded size
///
/// Validation errors are returned as `ContextDecisionReport` so callers can
/// surface them in the same diagnostics pipeline as builtin engine decisions.
pub fn validate_external_result(
    result: &ContextAssembleResult,
    input: &ContextAssembleInput,
    policy: &ContextAdapterSafetyPolicy,
) -> Result<(), ContextDecisionReport> {
    if policy.require_budget_validation
        && result.report.estimated_total_tokens > input.budget.input_budget()
    {
        return Err(ContextDecisionReport {
            code: "external_context_budget_exceeded".into(),
            severity: ContextDecisionSeverity::Error,
            message: "External context adapter exceeded the request input budget.".into(),
        });
    }

    let payload_bytes = serde_json::to_vec(&result.report)
        .map(|payload| payload.len())
        .unwrap_or(usize::MAX);
    if payload_bytes > policy.max_payload_bytes {
        return Err(ContextDecisionReport {
            code: "external_context_payload_too_large".into(),
            severity: ContextDecisionSeverity::Error,
            message: "External context adapter report payload exceeded maximum size.".into(),
        });
    }

    Ok(())
}

/// Lightweight conformance assertions for engines/adapters.
///
/// These helpers document invariants the rest of the stack assumes from any
/// engine-like component without forcing a heavyweight certification layer.
pub struct ContextEngineConformance;

impl ContextEngineConformance {
    /// Assert that the result preserves the minimum report fields required by upper layers.
    pub fn assert_preserves_required_report_fields(result: &ContextAssembleResult) {
        assert!(!result.report.engine_id.trim().is_empty());
        assert!(!result.report.request_id.trim().is_empty());
        assert!(result.report.token_budget > 0);
    }

    /// Compile-time reminder that upper layers should depend on abstractions, not concrete types.
    pub fn assert_upper_layers_need_no_concrete_type<T: ?Sized + Send + Sync>(_engine: &T) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{ContextAssembleInput, ContextEngine};
    use macaca_proto::{LlmMessage, LlmOptions};

    #[test]
    fn safety_policy_defaults_are_bounded() {
        let policy = ContextAdapterSafetyPolicy::default();
        assert!(policy.timeout().as_millis() > 0);
        assert!(policy.max_payload_bytes > 0);
        assert!(policy.require_schema_validation);
    }

    #[tokio::test]
    async fn conformance_checks_legacy_result_shape() {
        let input = ContextAssembleInput::legacy(
            "agent",
            "model",
            vec![LlmMessage::user("hello")],
            LlmOptions::default(),
        );
        let result = crate::LegacyContextEngine.assemble(input).await.unwrap();
        ContextEngineConformance::assert_preserves_required_report_fields(&result);
    }
}
