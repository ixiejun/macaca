//! [`DefaultContextComposer`]: default strategy stack (Strategy + Builder).
//!
//! Policy:
//! - **Order**: foundation release encodes ordering primarily via `priority` DESC then
//!   `source_id` ASC (variant tie-break via `ContextCandidateKind` debug order).
//! - **Dedup**: same `source_id` keeps the highest `priority` row.
//! - **Budget**: reserves a slice of `ContextBudget.input_budget()` for composer output so the
//!   engine still has room; default fraction is **20%** (minimum 1024 tokens).

use macaca_proto::MacacaResult;

use crate::budget::ContextBudget;
use crate::composer::candidate::{ContextCacheClass, ContextCandidate, ContextCandidateKind};
use crate::composer::plan::{ContextPlan, ContextPlanDecision};
use crate::composer::provider::validate_candidate;
use crate::engine::ContextAssembleInput;
use crate::prompt::{CompiledPrompt, PromptComposer, PromptSection, PromptStability, TrustLevel};
use crate::report::ContextSourceKind;

/// Builder: consumes collected candidates and emits [`ContextPlan`] plus [`CompiledPrompt`].
#[derive(Debug, Clone, Default)]
pub struct ContextPlanBuilder {
    budget_fraction_percent: u32,
}

impl ContextPlanBuilder {
    pub fn new() -> Self {
        Self {
            budget_fraction_percent: 20,
        }
    }

    /// Sets the percentage (1–100) of input token budget reserved for composer output.
    pub fn budget_fraction_percent(mut self, pct: u32) -> Self {
        self.budget_fraction_percent = pct.clamp(1, 100);
        self
    }

    /// Runs validate → dedupe → sort → budget trim → render `CompiledPrompt`.
    pub fn build(
        &self,
        plan_id: impl Into<String>,
        assemble_input: &ContextAssembleInput,
        mut candidates: Vec<ContextCandidate>,
    ) -> MacacaResult<(ContextPlan, CompiledPrompt)> {
        let plan_id = plan_id.into();
        let mut skipped: Vec<ContextPlanDecision> = Vec::new();

        // 1) Validate: failures become skip rows.
        candidates.retain(|c| match validate_candidate(c) {
            Ok(()) => true,
            Err(msg) => {
                skipped.push(ContextPlanDecision {
                    source_id: c.source_id.clone(),
                    reason_code: "invalid_candidate".into(),
                    message: msg,
                });
                false
            }
        });

        // 2) Drop empty bodies (some providers may be diagnostics-only).
        candidates.retain(|c| {
            if c.content.trim().is_empty() {
                skipped.push(ContextPlanDecision {
                    source_id: c.source_id.clone(),
                    reason_code: "empty_content".into(),
                    message: "Candidate had empty body after trim".into(),
                });
                false
            } else {
                true
            }
        });

        // 3) Dedupe by `source_id`, keeping highest priority first due to prior sort.
        candidates.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then_with(|| a.source_id.cmp(&b.source_id))
        });
        let mut deduped: Vec<ContextCandidate> = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        for c in candidates {
            if seen.insert(c.source_id.clone()) {
                deduped.push(c);
            } else {
                skipped.push(ContextPlanDecision {
                    source_id: c.source_id.clone(),
                    reason_code: "duplicate_source_id".into(),
                    message: "Lower priority duplicate dropped".into(),
                });
            }
        }

        // 4) Final deterministic sort (stable): priority desc, source_id asc, kind ord.
        deduped.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then_with(|| a.source_id.cmp(&b.source_id))
                .then_with(|| format!("{:?}", a.kind).cmp(&format!("{:?}", b.kind)))
        });

        // 5) Budget trim.
        let composer_cap = composer_token_cap(assemble_input.budget, self.budget_fraction_percent);
        let mut used: u32 = 0;
        let mut selected: Vec<ContextCandidate> = Vec::new();
        for c in deduped {
            let need = c.token_estimate.max(1);
            if used.saturating_add(need) > composer_cap {
                skipped.push(ContextPlanDecision {
                    source_id: c.source_id.clone(),
                    reason_code: "budget_exceeded".into(),
                    message: format!(
                        "Composer cap {composer_cap} tokens; would need {need} more (used {used})"
                    ),
                });
                continue;
            }
            used = used.saturating_add(need);
            selected.push(c);
        }

        let plan = ContextPlan {
            plan_id,
            selected: selected.clone(),
            skipped,
        };

        let compiled = render_candidates_to_prompt(selected)?;
        Ok((plan, compiled))
    }
}

/// Computes the composer token cap: fraction of input budget, clamped and at least 1024.
fn composer_token_cap(budget: ContextBudget, pct: u32) -> u32 {
    let input = budget.input_budget();
    let p = pct.clamp(1, 100) as u64;
    let frac = (input as u64).saturating_mul(p) / 100;
    u32::try_from(frac).unwrap_or(u32::MAX).max(1024).min(input)
}

