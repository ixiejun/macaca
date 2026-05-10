//! [`ContextFacade`] (Facade): single entry that orchestrates:
//! 1) Chain of Responsibility across [`crate::composer::ContextProvider`];
//! 2) Builder / Composite via [`crate::composer::DefaultContextComposer`];
//! 3) Delegation to [`crate::engine::ContextRuntimeFacade`] for the engine stage.

use std::collections::BTreeMap;
use std::sync::Arc;

use macaca_proto::{LlmMessage, LlmRole, MacacaResult};
use uuid::Uuid;

use crate::composer::assembly_policy::ContextFacadeAssemblyPolicy;
use crate::composer::candidate_fingerprint::fingerprint_selected_candidates;
use crate::composer::default_composer::{ContextComposer, DefaultContextComposer};
use crate::composer::plan::ContextPlan;
use crate::composer::provider::{ContextComposeContext, ContextProvider};
use crate::engine::{
    ContextAssembleInput, ContextAssembleResult, ContextEngineSelection, ContextRuntimeFacade,
};
use crate::governance::{run_governed_provider_chain, run_ungoverned_provider_chain};
use crate::knowledge_digest::apply_digest_vs_raw_selection;
use crate::prompt::CompiledPrompt;
use crate::report::{
    ComposerPlanSummary, ComposerSkipRecord, ContextDecisionReport, ContextSourceReport,
    KnowledgeDigestDiagnostics,
};

/// Merges composer output (`CompiledPrompt::text`) with the visible message history for this call.
///
/// ### Why insert a `user` row after the system prefix?
/// - Hermes/OpenClaw guidance: keep volatile recall on the **user** side (often fenced) to avoid
///   busting system-side prompt caches.
/// - Foundation packs the full composer payload into **one** synthetic user message; future
///   providers may split by `ContextTarget` while this stays intentionally minimal.
///
/// ### Canonical transcript boundary
/// This only mutates the `base_messages` snapshot passed into `ContextEngine` for the current
/// request. Runtimes/frameworks must explicitly choose whether to persist injected rows; default
/// call paths clone before merge and do not append composer rows to long-lived transcripts.
pub fn merge_composer_into_messages(
    mut base: Vec<LlmMessage>,
    compiled: &CompiledPrompt,
) -> Vec<LlmMessage> {
    let text = compiled.text.trim();
    if text.is_empty() {
        return base;
    }

    let inject = LlmMessage::user(format!("<!-- MACACA_CONTEXT_COMPOSER -->\n{text}"));

    // Insert after the leading run of system messages so instructions stay first.
    let mut pos = 0usize;
    while pos < base.len() && base[pos].role == LlmRole::System {
        pos += 1;
    }
    base.insert(pos, inject);
    base
}

fn summarize_plan(plan: &ContextPlan) -> ComposerPlanSummary {
    let (stable_candidate_fingerprint, dynamic_candidate_fingerprint) =
        fingerprint_selected_candidates(&plan.selected);
    ComposerPlanSummary {
        plan_id: plan.plan_id.clone(),
        selected_source_ids: plan.selected.iter().map(|c| c.source_id.clone()).collect(),
        skipped: plan
            .skipped
            .iter()
            .map(|s| ComposerSkipRecord {
                source_id: s.source_id.clone(),
                reason_code: s.reason_code.clone(),
                message: s.message.clone(),
            })
            .collect(),
        stable_candidate_fingerprint,
        dynamic_candidate_fingerprint,
    }
}

fn summarize_knowledge_digest_candidates(
    candidates: &[crate::composer::ContextCandidate],
    selected: &[crate::composer::ContextCandidate],
) -> Vec<KnowledgeDigestDiagnostics> {
    let mut total_by_provider: BTreeMap<String, usize> = BTreeMap::new();
    let mut selected_by_provider: BTreeMap<String, Vec<ContextSourceReport>> = BTreeMap::new();

    for candidate in candidates.iter().filter(|candidate| {
        candidate.kind == crate::composer::ContextCandidateKind::KnowledgeDigest
    }) {
        let provider_id = candidate
            .source_report
            .as_ref()
            .and_then(|report| report.provenance_provider_id.clone())
            .unwrap_or_else(|| "knowledge_digest".into());
        *total_by_provider.entry(provider_id).or_default() += 1;
    }

    for candidate in selected.iter().filter(|candidate| {
        candidate.kind == crate::composer::ContextCandidateKind::KnowledgeDigest
    }) {
        let Some(report) = candidate.source_report.clone() else {
            continue;
        };
        let provider_id = report
            .provenance_provider_id
            .clone()
            .unwrap_or_else(|| "knowledge_digest".into());
        selected_by_provider
            .entry(provider_id)
            .or_default()
            .push(report);
    }

    let mut summaries = Vec::new();
    for (provider_id, total_candidates) in total_by_provider {
        let source_breakdown = selected_by_provider
            .remove(&provider_id)
            .unwrap_or_default();
        summaries.push(KnowledgeDigestDiagnostics {
            provider_id,
            total_candidates,
            selected_candidates: source_breakdown.len(),
            source_breakdown,
        });
    }

    for (provider_id, source_breakdown) in selected_by_provider {
        summaries.push(KnowledgeDigestDiagnostics {
            provider_id,
            total_candidates: source_breakdown.len(),
            selected_candidates: source_breakdown.len(),
            source_breakdown,
        });
    }

    summaries
}

