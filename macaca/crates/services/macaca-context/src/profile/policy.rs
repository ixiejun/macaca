//! Default **Strategy** mapping from [`super::kinds::AgentProfileFileKind`] to composer knobs.
//!
//! Separating this table from I/O keeps the provider easy to unit-test and lets integrators swap
//! policies later without touching the filesystem template method.

use macaca_proto::config::AgentProfileContextConfig;

use crate::composer::{ContextCacheClass, ContextCandidateKind, ContextScope, ContextTarget};
use crate::prompt::TrustLevel;

use super::kinds::AgentProfileFileKind;

/// Immutable bundle of composer hints for one profile file kind at runtime.
///
/// Policy objects are cheap to clone; they never embed raw file bytes — only metadata used when a
/// load succeeds.
#[derive(Debug, Clone)]
pub struct ProfileKindPolicy {
    /// Used by [`crate::composer::DefaultContextComposer`] to rank candidates before budgeting.
    pub priority: u32,
    /// Stable files can participate in stable prompt hashing when trust/cache rules allow.
    pub cache_class: ContextCacheClass,
    /// Rendering bucket for reporting (`ProfileBootstrap` maps to workspace-like classification).
    pub candidate_kind: ContextCandidateKind,
    /// All Markdown profile injections are authored by the application operator for this agent.
    pub trust: TrustLevel,
    /// Hermes-style placement hint: stable identity text targets the stable system lane by default.
    pub target: ContextTarget,
    /// Semantic scope boundary for audit reports.
    pub scope: ContextScope,
    /// Additional diagnostics lines attached when the candidate is emitted (never sent verbatim to
    /// the model unless another layer chooses to mirror them).
    pub static_diagnostics: &'static [&'static str],
}

/// Computes the default policy table bundled with Macaca.
///
/// Priority rationale (higher beats lower under budget contention):
/// - `AGENTS` / `SOUL` shape global behaviour and should survive trimming longest.
/// - `TOOLS` / `IDENTITY` are still foundational but slightly less “macro-policy”.
/// - `MEMORY` is **seed / audit** — important but must never be mistaken for durable recall.
/// - `USER` is personalization that can change frequently.
/// - `HEARTBEAT` is intentionally lowest because it often contains cadence noise.
#[must_use]
pub fn default_policy_for(
    kind: AgentProfileFileKind,
    _config: &AgentProfileContextConfig,
) -> ProfileKindPolicy {
    let _ = _config; // Hook for future per-field overrides driven by `AgentProfileContextConfig`.
    match kind {
        AgentProfileFileKind::Agents => ProfileKindPolicy {
            priority: 120,
            cache_class: ContextCacheClass::Stable,
            candidate_kind: ContextCandidateKind::ProfileBootstrap,
            trust: TrustLevel::Trusted,
            target: ContextTarget::SystemStable,
            scope: ContextScope::Agent,
            static_diagnostics: &[],
        },
        AgentProfileFileKind::Soul => ProfileKindPolicy {
            priority: 115,
            cache_class: ContextCacheClass::Stable,
            candidate_kind: ContextCandidateKind::ProfileBootstrap,
            trust: TrustLevel::Trusted,
            target: ContextTarget::SystemStable,
            scope: ContextScope::Agent,
            static_diagnostics: &[],
        },
        AgentProfileFileKind::Tools => ProfileKindPolicy {
            priority: 80,
            cache_class: ContextCacheClass::Stable,
            candidate_kind: ContextCandidateKind::ProfileBootstrap,
            trust: TrustLevel::Trusted,
            target: ContextTarget::SystemStable,
            scope: ContextScope::Agent,
            static_diagnostics: &[],
        },
        AgentProfileFileKind::Identity => ProfileKindPolicy {
            priority: 75,
            cache_class: ContextCacheClass::Stable,
            candidate_kind: ContextCandidateKind::ProfileBootstrap,
            trust: TrustLevel::Trusted,
            target: ContextTarget::SystemStable,
            scope: ContextScope::Agent,
            static_diagnostics: &[],
        },
        AgentProfileFileKind::Memory => ProfileKindPolicy {
            priority: 60,
            cache_class: ContextCacheClass::Dynamic,
            candidate_kind: ContextCandidateKind::ProfileBootstrap,
            trust: TrustLevel::Trusted,
            target: ContextTarget::UserSide,
            scope: ContextScope::Agent,
            static_diagnostics: &[concat!(
                "memory_seed_audit_only:",
                "this text seeds guidelines — it is never auto-ingested into vector memory by the ",
                "profile provider."
            )],
        },
        AgentProfileFileKind::User => ProfileKindPolicy {
            priority: 45,
            cache_class: ContextCacheClass::Dynamic,
            candidate_kind: ContextCandidateKind::ProfileBootstrap,
            trust: TrustLevel::Trusted,
            target: ContextTarget::UserSide,
            scope: ContextScope::Session,
            static_diagnostics: &[],
        },
        AgentProfileFileKind::Heartbeat => ProfileKindPolicy {
            priority: 35,
            cache_class: ContextCacheClass::Dynamic,
            candidate_kind: ContextCandidateKind::ProfileBootstrap,
            trust: TrustLevel::Trusted,
            target: ContextTarget::SystemDynamic,
            scope: ContextScope::Request,
            static_diagnostics: &[],
        },
    }
}