/// Maps candidates to `PromptSection` values then compiles stable/dynamic boundaries.
fn render_candidates_to_prompt(candidates: Vec<ContextCandidate>) -> MacacaResult<CompiledPrompt> {
    let mut composer = PromptComposer::new();
    for c in candidates {
        let stability = match c.effective_cache_class() {
            ContextCacheClass::Stable => PromptStability::Stable,
            ContextCacheClass::Dynamic | ContextCacheClass::Unknown => PromptStability::Dynamic,
        };
        let section_kind = map_candidate_kind_to_source_kind(c.kind);
        let mut builder = PromptSection::builder(format!("composer/{}", c.source_id), section_kind)
            .content(&c.content);
        builder = match stability {
            PromptStability::Stable => builder.stable(),
            PromptStability::Dynamic => builder.dynamic(),
        };
        builder = match c.trust {
            TrustLevel::Trusted => builder.trusted(),
            TrustLevel::Untrusted => builder.untrusted(),
        };
        // `ContextTarget` is reserved for future per-target splits; foundation renders one block.
        let _ = c.target;
        composer = composer.push_section(builder.build());
    }

    Ok(composer.compile())
}

fn map_candidate_kind_to_source_kind(k: ContextCandidateKind) -> ContextSourceKind {
    match k {
        ContextCandidateKind::ProfileBootstrap => ContextSourceKind::Workspace,
        ContextCandidateKind::CapabilityIndex => ContextSourceKind::Skill,
        ContextCandidateKind::KnowledgeDigest => ContextSourceKind::WikiDigest,
        ContextCandidateKind::MemoryRecall => ContextSourceKind::Memory,
        ContextCandidateKind::WorkspaceGuide => ContextSourceKind::Workspace,
        ContextCandidateKind::RuntimeDiagnostic => ContextSourceKind::Trace,
        ContextCandidateKind::Custom => ContextSourceKind::External,
    }
}

/// Default composer container (replaceable implementation later).
#[derive(Debug, Clone)]
pub struct DefaultContextComposer {
    builder: ContextPlanBuilder,
}

impl DefaultContextComposer {
    pub fn new() -> Self {
        Self {
            builder: ContextPlanBuilder::new(),
        }
    }

    pub fn with_budget_fraction_percent(mut self, pct: u32) -> Self {
        self.builder = self.builder.budget_fraction_percent(pct);
        self
    }
}

/// Pluggable composer strategy (swap the default builder implementation).
pub trait ContextComposer: Send + Sync {
    /// Turn flat candidates into a plan plus compiled prompt text.
    fn compose(
        &self,
        plan_id: impl Into<String>,
        input: &ContextAssembleInput,
        candidates: Vec<ContextCandidate>,
    ) -> MacacaResult<(ContextPlan, CompiledPrompt)>;
}

impl ContextComposer for DefaultContextComposer {
    fn compose(
        &self,
        plan_id: impl Into<String>,
        input: &ContextAssembleInput,
        candidates: Vec<ContextCandidate>,
    ) -> MacacaResult<(ContextPlan, CompiledPrompt)> {
        self.builder.build(plan_id, input, candidates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composer::candidate::{ContextScope, ContextTarget};
    use crate::composer::ContextComposer;
    use macaca_proto::{LlmMessage, LlmOptions};

    fn sample_input() -> ContextAssembleInput {
        ContextAssembleInput {
            app_id: None,
            session_id: None,
            agent_name: "agent".into(),
            model: "gpt".into(),
            base_messages: vec![LlmMessage::system("sys"), LlmMessage::user("hi")],
            options: LlmOptions::default(),
            budget: ContextBudget::new(10_000, 1000),
        }
    }

    #[test]
    fn deterministic_ordering() {
        let input = sample_input();
        let c1 = ContextCandidate {
            source_id: "a".into(),
            kind: ContextCandidateKind::Custom,
            scope: ContextScope::Request,
            priority: 10,
            trust: TrustLevel::Trusted,
            cache_class: ContextCacheClass::Dynamic,
            target: ContextTarget::UserSide,
            content: "first".into(),
            token_estimate: 1,
            diagnostics: vec![],
            evidence_memory_ids: vec![],
            digest_strength: None,
            source_report: None,
        };
        let c2 = ContextCandidate {
            source_id: "b".into(),
            kind: ContextCandidateKind::Custom,
            scope: ContextScope::Request,
            priority: 10,
            trust: TrustLevel::Trusted,
            cache_class: ContextCacheClass::Dynamic,
            target: ContextTarget::UserSide,
            content: "second".into(),
            token_estimate: 1,
            diagnostics: vec![],
            evidence_memory_ids: vec![],
            digest_strength: None,
            source_report: None,
        };
        let composer = DefaultContextComposer::new();
        let (p1, _) =
            ContextComposer::compose(&composer, "p", &input, vec![c1.clone(), c2.clone()]).unwrap();
        let (p2, _) = ContextComposer::compose(&composer, "p", &input, vec![c2, c1]).unwrap();
        assert_eq!(p1.selected, p2.selected);
    }

    #[test]
    fn budget_skips_with_reason() {
        let input = sample_input();
        let big = ContextCandidate {
            source_id: "big".into(),
            kind: ContextCandidateKind::Custom,
            scope: ContextScope::Request,
            priority: 100,
            trust: TrustLevel::Untrusted,
            cache_class: ContextCacheClass::Dynamic,
            target: ContextTarget::UserSide,
            content: "x".into(),
            token_estimate: 1_000_000,
            diagnostics: vec![],
            evidence_memory_ids: vec![],
            digest_strength: None,
            source_report: None,
        };
        let composer = DefaultContextComposer::new();
        let (plan, _) = ContextComposer::compose(&composer, "p", &input, vec![big]).unwrap();
        assert!(plan.selected.is_empty());
        assert!(plan
            .skipped
            .iter()
            .any(|s| s.reason_code == "budget_exceeded"));
    }
}
