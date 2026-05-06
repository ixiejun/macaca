//! Orchestrated **Chain-of-Responsibility** execution with timeouts (Decorator + Strategy).
//!
//! The composer already sorts providers, but this module adds:
//! - per-provider wall-clock isolation (`tokio::time::timeout`);
//! - anti-corruption validation + deny-prefix + redaction strategies;
//! - optional cumulative token guardrails before handing candidates to [`crate::composer::ContextPlanBuilder`].

use std::sync::Arc;
use std::time::{Duration, Instant};

use macaca_proto::config::ContextGovernanceRuntimeConfig;
use macaca_proto::{MacacaError, MacacaResult};
use tokio::time::timeout;

use crate::composer::{
    sort_providers, ContextCandidate, ContextComposeContext, ContextProvider,
    ContextProviderDiagnostics, ContextProviderOutcome,
};
use crate::governance::candidate_ops::{
    is_denied_by_prefix, redact_plain_substrings, validate_governed_candidate,
    with_rewritten_content,
};
use crate::governance::fingerprint::governance_config_fingerprint;
use crate::report::{
    ActiveRecallDiagnostics, ContextDecisionReport, ContextDecisionSeverity,
    ProviderInvocationSummary, ProviderRuntimeSummary,
};

/// Result of the governed provider collection pass executed inside [`crate::composer::ContextFacade`].
pub struct GovernedCollection {
    /// Candidates ready for composer budgeting / rendering.
    pub candidates: Vec<ContextCandidate>,
    /// Diagnostics emitted directly by providers (pre-governance pipeline notes).
    pub provider_diagnostics: Vec<ContextProviderDiagnostics>,
    /// Active recall telemetry bubbles unchanged — it is already bounded by recall policy.
    pub active_recall: Vec<ActiveRecallDiagnostics>,
    /// Additional governance decisions (timeouts, validation, token trims).
    pub governance_decisions: Vec<ContextDecisionReport>,
    /// Summarized audit row for [`crate::report::ContextReport::provider_runtime`].
    pub summary: ProviderRuntimeSummary,
}

/// Enforces an optional **hard cap** on the sum of declared `token_estimate` values.
///
/// The composer still performs fine-grained budgeting — this guard exists for catastrophic cases
/// where providers over-declare or duplicate content slips through. Lowest composer `priority`
/// rows are dropped first (stable `source_id` tie-break).
fn trim_candidates_by_total_tokens(
    mut candidates: Vec<ContextCandidate>,
    max_total_tokens: u32,
    decisions: &mut Vec<ContextDecisionReport>,
) -> Vec<ContextCandidate> {
    if max_total_tokens == 0 {
        return candidates;
    }
    let mut total: u32 = candidates.iter().map(|c| c.token_estimate).sum();
    if total <= max_total_tokens {
        return candidates;
    }
    candidates.sort_by(|a, b| {
        a.priority
            .cmp(&b.priority)
            .then_with(|| a.source_id.cmp(&b.source_id))
    });
    while total > max_total_tokens && !candidates.is_empty() {
        let dropped = candidates.remove(0);
        decisions.push(ContextDecisionReport {
            code: "governance_token_trim".into(),
            severity: ContextDecisionSeverity::Warning,
            message: format!(
                "dropped candidate {} (priority {}) to satisfy max_total_candidate_tokens={}",
                dropped.source_id, dropped.priority, max_total_tokens
            ),
        });
        total = total.saturating_sub(dropped.token_estimate);
    }
    candidates
}

/// Executes one provider under a wall-clock ceiling and classifies the coarse outcome for auditing.
async fn execute_one_provider(
    provider: Arc<dyn ContextProvider>,
    ctx: &ContextComposeContext<'_>,
    limit: Duration,
    fail_open: bool,
) -> Result<(ContextProviderOutcome, &'static str), String> {
    let fut = provider.contribute(ctx);
    match timeout(limit, fut).await {
        Ok(Ok(out)) => Ok((out, "ok")),
        Ok(Err(e)) => {
            let msg = e.to_string();
            if fail_open {
                Ok((
                    ContextProviderOutcome {
                        candidates: vec![],
                        diagnostics: vec![ContextProviderDiagnostics {
                            provider_id: provider.provider_id().to_string(),
                            message: format!("provider error (fail-open): {msg}"),
                        }],
                        active_recall_report: None,
                    },
                    "error",
                ))
            } else {
                Err(msg)
            }
        }
        Err(_elapsed) => {
            if fail_open {
                Ok((
                    ContextProviderOutcome {
                        candidates: vec![],
                        diagnostics: vec![ContextProviderDiagnostics {
                            provider_id: provider.provider_id().to_string(),
                            message: "provider timed out (fail-open)".into(),
                        }],
                        active_recall_report: None,
                    },
                    "timeout",
                ))
            } else {
                Err("provider timed out".into())
            }
        }
    }
}