/// Preferred OS entry point: composer first, engine second.
#[derive(Clone)]
pub struct ContextFacade {
    engine: ContextRuntimeFacade,
    composer: DefaultContextComposer,
}

impl ContextFacade {
    /// Same builtin engine registry as [`ContextRuntimeFacade::builtins`], plus the composer.
    pub fn builtins(selection: ContextEngineSelection) -> Self {
        Self {
            engine: ContextRuntimeFacade::builtins(selection),
            composer: DefaultContextComposer::new(),
        }
    }

    /// Same builtin engine registry as [`ContextRuntimeFacade::builtins_with_overlay`], plus the composer.
    pub fn builtins_with_engine_overlay(
        selection: ContextEngineSelection,
        overlay: &crate::engine::ContextEngineRegistry,
    ) -> Self {
        Self {
            engine: ContextRuntimeFacade::builtins_with_overlay(selection, overlay),
            composer: DefaultContextComposer::new(),
        }
    }

    /// Shortcut when both engines are legacy.
    pub fn legacy() -> Self {
        Self::builtins(ContextEngineSelection::legacy())
    }

    pub fn with_composer_fraction(mut self, pct: u32) -> Self {
        self.composer = DefaultContextComposer::new().with_budget_fraction_percent(pct);
        self
    }

