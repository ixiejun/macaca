//! Context skill lifecycle visibility settings for shell-side catalog assembly.
//!
//! The Skill service owns lifecycle state and policy.  This module is only a
//! small Adapter/Specification layer that reads neutral Context provider
//! parameters and turns them into visibility rules for the compact catalog that
//! `macaca-context` receives.  Keeping the rule here prevents the context crate
//! from importing skill-governance DTOs while still making profile choices
//! explicit and auditable.

use macaca_host_composition::context::catalog::constants::FAMILY_SKILL_CAPABILITY;
use macaca_host_composition::runtime_host::SkillLifecycleState;
use macaca_proto::config::ContextConfig;

/// Provider-neutral profile label for non-active skill rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SkillLifecycleVisibilityProfile {
    /// Production/default profile: only active skills are model-visible.
    Normal,
    /// Review profile: operators may inspect non-active rows except drafts.
    Review,
    /// Experimental profile: draft rows may be included for explicit trials.
    Experimental,
}

/// Visibility settings applied to Skill-service governance snapshots.
#[derive(Debug, Clone)]
pub(crate) struct SkillLifecycleVisibilitySettings {
    profile: SkillLifecycleVisibilityProfile,
    visible_lifecycles: Vec<SkillLifecycleState>,
}

impl Default for SkillLifecycleVisibilitySettings {
    fn default() -> Self {
        Self {
            profile: SkillLifecycleVisibilityProfile::Normal,
            visible_lifecycles: vec![SkillLifecycleState::Active],
        }
    }
}

impl SkillLifecycleVisibilitySettings {
    /// Builds visibility rules from `context.provider_families[]` params.
    ///
    /// Supported `skill_capability` params are intentionally generic:
    /// - `lifecycle_profile`: `normal`, `review`, or `experimental`
    /// - `visible_lifecycles`: labels such as `active`, `stale`, or `draft`
    ///
    /// `Active` is always visible.  Draft rows are ignored unless the profile
    /// is explicitly `experimental`, which keeps draft skills out of normal
    /// and review catalogs even when config accidentally lists them.
    pub(crate) fn from_context_config(config: &ContextConfig) -> Self {
        let Some(row) = config
            .provider_families
            .iter()
            .find(|row| row.family_id == FAMILY_SKILL_CAPABILITY)
        else {
            return Self::default();
        };
        let Some(params) = row.params.as_object() else {
            return Self::default();
        };

        let profile = params
            .get("lifecycle_profile")
            .or_else(|| params.get("visibility_profile"))
            .and_then(|value| value.as_str())
            .map(parse_profile)
            .unwrap_or(SkillLifecycleVisibilityProfile::Normal);
        let mut settings = Self {
            profile,
            visible_lifecycles: vec![SkillLifecycleState::Active],
        };

        if let Some(labels) = params
            .get("visible_lifecycles")
            .or_else(|| params.get("include_lifecycles"))
            .and_then(|value| value.as_array())
        {
            for label in labels.iter().filter_map(|value| value.as_str()) {
                if let Some(lifecycle) = parse_lifecycle(label) {
                    settings.allow_lifecycle(lifecycle);
                }
            }
        } else if matches!(profile, SkillLifecycleVisibilityProfile::Experimental) {
            settings.allow_lifecycle(SkillLifecycleState::Draft);
        }

        settings
    }

    /// Returns true when the compact catalog may expose a row with this
    /// lifecycle.  Non-active rows stay annotated by their lifecycle label in
    /// the `SkillCapabilityRecord`, so reports remain explainable without
    /// reading full skill instructions.
    pub(crate) fn is_visible(&self, lifecycle: &SkillLifecycleState) -> bool {
        self.visible_lifecycles
            .iter()
            .any(|visible| visible == lifecycle)
    }

    pub(crate) fn profile_label(&self) -> &'static str {
        match self.profile {
            SkillLifecycleVisibilityProfile::Normal => "normal",
            SkillLifecycleVisibilityProfile::Review => "review",
            SkillLifecycleVisibilityProfile::Experimental => "experimental",
        }
    }

    fn allow_lifecycle(&mut self, lifecycle: SkillLifecycleState) {
        if lifecycle == SkillLifecycleState::Draft
            && !matches!(self.profile, SkillLifecycleVisibilityProfile::Experimental)
        {
            tracing::warn!(
                profile = self.profile_label(),
                "draft skill lifecycle visibility ignored outside experimental profile"
            );
            return;
        }
        if !self
            .visible_lifecycles
            .iter()
            .any(|visible| visible == &lifecycle)
        {
            self.visible_lifecycles.push(lifecycle);
        }
    }
}

fn parse_profile(value: &str) -> SkillLifecycleVisibilityProfile {
    match value.trim().to_ascii_lowercase().as_str() {
        "experimental" | "experimental_drafts" => SkillLifecycleVisibilityProfile::Experimental,
        "review" | "operator_review" => SkillLifecycleVisibilityProfile::Review,
        _ => SkillLifecycleVisibilityProfile::Normal,
    }
}

fn parse_lifecycle(value: &str) -> Option<SkillLifecycleState> {
    match value.trim().to_ascii_lowercase().as_str() {
        "draft" => Some(SkillLifecycleState::Draft),
        "active" => Some(SkillLifecycleState::Active),
        "stale" => Some(SkillLifecycleState::Stale),
        "archived" => Some(SkillLifecycleState::Archived),
        "quarantined" => Some(SkillLifecycleState::Quarantined),
        "superseded" => Some(SkillLifecycleState::Superseded),
        "rejected" => Some(SkillLifecycleState::Rejected),
        _ => None,
    }
}
