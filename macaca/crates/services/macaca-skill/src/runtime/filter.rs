//! Policy filtering (Chain of Responsibility) for discovered skill entries.
//!
//! Delegates allow/deny and metadata gates to `SkillPolicyChain` so shells and
//! providers can extend rules without modifying discovery or prompt code.

use crate::agent_skill::SkillEntry;
use crate::policy::{
    normalize_policy_set, PolicyDecision, SkillExposureContext, SkillPolicyChain,
};

use super::config::SkillRuntimeOptions;
use super::types::FilteredSkill;

/// Split discovered entries into visible (prompt-eligible) and filtered (denied) sets.
pub(crate) fn filter_entries(
    entries: Vec<SkillEntry>,
    options: &SkillRuntimeOptions,
) -> (Vec<SkillEntry>, Vec<FilteredSkill>) {
    let allow = options
        .policy
        .allow
        .as_ref()
        .map(|items| normalize_policy_set(items.iter().map(String::as_str)));
    let deny = normalize_policy_set(options.policy.deny.iter().map(String::as_str));
    let chain = SkillPolicyChain::default_chain();

    let mut visible = Vec::new();
    let mut filtered = Vec::new();
    for entry in entries {
        let name = entry.skill.name.clone();
        let source = entry.skill.source.clone();
        let ctx = SkillExposureContext {
            allow: allow.as_ref(),
            deny: &deny,
            options,
        };
        match chain.evaluate(&entry, &ctx) {
            PolicyDecision::Allow | PolicyDecision::AllowFinal => visible.push(entry),
            PolicyDecision::Deny(reason) => filtered.push(FilteredSkill {
                name,
                reason,
                source,
            }),
        }
    }
    (visible, filtered)
}