    /// Providers → governance pipeline (optional) → **trust policy** (optional) → plan → merge into messages → engine assembly.
    ///
    /// When [`ContextGovernanceRuntimeConfig::enabled`] is `true`, each `ContextProvider::contribute`
    /// call is bounded by `per_provider_timeout_ms` and outputs pass through redaction/deny/validation
    /// strategies before [`ContextComposer::compose`]. When disabled, the legacy ungoverned fan-in is
    /// used to preserve deterministic unit-test baselines.
    ///
    /// Trust promotions run **after** the governed/ungoverned collection finishes but **before**
    /// composer budgeting so prompt trust metadata reflects operator policy.
    pub async fn assemble_model_context(
        &self,
        mut input: ContextAssembleInput,
        providers: &[Arc<dyn ContextProvider>],
        policy: ContextFacadeAssemblyPolicy,
    ) -> MacacaResult<ContextAssembleResult> {
        let ctx = ContextComposeContext {
            assemble_input: &input,
        };

        let gov = policy.governance;

        let (
            mut collected,
            pipeline_notes,
            active_recall_telemetry,
            mut governance_decisions,
            provider_runtime,
        ) = if gov.enabled {
            let bundle = run_governed_provider_chain(providers, &ctx, &gov).await?;
            (
                bundle.candidates,
                bundle.provider_diagnostics,
                bundle.active_recall,
                bundle.governance_decisions,
                Some(bundle.summary),
            )
        } else {
            let (c, n, a) = run_ungoverned_provider_chain(providers, &ctx).await?;
            (c, n, a, Vec::new(), None)
        };

        if policy.knowledge_digest.enabled {
            let (next, digest_decisions) =
                apply_digest_vs_raw_selection(collected, &policy.knowledge_digest);
            collected = next;
            governance_decisions.extend(digest_decisions);
        }

        if let Some(ref tg) = policy.trust_governance {
            governance_decisions.extend(
                crate::governance::trust_policy::apply_trust_policies_to_candidates(
                    &mut collected,
                    tg,
                ),
            );
        }

        let digest_candidates = collected.clone();
        let plan_tag = Uuid::new_v4().to_string();
        let (plan, compiled) =
            ContextComposer::compose(&self.composer, plan_tag, &input, collected)?;
        let summary = summarize_plan(&plan);
        let knowledge_digest =
            summarize_knowledge_digest_candidates(&digest_candidates, &plan.selected);

        input.base_messages = merge_composer_into_messages(input.base_messages.clone(), &compiled);

        let mut assembled = self.engine.assemble(input).await?;
        assembled.report.composer = Some(summary);
        assembled
            .report
            .active_recall
            .extend(active_recall_telemetry);
        assembled.report.knowledge_digest = knowledge_digest;
        assembled.report.provider_runtime = provider_runtime;
        for d in governance_decisions {
            assembled.report.decisions.push(d);
        }
        for note in pipeline_notes {
            assembled.report.decisions.push(ContextDecisionReport {
                code: "context_provider_diagnostic".into(),
                severity: crate::report::ContextDecisionSeverity::Info,
                message: format!("{}: {}", note.provider_id, note.message),
            });
        }
        Ok(assembled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composer::ContextFacadeAssemblyPolicy;
    use crate::prompt::{PromptComposer, PromptSection};
    use crate::report::{ContextSourceKind, ContextSourceReport};

    #[test]
    fn stable_hash_ignores_dynamic_only_when_stable_empty() {
        let stable = PromptSection::builder("s", ContextSourceKind::Workspace)
            .stable()
            .trusted()
            .content("alpha")
            .build();
        let dynamic = PromptSection::builder("d", ContextSourceKind::Memory)
            .dynamic()
            .untrusted()
            .content("beta-change")
            .build();
        let c1 = PromptComposer::new()
            .push_section(stable.clone())
            .push_section(dynamic.clone())
            .compile();
        let dynamic2 = PromptSection::builder("d", ContextSourceKind::Memory)
            .dynamic()
            .untrusted()
            .content("gamma-change")
            .build();
        let c2 = PromptComposer::new()
            .push_section(stable.clone())
            .push_section(dynamic2)
            .compile();
        assert_eq!(c1.stable_hash, c2.stable_hash);
        assert_ne!(c1.full_hash, c2.full_hash);
    }

    #[test]
    fn merge_inserts_after_system_prefix() {
        let base = vec![LlmMessage::system("sys"), LlmMessage::user("hello")];
        let p = PromptComposer::new()
            .push_section(
                PromptSection::builder("x", ContextSourceKind::Workspace)
                    .dynamic()
                    .untrusted()
                    .content("extra")
                    .build(),
            )
            .compile();
        let m = merge_composer_into_messages(base, &p);
        assert_eq!(m.len(), 3);
        assert_eq!(m[0].role, LlmRole::System);
        assert!(m[1].content.contains("MACACA_CONTEXT_COMPOSER"));
        assert_eq!(m[2].content, "hello");
    }

    #[test]
    fn knowledge_digest_summary_tracks_selected_rows() {
        let selected_row = ContextSourceReport::included(
            "claim-1",
            ContextSourceKind::WikiDigest,
            "compiled-knowledge",
            12,
            64,
        )
        .with_rendering("full", "untrusted", None, 0)
        .with_recall_metadata(
            "workspace-knowledge-digest",
            "claim-1",
            88,
            "workspace",
            true,
        );

        let all_candidates = vec![
            crate::composer::ContextCandidate {
                source_id: "knowledge_digest/claim/claim-1".into(),
                kind: crate::composer::ContextCandidateKind::KnowledgeDigest,
                scope: crate::composer::ContextScope::Session,
                priority: 55,
                trust: crate::prompt::TrustLevel::Untrusted,
                cache_class: crate::composer::ContextCacheClass::Dynamic,
                target: crate::composer::ContextTarget::UserSide,
                content: "claim 1".into(),
                token_estimate: 12,
                diagnostics: vec![],
                evidence_memory_ids: vec!["memory-a".into()],
                digest_strength: None,
                source_report: Some(selected_row.clone()),
            },
            crate::composer::ContextCandidate {
                source_id: "knowledge_digest/claim/claim-2".into(),
                kind: crate::composer::ContextCandidateKind::KnowledgeDigest,
                scope: crate::composer::ContextScope::Session,
                priority: 55,
                trust: crate::prompt::TrustLevel::Untrusted,
                cache_class: crate::composer::ContextCacheClass::Dynamic,
                target: crate::composer::ContextTarget::UserSide,
                content: "claim 2".into(),
                token_estimate: 12,
                diagnostics: vec![],
                evidence_memory_ids: vec!["memory-b".into()],
                digest_strength: None,
                source_report: Some(
                    ContextSourceReport::included(
                        "claim-2",
                        ContextSourceKind::WikiDigest,
                        "compiled-knowledge",
                        12,
                        64,
                    )
                    .with_rendering("full", "untrusted", None, 0)
                    .with_recall_metadata(
                        "workspace-knowledge-digest",
                        "claim-2",
                        77,
                        "workspace",
                        true,
                    ),
                ),
            },
        ];

        let summaries =
            summarize_knowledge_digest_candidates(&all_candidates, &all_candidates[..1]);
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].provider_id, "workspace-knowledge-digest");
        assert_eq!(summaries[0].total_candidates, 2);
        assert_eq!(summaries[0].selected_candidates, 1);
        assert_eq!(summaries[0].source_breakdown.len(), 1);
        assert_eq!(
            summaries[0].source_breakdown[0]
                .provenance_source_id
                .as_deref(),
            Some("claim-1")
        );
    }

    #[tokio::test]
    async fn empty_providers_matches_runtime_facade_legacy() {
        use macaca_proto::LlmOptions;

        let input = ContextAssembleInput {
            app_id: None,
            session_id: None,
            agent_name: "a".into(),
            model: "m".into(),
            base_messages: vec![LlmMessage::system("s"), LlmMessage::user("u")],
            options: LlmOptions::default(),
            budget: crate::budget::ContextBudget::default(),
        };

        let f = ContextFacade::legacy();
        let via_facade = f
            .assemble_model_context(input.clone(), &[], ContextFacadeAssemblyPolicy::default())
            .await
            .unwrap();

        let via_engine = ContextRuntimeFacade::legacy()
            .assemble(input)
            .await
            .unwrap();

        assert_eq!(via_facade.messages.len(), via_engine.messages.len());
        for (a, b) in via_facade.messages.iter().zip(via_engine.messages.iter()) {
            assert_eq!(a.role, b.role);
            assert_eq!(a.content, b.content);
        }
    }
}
