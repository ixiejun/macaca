use chrono::{DateTime, Utc};
use macaca_proto::MemoryId;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::scope::MemoryScope;

use super::candidate::MemoryCandidate;

/// Confidence attached to a compiled claim.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ClaimConfidence(pub f32);

/// Evidence pointer used to keep the compiled layer auditable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimEvidence {
    pub source_kind: String,
    pub source_id: String,
}

/// Freshness signal attached to a compiled knowledge item.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ClaimFreshness(pub f32);

/// A conflict group keeps contradictory items visible together.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimGroup {
    pub group_id: String,
    pub claims: Vec<String>,
    pub reason: String,
}

/// Structured knowledge claim compiled from memory evidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeClaim {
    pub id: String,
    pub scope: MemoryScope,
    pub statement: String,
    pub evidence: Vec<ClaimEvidence>,
    pub confidence: ClaimConfidence,
    pub freshness: ClaimFreshness,
    pub conflict_group: Option<String>,
    pub supersedes: Vec<MemoryId>,
    pub revoked: bool,
    pub created_at: DateTime<Utc>,
    pub metadata: Value,
}

impl KnowledgeClaim {
    pub fn new(id: impl Into<String>, scope: MemoryScope, statement: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            scope,
            statement: statement.into(),
            evidence: Vec::new(),
            confidence: ClaimConfidence(0.0),
            freshness: ClaimFreshness(0.0),
            conflict_group: None,
            supersedes: Vec::new(),
            revoked: false,
            created_at: Utc::now(),
            metadata: Value::Null,
        }
    }
}

/// Request for the knowledge compiler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeCompileRequest {
    pub scope: MemoryScope,
    pub candidates: Vec<MemoryCandidate>,
    pub existing_claims: Vec<KnowledgeClaim>,
}

/// Output of a compilation pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeCompileResult {
    pub scope: MemoryScope,
    pub claims: Vec<KnowledgeClaim>,
    pub conflicts: Vec<ClaimGroup>,
    pub compiled_at: DateTime<Utc>,
}

/// Replaceable knowledge compiler boundary.
pub trait KnowledgeCompileCapability: Send + Sync {
    fn compile(&self, request: KnowledgeCompileRequest) -> KnowledgeCompileResult;
}

/// Default local compiler that lifts high-confidence candidates into claims.
#[derive(Debug, Default)]
pub struct KnowledgeCompiler;

impl KnowledgeCompileCapability for KnowledgeCompiler {
    fn compile(&self, request: KnowledgeCompileRequest) -> KnowledgeCompileResult {
        let claims = request
            .candidates
            .into_iter()
            .filter(|candidate| candidate.confidence >= 0.75 || candidate.recurrence_count >= 2)
            .map(|candidate| {
                let mut claim = KnowledgeClaim::new(
                    format!("claim-{}", candidate.id),
                    candidate.scope.clone(),
                    candidate.content,
                );
                claim.evidence.push(ClaimEvidence {
                    source_kind: format!("{:?}", candidate.source),
                    source_id: candidate.id,
                });
                claim.confidence = ClaimConfidence(candidate.confidence);
                claim.freshness = ClaimFreshness(1.0);
                claim.metadata = candidate.metadata;
                claim
            })
            .collect();
        KnowledgeCompileResult {
            scope: request.scope,
            claims,
            conflicts: Vec::new(),
            compiled_at: Utc::now(),
        }
    }
}
