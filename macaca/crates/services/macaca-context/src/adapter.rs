//! External context adapter safety boundary contracts.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::engine::{
    ContextAssembleInput, ContextAssembleResult, ContextEngine, ContextEngineInfo,
    ContextEngineRegistry, ContextEngineSelection, ContextRuntimeFacade,
};
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
            fallback_engine_id: "passthrough".into(),
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

/// Context-engine wrapper that runs an external adapter behind explicit safety and fallback policy.
///
/// Transport details remain outside this crate: callers install any process/RPC/WASM bridge by
/// implementing [`ExternalContextAdapter`] and optionally overlaying this engine into a runtime
/// registry. The wrapper enforces bounded execution, validates adapter output, and degrades through
/// a builtin-or-overlay fallback registry instead of crashing the main loop.
pub struct ExternalAdapterContextEngine {
    adapter: Arc<dyn ExternalContextAdapter>,
    safety: ContextAdapterSafetyPolicy,
    fallback: ContextFallbackPolicy,
    fallback_registry: ContextEngineRegistry,
}

impl ExternalAdapterContextEngine {
    pub fn new(
        adapter: Arc<dyn ExternalContextAdapter>,
        safety: ContextAdapterSafetyPolicy,
        fallback: ContextFallbackPolicy,
    ) -> Self {
        Self {
            adapter,
            safety,
            fallback,
            fallback_registry: ContextEngineRegistry::with_builtins(),
        }
    }

    pub fn with_fallback_registry(mut self, registry: ContextEngineRegistry) -> Self {
        self.fallback_registry = registry;
        self
    }

    async fn fallback_with_decision(
        &self,
        input: ContextAssembleInput,
        decision: ContextDecisionReport,
    ) -> macaca_proto::MacacaResult<ContextAssembleResult> {
        let fallback = ContextRuntimeFacade::new(
            self.fallback_registry.clone(),
            ContextEngineSelection {
                engine_id: self.fallback.fallback_engine_id.clone(),
                fallback_engine_id: self.fallback.fallback_engine_id.clone(),
            },
        );
        let mut result = fallback.assemble(input).await?;
        result.report.decisions.push(decision);
        Ok(result)
    }
}

#[async_trait]
impl ContextEngine for ExternalAdapterContextEngine {
    fn info(&self) -> ContextEngineInfo {
        let adapter = self.adapter.info();
        ContextEngineInfo {
            id: adapter.id,
            name: "External Adapter Context Engine".into(),
            version: adapter.version,
        }
    }

    async fn assemble(
        &self,
        input: ContextAssembleInput,
    ) -> macaca_proto::MacacaResult<ContextAssembleResult> {
        let adapter_id = self.info().id;
        let timeout = self.safety.timeout();
        let call = tokio::time::timeout(timeout, self.adapter.assemble(input.clone())).await;
        let result = match call {
            Ok(Ok(result)) => result,
            Ok(Err(error)) => {
                return self
                    .fallback_with_decision(
                        input,
                        ContextDecisionReport {
                            code: "external_context_adapter_fallback".into(),
                            severity: ContextDecisionSeverity::Warning,
                            message: format!(
                                "External context adapter '{adapter_id}' failed; fallback '{}' was used: {error}",
                                self.fallback.fallback_engine_id
                            ),
                        },
                    )
                    .await;
            }
            Err(_) => {
                return self
                    .fallback_with_decision(
                        input,
                        ContextDecisionReport {
                            code: "external_context_adapter_timeout".into(),
                            severity: ContextDecisionSeverity::Warning,
                            message: format!(
                                "External context adapter '{adapter_id}' exceeded {}ms; fallback '{}' was used.",
                                self.safety.timeout_ms, self.fallback.fallback_engine_id
                            ),
                        },
                    )
                    .await;
            }
        };

        if let Err(decision) = validate_external_result(&result, &input, &self.safety) {
            return self.fallback_with_decision(input, decision).await;
        }

        Ok(result)
    }
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
    use std::sync::Arc;

