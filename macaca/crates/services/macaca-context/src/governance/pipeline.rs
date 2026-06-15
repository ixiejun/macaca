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

/// Ungoverned collection path: no timeouts; preserves local collection semantics for conformance tests.
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
mod tests;
