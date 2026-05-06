//! **Strategy** chain for trust adjustments on [`crate::composer::ContextCandidate`] values.
//!
//! Rules come entirely from [`macaca_proto::config::TrustPromotionRule`] — no application names,
//! workflow ids, or bespoke business predicates.

use macaca_proto::config::{ContextTrustGovernanceConfig, TrustPromotionRule};

use crate::composer::{ContextCandidate, ContextCandidateKind};
use crate::prompt::TrustLevel;
use crate::report::{ContextDecisionReport, ContextDecisionSeverity};

/// Numeric ordering for `TrustLevel` — used for `if_trust_at_most` ceiling checks.
fn trust_rank(t: TrustLevel) -> u8 {
    match t {
        TrustLevel::Untrusted => 0,
        TrustLevel::Trusted => 1,
    }
}

/// Parse operator-configured trust strings (`trusted` | `untrusted`).
pub(crate) fn parse_trust_label(label: &str) -> Option<TrustLevel> {
    match label.trim().to_lowercase().as_str() {
        "trusted" => Some(TrustLevel::Trusted),
        "untrusted" => Some(TrustLevel::Untrusted),
        _ => None,
    }
}

/// Stable snake_case name for [`ContextCandidateKind`] — matches serde on the `ContextReport` side.
pub(crate) fn candidate_kind_snake(kind: ContextCandidateKind) -> &'static str {
    match kind {
        ContextCandidateKind::ProfileBootstrap => "profile_bootstrap",
        ContextCandidateKind::CapabilityIndex => "capability_index",
        ContextCandidateKind::KnowledgeDigest => "knowledge_digest",
        ContextCandidateKind::MemoryRecall => "memory_recall",
        ContextCandidateKind::WorkspaceGuide => "workspace_guide",
        ContextCandidateKind::RuntimeDiagnostic => "runtime_diagnostic",
        ContextCandidateKind::Custom => "custom",
    }
}

fn rule_matches(rule: &TrustPromotionRule, c: &ContextCandidate) -> bool {
    if !rule.match_source_id_prefix.is_empty()
        && !c
            .source_id
            .starts_with(rule.match_source_id_prefix.as_str())
    {
        return false;
    }
    if let Some(ref want_kind) = rule.match_candidate_kind {
        if want_kind.is_empty() || want_kind.to_lowercase().as_str() != candidate_kind_snake(c.kind)
        {
            return false;
        }
    }
    true
}

/// Applies every promotion rule; may change `candidate.trust` in place.
///
/// Returns one decision row per successful change for [`crate::report::ContextReport::decisions`].
pub fn apply_trust_policies_to_candidates(
    candidates: &mut [ContextCandidate],
    cfg: &ContextTrustGovernanceConfig,
) -> Vec<ContextDecisionReport> {
    let mut out = Vec::new();
    if cfg.promotions.is_empty() {
        return out;
    }

    for candidate in candidates.iter_mut() {
        for rule in &cfg.promotions {
            if !rule_matches(rule, candidate) {
                continue;
            }
            let Some(ceiling) = parse_trust_label(&rule.if_trust_at_most) else {
                out.push(ContextDecisionReport {
                    code: "trust_governance_config_error".into(),
                    severity: ContextDecisionSeverity::Warning,
                    message: format!(
                        "invalid if_trust_at_most for rule targeting {}",
                        candidate.source_id
                    ),
                });
                continue;
            };
            let Some(target) = parse_trust_label(&rule.promote_to) else {
                out.push(ContextDecisionReport {
                    code: "trust_governance_config_error".into(),
                    severity: ContextDecisionSeverity::Warning,
                    message: format!(
                        "invalid promote_to for rule targeting {}",
                        candidate.source_id
                    ),
                });
                continue;
            };
            if trust_rank(candidate.trust) > trust_rank(ceiling) {
                continue;
            }
            if candidate.trust != target {
                let before = format!("{:?}", candidate.trust);
                candidate.trust = target;
                out.push(ContextDecisionReport {
                    code: "trust_governance_promotion".into(),
                    severity: ContextDecisionSeverity::Info,
                    message: format!(
                        "adjusted trust for {} from {} to {:?} via operator rule",
                        candidate.source_id, before, target
                    ),
                });
            }
        }
    }
    out
}