    #[test]
    fn safety_policy_defaults_are_bounded() {
        let policy = ContextAdapterSafetyPolicy::default();
        assert!(policy.timeout().as_millis() > 0);
        assert!(policy.max_payload_bytes > 0);
        assert!(policy.require_schema_validation);
    }

    #[tokio::test]
    async fn conformance_checks_passthrough_result_shape() {
        let input = ContextAssembleInput::unscoped(
            "agent",
            "model",
            vec![LlmMessage::user("hello")],
            LlmOptions::default(),
        );
        let result = crate::PassthroughContextEngine
            .assemble(input)
            .await
            .unwrap();
        ContextEngineConformance::assert_preserves_required_report_fields(&result);
    }

    struct StaticExternalAdapter {
        result: Option<ContextAssembleResult>,
        sleep_ms: u64,
    }

    #[async_trait]
    impl ExternalContextAdapter for StaticExternalAdapter {
        fn info(&self) -> ExternalContextAdapterInfo {
            ExternalContextAdapterInfo {
                id: "external-test".into(),
                version: "1.0.0".into(),
            }
        }

        async fn assemble(
            &self,
            input: ContextAssembleInput,
        ) -> macaca_proto::MacacaResult<ContextAssembleResult> {
            if self.sleep_ms > 0 {
                tokio::time::sleep(Duration::from_millis(self.sleep_ms)).await;
            }
            Ok(self.result.clone().unwrap_or(ContextAssembleResult {
                messages: input.base_messages.clone(),
                options: input.options.clone(),
                options_patch: Default::default(),
                report: crate::ContextReportBuilder::new("external-test")
                    .identity(
                        input.app_id,
                        input.session_id.clone(),
                        input.agent_name.clone(),
                        input.model.clone(),
                    )
                    .budget(input.budget)
                    .engine("external-test")
                    .build(),
            }))
        }
    }

    #[tokio::test]
    async fn external_adapter_engine_degrades_to_fallback_on_timeout() {
        let engine = ExternalAdapterContextEngine::new(
            Arc::new(StaticExternalAdapter {
                result: None,
                sleep_ms: 20,
            }),
            ContextAdapterSafetyPolicy {
                timeout_ms: 1,
                ..Default::default()
            },
            ContextFallbackPolicy::default(),
        );
        let result = engine
            .assemble(ContextAssembleInput::unscoped(
                "agent",
                "model",
                vec![LlmMessage::user("hello")],
                LlmOptions::default(),
            ))
            .await
            .unwrap();
        assert_eq!(result.report.engine_id, "passthrough");
        assert!(result
            .report
            .decisions
            .iter()
            .any(|decision| decision.code == "external_context_adapter_timeout"));
    }

    #[tokio::test]
    async fn external_adapter_engine_degrades_to_fallback_on_validation_failure() {
        let input = ContextAssembleInput::unscoped(
            "agent",
            "model",
            vec![LlmMessage::user("hello")],
            LlmOptions::default(),
        );
        let mut oversized_report = crate::ContextReportBuilder::new("external-test")
            .identity(
                input.app_id,
                input.session_id.clone(),
                input.agent_name.clone(),
                input.model.clone(),
            )
            .budget(input.budget)
            .engine("external-test")
            .build();
        oversized_report.estimated_total_tokens = input.budget.input_budget() + 1;
        let oversized = ContextAssembleResult {
            messages: input.base_messages.clone(),
            options: input.options.clone(),
            options_patch: Default::default(),
            report: oversized_report,
        };
        let engine = ExternalAdapterContextEngine::new(
            Arc::new(StaticExternalAdapter {
                result: Some(oversized),
                sleep_ms: 0,
            }),
            ContextAdapterSafetyPolicy::default(),
            ContextFallbackPolicy::default(),
        );
        let result = engine.assemble(input).await.unwrap();
        assert_eq!(result.report.engine_id, "passthrough");
        assert!(result
            .report
            .decisions
            .iter()
            .any(|decision| decision.code == "external_context_budget_exceeded"));
    }
}
