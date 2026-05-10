use chrono::{DateTime, Utc};
use macaca_proto::MemoryId;
use serde::{Deserialize, Serialize};

use super::candidate::{CandidateDecision, MemoryCandidate};
use crate::core::scope::MemoryScope;

/// Identifier for a promotion policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryPromotionPolicyId(pub String);

/// Target class for a promoted memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryPromotionTarget {
    AgentPrivate,
    SessionShared,
    ApplicationShared,
    UserScoped,
    KnowledgeLayer,
}

/// Result of evaluating a candidate against a promotion policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPromotionDecision {
    pub policy_id: MemoryPromotionPolicyId,
    pub candidate_id: String,
    pub target: Option<MemoryPromotionTarget>,
    pub decision: CandidateDecision,
    pub reason: String,
    pub target_scope: MemoryScope,
    pub committed_memory_id: Option<MemoryId>,
    pub evaluated_at: DateTime<Utc>,
}

impl MemoryPromotionDecision {
    pub fn new(
        policy_id: impl Into<String>,
        candidate: &MemoryCandidate,
        decision: CandidateDecision,
        reason: impl Into<String>,
        target_scope: MemoryScope,
    ) -> Self {
        Self {
            policy_id: MemoryPromotionPolicyId(policy_id.into()),
            candidate_id: candidate.id.clone(),
            target: None,
            decision,
            reason: reason.into(),
            target_scope,
            committed_memory_id: candidate.committed_memory_id,
            evaluated_at: Utc::now(),
        }
    }
}

/// Replaceable promotion policy for governance.
pub trait MemoryPromotionPolicy: Send + Sync {
    fn policy_id(&self) -> MemoryPromotionPolicyId;
    fn evaluate(&self, candidate: &MemoryCandidate) -> MemoryPromotionDecision;
}

/// Conservative default policy that requires explicitness or high confidence.
#[derive(Debug, Default)]
pub struct ConservativePromotionPolicy;

impl MemoryPromotionPolicy for ConservativePromotionPolicy {
    fn policy_id(&self) -> MemoryPromotionPolicyId {
        MemoryPromotionPolicyId("conservative-default".into())
    }

    fn evaluate(&self, candidate: &MemoryCandidate) -> MemoryPromotionDecision {
        let explicit = matches!(
            candidate.source,
            super::candidate::CandidateSource::UserExplicit
        );
        let recurrent = candidate.recurrence_count >= 2;
        let high_confidence = candidate.confidence >= 0.8;
        let promote = explicit || (recurrent && high_confidence);
        let decision = if promote {
            CandidateDecision::Promote
        } else {
            CandidateDecision::Defer
        };
        let target = if promote {
            Some(match candidate.scope.visibility {
                crate::core::scope::MemoryVisibility::AgentPrivate => {
                    MemoryPromotionTarget::AgentPrivate
                }
                crate::core::scope::MemoryVisibility::SessionShared => {
                    MemoryPromotionTarget::SessionShared
                }
                crate::core::scope::MemoryVisibility::ApplicationShared => {
                    MemoryPromotionTarget::ApplicationShared
                }
                crate::core::scope::MemoryVisibility::UserScoped => {
                    MemoryPromotionTarget::UserScoped
                }
                crate::core::scope::MemoryVisibility::GlobalSystem => {
                    MemoryPromotionTarget::KnowledgeLayer
                }
            })
        } else {
            None
        };
        let mut result = MemoryPromotionDecision::new(
            self.policy_id().0,
            candidate,
            decision,
            if promote {
                "explicit or sufficiently recurrent high-confidence candidate"
            } else {
                "candidate below conservative promotion threshold"
            },
            candidate.scope.clone(),
        );
        result.target = target;
        result
    }
}
