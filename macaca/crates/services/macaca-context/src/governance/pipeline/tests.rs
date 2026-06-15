//! Contract tests for `pipeline.rs` (extracted to satisfy OS file-size gate).

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