/// Runs the sorted provider chain with governance strategies applied.
///
/// ### Failure model
/// - `fail_open_on_provider_error == true`: errors and timeouts degrade to empty candidates for that
///   provider plus diagnostics, and the loop continues.
/// - `false`: the first fatal error aborts the whole collection (rare; for regulated deployments).
pub async fn run_governed_provider_chain(
    providers: &[Arc<dyn ContextProvider>],
    ctx: &ContextComposeContext<'_>,
    cfg: &ContextGovernanceRuntimeConfig,
) -> MacacaResult<GovernedCollection> {
    let ordered = sort_providers(providers);
    let limit = Duration::from_millis(cfg.per_provider_timeout_ms.max(1));
    let fp = governance_config_fingerprint(cfg);

    let mut collected: Vec<ContextCandidate> = Vec::new();
    let mut provider_diagnostics: Vec<ContextProviderDiagnostics> = Vec::new();
    let mut active_recall: Vec<ActiveRecallDiagnostics> = Vec::new();
    let mut governance_decisions: Vec<ContextDecisionReport> = Vec::new();
    let mut invocations: Vec<ProviderInvocationSummary> = Vec::new();

    for provider in &ordered {
        let pid = provider.provider_id().to_string();
        let start = Instant::now();

        let (outcome, invoke_status) = match execute_one_provider(
            Arc::clone(provider),
            ctx,
            limit,
            cfg.fail_open_on_provider_error,
        )
        .await
        {
            Ok(pair) => pair,
            Err(e) => {
                return Err(MacacaError::Config(format!(
                    "governance closed pipeline on provider {pid}: {e}"
                )));
            }
        };

        let mut accepted = 0usize;
        let mut dropped = 0usize;

        if let Some(report) = outcome.active_recall_report {
            active_recall.push(report);
        }
        provider_diagnostics.extend(outcome.diagnostics);

        for mut c in outcome.candidates {
            if is_denied_by_prefix(&c.source_id, &cfg.deny_source_id_prefixes) {
                dropped += 1;
                governance_decisions.push(ContextDecisionReport {
                    code: "governance_deny_prefix".into(),
                    severity: ContextDecisionSeverity::Info,
                    message: format!("skipped {} due to deny_source_id_prefixes", c.source_id),
                });
                continue;
            }
            if !cfg.redact_substrings.is_empty() {
                let new_text = redact_plain_substrings(&c.content, &cfg.redact_substrings);
                if new_text != c.content {
                    c = with_rewritten_content(c, new_text);
                    governance_decisions.push(ContextDecisionReport {
                        code: "governance_redaction".into(),
                        severity: ContextDecisionSeverity::Info,
                        message: format!("redacted configured substrings for {}", c.source_id),
                    });
                }
            }
            match validate_governed_candidate(&c) {
                Ok(()) => {
                    collected.push(c);
                    accepted += 1;
                }
                Err(reason) => {
                    dropped += 1;
                    governance_decisions.push(ContextDecisionReport {
                        code: "governance_invalid_candidate".into(),
                        severity: ContextDecisionSeverity::Warning,
                        message: format!("rejected {}: {reason}", c.source_id),
                    });
                }
            }
        }

        let latency_ms = start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
        invocations.push(ProviderInvocationSummary {
            provider_id: pid.clone(),
            outcome: invoke_status.into(),
            latency_ms,
            candidates_accepted: accepted,
            candidates_dropped: dropped,
            implementation_version: provider.implementation_version().map(String::from),
        });
    }

    let collected = trim_candidates_by_total_tokens(
        collected,
        cfg.max_total_candidate_tokens,
        &mut governance_decisions,
    );

    let summary = ProviderRuntimeSummary {
        policy_fingerprint: fp,
        policy_label: cfg.policy_label.clone(),
        invocations,
    };

    Ok(GovernedCollection {
        candidates: collected,
        provider_diagnostics,
        active_recall,
        governance_decisions,
        summary,
    })
}

