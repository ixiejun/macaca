//! [`ContextFacade`] (Facade): single entry that orchestrates:
//! 1) Chain of Responsibility across [`crate::composer::ContextProvider`];
//! 2) Builder / Composite via [`crate::composer::DefaultContextComposer`];
//! 3) Delegation to [`crate::engine::ContextRuntimeFacade`] for the engine stage.

use std::sync::Arc;

use macaca_proto::{LlmMessage, LlmRole, MacacaResult};
use uuid::Uuid;

use crate::composer::assembly_policy::ContextFacadeAssemblyPolicy;
use crate::composer::default_composer::{ContextComposer, DefaultContextComposer};
use crate::composer::plan::ContextPlan;
use crate::composer::provider::{ContextComposeContext, ContextProvider};
use crate::engine::{
    ContextAssembleInput, ContextAssembleResult, ContextEngineSelection, ContextRuntimeFacade,
};
use crate::governance::{run_governed_provider_chain, run_ungoverned_provider_chain};
use crate::prompt::CompiledPrompt;
use crate::report::{
    ComposerPlanSummary, ComposerSkipRecord, ContextDecisionReport,
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
    }
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

        if let Some(ref tg) = policy.trust_governance {
            governance_decisions.extend(crate::governance::trust_policy::apply_trust_policies_to_candidates(
                &mut collected,
                tg,
            ));
        }

        let plan_tag = Uuid::new_v4().to_string();
        let (plan, compiled) =
            ContextComposer::compose(&self.composer, plan_tag, &input, collected)?;
        let summary = summarize_plan(&plan);

        input.base_messages = merge_composer_into_messages(input.base_messages.clone(), &compiled);

        let mut assembled = self.engine.assemble(input).await?;
        assembled.report.composer = Some(summary);
        assembled.report.active_recall.extend(active_recall_telemetry);
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
    use crate::report::ContextSourceKind;

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
            .assemble_model_context(
                input.clone(),
                &[],
                ContextFacadeAssemblyPolicy::default(),
            )
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
