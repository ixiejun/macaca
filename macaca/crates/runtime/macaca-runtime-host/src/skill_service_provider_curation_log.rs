//! Structured log helpers for Skill curation provider runs.
//!
//! The runtime-host provider owns operational visibility around command
//! execution, but it must not reimplement curation policy.  This module only
//! aggregates already-typed recommendation phases so logs can expose stable
//! counts without leaking skill bodies, package bytes, raw metadata, prompts,
//! provider payloads, or application-specific workflow names.

use macaca_skill::{SkillCurationPhase, SkillCurationRunResult};

#[derive(Default)]
pub(crate) struct SkillCurationPhaseCounts {
    pub(crate) keep: usize,
    pub(crate) protected: usize,
    pub(crate) quarantine: usize,
    pub(crate) size: usize,
    pub(crate) invalid_metadata: usize,
    pub(crate) missing_dependency: usize,
    pub(crate) stale: usize,
    pub(crate) archive: usize,
    pub(crate) consolidation: usize,
}

/// Count deterministic curation phases for structured operational logs.
pub(crate) fn curation_phase_counts(result: &SkillCurationRunResult) -> SkillCurationPhaseCounts {
    let mut counts = SkillCurationPhaseCounts::default();
    for recommendation in &result.recommendations {
        for phase in &recommendation.phases {
            match phase {
                SkillCurationPhase::Keep => counts.keep += 1,
                SkillCurationPhase::Protected => counts.protected += 1,
                SkillCurationPhase::Quarantine => counts.quarantine += 1,
                SkillCurationPhase::Size => counts.size += 1,
                SkillCurationPhase::InvalidMetadata => counts.invalid_metadata += 1,
                SkillCurationPhase::MissingDependency => counts.missing_dependency += 1,
                SkillCurationPhase::Stale => counts.stale += 1,
                SkillCurationPhase::Archive => counts.archive += 1,
                SkillCurationPhase::Consolidation => counts.consolidation += 1,
            }
        }
    }
    counts
}
