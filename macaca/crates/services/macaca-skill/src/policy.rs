//! Skill exposure policy chain.

use std::collections::HashSet;
use std::env;

use crate::agent_skill::SkillEntry;
use crate::runtime::SkillRuntimeOptions;

/// Decision returned by a skill exposure policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDecision {
    /// Continue evaluating remaining policies.
    Allow,
    /// Stop the chain and expose the skill.
    AllowFinal,
    /// Stop the chain and hide the skill with a stable reason.
    Deny(String),
}

impl PolicyDecision {
    pub fn deny(reason: impl Into<String>) -> Self {
        Self::Deny(reason.into())
    }
}

/// One skill exposure decision step.
pub trait SkillExposurePolicy: Send + Sync {
    fn allows(&self, entry: &SkillEntry, ctx: &SkillExposureContext<'_>) -> PolicyDecision;
}

/// Runtime context available to all exposure policies.
pub struct SkillExposureContext<'a> {
    pub allow: Option<&'a HashSet<String>>,
    pub deny: &'a HashSet<String>,
    pub options: &'a SkillRuntimeOptions,
}

/// Ordered chain of skill exposure policies.
pub struct SkillPolicyChain {
    policies: Vec<Box<dyn SkillExposurePolicy>>,
}

impl SkillPolicyChain {
    pub fn default_chain() -> Self {
        Self {
            policies: vec![
                Box::new(AllowDenyPolicy),
                Box::new(ModelInvocationPolicy),
                Box::new(MetadataAlwaysPolicy),
                Box::new(OsPolicy),
                Box::new(BinaryPolicy),
                Box::new(EnvironmentPolicy),
                Box::new(ConfigPolicy),
            ],
        }
    }

    pub fn evaluate(&self, entry: &SkillEntry, ctx: &SkillExposureContext<'_>) -> PolicyDecision {
        for policy in &self.policies {
            match policy.allows(entry, ctx) {
                PolicyDecision::Allow => {}
                allowed @ PolicyDecision::AllowFinal => return allowed,
                denied @ PolicyDecision::Deny(_) => return denied,
            }
        }
        PolicyDecision::Allow
    }
}

struct AllowDenyPolicy;

impl SkillExposurePolicy for AllowDenyPolicy {
    fn allows(&self, entry: &SkillEntry, ctx: &SkillExposureContext<'_>) -> PolicyDecision {
        let key = entry
            .metadata
            .skill_key
            .as_deref()
            .unwrap_or(entry.skill.name.as_str());
        if ctx.deny.contains(entry.skill.name.as_str()) || ctx.deny.contains(key) {
            return PolicyDecision::deny("denied_by_policy");
        }
        if let Some(allow) = ctx.allow {
            if !allow.contains(entry.skill.name.as_str()) && !allow.contains(key) {
                return PolicyDecision::deny("denied_by_policy");
            }
        }
        PolicyDecision::Allow
    }
}

struct ModelInvocationPolicy;

impl SkillExposurePolicy for ModelInvocationPolicy {
    fn allows(&self, entry: &SkillEntry, _ctx: &SkillExposureContext<'_>) -> PolicyDecision {
        if entry.invocation.disable_model_invocation {
            return PolicyDecision::deny("disabled_model_invocation");
        }
        PolicyDecision::Allow
    }
}

struct MetadataAlwaysPolicy;

impl SkillExposurePolicy for MetadataAlwaysPolicy {
    fn allows(&self, entry: &SkillEntry, _ctx: &SkillExposureContext<'_>) -> PolicyDecision {
        if entry.metadata.always {
            return PolicyDecision::AllowFinal;
        }
        PolicyDecision::Allow
    }
}

struct OsPolicy;

impl SkillExposurePolicy for OsPolicy {
    fn allows(&self, entry: &SkillEntry, _ctx: &SkillExposureContext<'_>) -> PolicyDecision {
        if !entry.metadata.os.is_empty()
            && !entry.metadata.os.iter().any(|os| os_matches_current(os))
        {
            return PolicyDecision::deny("os_mismatch");
        }
        PolicyDecision::Allow
    }
}

struct BinaryPolicy;

impl SkillExposurePolicy for BinaryPolicy {
    fn allows(&self, entry: &SkillEntry, _ctx: &SkillExposureContext<'_>) -> PolicyDecision {
        for bin in &entry.metadata.requires_bins {
            if !has_binary(bin) {
                return PolicyDecision::deny("missing_bin");
            }
        }
        if !entry.metadata.requires_any_bins.is_empty()
            && !entry
                .metadata
                .requires_any_bins
                .iter()
                .any(|bin| has_binary(bin))
        {
            return PolicyDecision::deny("missing_bin");
        }
        PolicyDecision::Allow
    }
}

struct EnvironmentPolicy;

impl SkillExposurePolicy for EnvironmentPolicy {
    fn allows(&self, entry: &SkillEntry, ctx: &SkillExposureContext<'_>) -> PolicyDecision {
        for env_name in &entry.metadata.requires_env {
            if env::var_os(env_name).is_none() && !ctx.options.env_overrides.contains(env_name) {
                return PolicyDecision::deny("missing_env");
            }
        }
        PolicyDecision::Allow
    }
}

struct ConfigPolicy;

impl SkillExposurePolicy for ConfigPolicy {
    fn allows(&self, entry: &SkillEntry, ctx: &SkillExposureContext<'_>) -> PolicyDecision {
        for config in &entry.metadata.requires_config {
            if !ctx.options.config_flags.contains(config) {
                return PolicyDecision::deny("missing_config");
            }
        }
        PolicyDecision::Allow
    }
}

pub fn normalize_policy_set<'a>(items: impl Iterator<Item = &'a str>) -> HashSet<String> {
    items
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

fn has_binary(bin: &str) -> bool {
    let bin = bin.trim();
    if bin.is_empty() {
        return false;
    }
    let Some(paths) = env::var_os("PATH") else {
        return false;
    };
    env::split_paths(&paths).any(|dir| dir.join(bin).is_file())
}

pub(crate) fn os_matches_current(skill_os: &str) -> bool {
    let requested = skill_os.trim().to_ascii_lowercase();
    let current = env::consts::OS;
    requested == current
        || matches!(
            (requested.as_str(), current),
            ("darwin", "macos") | ("macos", "macos")
        )
}