/// Legacy collection path: no timeouts; preserves pre-governance semantics for compatibility tests.
pub async fn run_ungoverned_provider_chain(
    providers: &[Arc<dyn ContextProvider>],
    ctx: &ContextComposeContext<'_>,
) -> MacacaResult<(
    Vec<ContextCandidate>,
    Vec<ContextProviderDiagnostics>,
    Vec<ActiveRecallDiagnostics>,
)> {
    let mut collected = Vec::new();
    let mut pipeline_notes = Vec::new();
    let mut active_recall_telemetry = Vec::new();
    for provider in sort_providers(providers) {
        let outcome = provider.contribute(ctx).await?;
        if let Some(report) = outcome.active_recall_report {
            active_recall_telemetry.push(report);
        }
        pipeline_notes.extend(outcome.diagnostics);
        collected.extend(outcome.candidates);
    }
    Ok((collected, pipeline_notes, active_recall_telemetry))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use async_trait::async_trait;
    use macaca_proto::config::ContextGovernanceRuntimeConfig;
    use macaca_proto::{LlmMessage, MacacaError, MacacaResult};

    use crate::budget::ContextBudget;
    use crate::composer::{
        ContextCacheClass, ContextCandidate, ContextCandidateKind, ContextComposeContext,
        ContextProvider, ContextProviderOutcome, ContextProviderStage, ContextScope, ContextTarget,
    };
    use crate::engine::ContextAssembleInput;
    use crate::prompt::TrustLevel;

    fn minimal_input() -> ContextAssembleInput {
        ContextAssembleInput {
            app_id: None,
            session_id: None,
            agent_name: "test-agent".into(),
            model: "test-model".into(),
            base_messages: vec![LlmMessage::system("sys")],
            options: macaca_proto::LlmOptions::default(),
            budget: ContextBudget::default(),
        }
    }

    fn sample_candidate(source_id: &str, content: &str) -> ContextCandidate {
        ContextCandidate {
            source_id: source_id.into(),
            kind: ContextCandidateKind::Custom,
            scope: ContextScope::Request,
            priority: 10,
            trust: TrustLevel::Untrusted,
            cache_class: ContextCacheClass::Dynamic,
            target: ContextTarget::UserSide,
            content: content.into(),
            token_estimate: 4,
            diagnostics: Vec::new(),
            evidence_memory_ids: Vec::new(),
            digest_strength: None,
            source_report: None,
        }
    }

    /// Artifically slow provider — used to verify `tokio::time::timeout` classification.
    struct SlowProvider;

    #[async_trait]
    impl ContextProvider for SlowProvider {
        fn provider_id(&self) -> &str {
            "slow"
        }

        fn stage(&self) -> ContextProviderStage {
            ContextProviderStage::StableProfile
        }

        async fn contribute(
            &self,
            _ctx: &ContextComposeContext<'_>,
        ) -> MacacaResult<ContextProviderOutcome> {
            tokio::time::sleep(Duration::from_millis(400)).await;
            Ok(ContextProviderOutcome::default())
        }
    }

    /// Provider that always fails — exercises fail-open vs fail-closed branches.
    struct ErrorProvider;

    #[async_trait]
    impl ContextProvider for ErrorProvider {
        fn provider_id(&self) -> &str {
            "err"
        }

        fn stage(&self) -> ContextProviderStage {
            ContextProviderStage::StableProfile
        }

        async fn contribute(
            &self,
            _ctx: &ContextComposeContext<'_>,
        ) -> MacacaResult<ContextProviderOutcome> {
            Err(MacacaError::Config("unit-test failure".into()))
        }
    }

    /// Emits a single static candidate (policy layers decide acceptance).
    struct StaticProvider {
        out: ContextProviderOutcome,
    }

    #[async_trait]
    impl ContextProvider for StaticProvider {
        fn provider_id(&self) -> &str {
            "static"
        }

        fn stage(&self) -> ContextProviderStage {
            ContextProviderStage::StableProfile
        }

        async fn contribute(
            &self,
            _ctx: &ContextComposeContext<'_>,
        ) -> MacacaResult<ContextProviderOutcome> {
            Ok(self.out.clone())
        }
    }

    #[tokio::test]
    async fn timeout_marks_invocation_and_continues_when_fail_open() {
        let input = minimal_input();
        let ctx = ContextComposeContext {
            assemble_input: &input,
        };
        let mut cfg = ContextGovernanceRuntimeConfig::default();
        cfg.enabled = true;
        cfg.per_provider_timeout_ms = 30;
        cfg.fail_open_on_provider_error = true;

        let providers: Vec<Arc<dyn ContextProvider>> = vec![Arc::new(SlowProvider)];
        let bundle = super::run_governed_provider_chain(&providers, &ctx, &cfg)
            .await
            .expect("fail-open must not abort");

        assert_eq!(bundle.summary.invocations.len(), 1);
        assert_eq!(bundle.summary.invocations[0].outcome, "timeout");
        assert!(bundle.candidates.is_empty());
    }

    #[tokio::test]
    async fn provider_error_surfaces_as_diag_when_fail_open() {
        let input = minimal_input();
        let ctx = ContextComposeContext {
            assemble_input: &input,
        };
        let mut cfg = ContextGovernanceRuntimeConfig::default();
        cfg.enabled = true;
        cfg.per_provider_timeout_ms = 5_000;
        cfg.fail_open_on_provider_error = true;

        let providers: Vec<Arc<dyn ContextProvider>> = vec![Arc::new(ErrorProvider)];
        let bundle = super::run_governed_provider_chain(&providers, &ctx, &cfg)
            .await
            .expect("fail-open must not abort");

        assert_eq!(bundle.summary.invocations[0].outcome, "error");
        assert!(bundle
            .provider_diagnostics
            .iter()
            .any(|d| d.message.contains("fail-open")));
    }

    #[tokio::test]
    async fn fail_closed_aborts_on_first_error() {
        let input = minimal_input();
        let ctx = ContextComposeContext {
            assemble_input: &input,
        };
        let mut cfg = ContextGovernanceRuntimeConfig::default();
        cfg.fail_open_on_provider_error = false;
        cfg.per_provider_timeout_ms = 5_000;

        let providers: Vec<Arc<dyn ContextProvider>> = vec![Arc::new(ErrorProvider)];
        let err = super::run_governed_provider_chain(&providers, &ctx, &cfg)
            .await
            .err()
            .expect("expected closed pipeline error");
        assert!(
            err.to_string().contains("governance closed pipeline"),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn deny_prefix_drops_candidate_but_finishes_pipeline() {
        let input = minimal_input();
        let ctx = ContextComposeContext {
            assemble_input: &input,
        };
        let mut cfg = ContextGovernanceRuntimeConfig::default();
        cfg.per_provider_timeout_ms = 5_000;
        cfg.fail_open_on_provider_error = true;
        cfg.deny_source_id_prefixes = vec!["blocked:".into()];

        let out = ContextProviderOutcome {
            candidates: vec![sample_candidate("blocked:x", "payload")],
            ..Default::default()
        };
        let providers: Vec<Arc<dyn ContextProvider>> = vec![Arc::new(StaticProvider { out })];
        let bundle = super::run_governed_provider_chain(&providers, &ctx, &cfg)
            .await
            .unwrap();
        assert!(bundle.candidates.is_empty());
        assert!(bundle
            .governance_decisions
            .iter()
            .any(|d| d.code == "governance_deny_prefix"));
    }

    #[tokio::test]
    async fn substring_redaction_emits_governance_decision() {
        let input = minimal_input();
        let ctx = ContextComposeContext {
            assemble_input: &input,
        };
        let mut cfg = ContextGovernanceRuntimeConfig::default();
        cfg.per_provider_timeout_ms = 5_000;
        cfg.fail_open_on_provider_error = true;
        cfg.redact_substrings = vec!["SECRET".into()];

        let out = ContextProviderOutcome {
            candidates: vec![sample_candidate("ok", "before SECRET after")],
            ..Default::default()
        };
        let providers: Vec<Arc<dyn ContextProvider>> = vec![Arc::new(StaticProvider { out })];
        let bundle = super::run_governed_provider_chain(&providers, &ctx, &cfg)
            .await
            .unwrap();
        assert_eq!(bundle.candidates.len(), 1);
        assert!(
            bundle.candidates[0].content.contains("[REDACTED]"),
            "{}",
            bundle.candidates[0].content
        );
        assert!(bundle
            .governance_decisions
            .iter()
            .any(|d| d.code == "governance_redaction"));
    }

    #[tokio::test]
    async fn summary_includes_policy_fingerprint() {
        let input = minimal_input();
        let ctx = ContextComposeContext {
            assemble_input: &input,
        };
        let cfg = ContextGovernanceRuntimeConfig::default();
        let providers: Vec<Arc<dyn ContextProvider>> = Vec::new();
        let bundle = super::run_governed_provider_chain(&providers, &ctx, &cfg)
            .await
            .unwrap();
        assert_eq!(bundle.summary.invocations.len(), 0);
        assert_eq!(bundle.summary.policy_fingerprint.len(), 64);
    }
}
