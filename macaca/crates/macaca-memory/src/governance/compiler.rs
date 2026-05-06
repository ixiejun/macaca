use chrono::{DateTime, Utc};
use macaca_proto::MemoryId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

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

/// Replaceable strategy for deterministic contradiction detection.
pub trait ContradictionStrategy: Send + Sync {
    fn detect(&self, claims: &[KnowledgeClaim]) -> Vec<ClaimGroup>;
}

/// Deterministic contradiction detector for simple negation pairs.
#[derive(Debug, Default)]
pub struct DeterministicContradictionStrategy;

impl ContradictionStrategy for DeterministicContradictionStrategy {
    fn detect(&self, claims: &[KnowledgeClaim]) -> Vec<ClaimGroup> {
        let mut by_positive: HashMap<String, Vec<String>> = HashMap::new();
        let mut by_negative: HashMap<String, Vec<String>> = HashMap::new();

        for claim in claims {
            if let Some(key) = normalized_positive_key(&claim.statement) {
                by_positive.entry(key).or_default().push(claim.id.clone());
            }
            if let Some(key) = normalized_negative_key(&claim.statement) {
                by_negative.entry(key).or_default().push(claim.id.clone());
            }
        }

        let mut groups = Vec::new();
        for (key, positives) in by_positive {
            let Some(negatives) = by_negative.get(&key) else {
                continue;
            };
            let mut claim_ids = positives.clone();
            claim_ids.extend(negatives.clone());
            claim_ids.sort();
            claim_ids.dedup();
            groups.push(ClaimGroup {
                group_id: format!("conflict-{}", stable_group_key(&key)),
                claims: claim_ids,
                reason: format!("deterministic negation conflict for '{key}'"),
            });
        }
        groups
    }
}

/// Default local compiler that lifts high-confidence candidates into claims.
#[derive(Debug, Default)]
pub struct KnowledgeCompiler;

impl KnowledgeCompileCapability for KnowledgeCompiler {
    fn compile(&self, request: KnowledgeCompileRequest) -> KnowledgeCompileResult {
        let mut claims: Vec<KnowledgeClaim> = request
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
        claims.extend(request.existing_claims);
        let conflicts = DeterministicContradictionStrategy.detect(&claims);
        for group in &conflicts {
            for claim in &mut claims {
                if group.claims.iter().any(|id| id == &claim.id) {
                    claim.conflict_group = Some(group.group_id.clone());
                }
            }
        }
        KnowledgeCompileResult {
            scope: request.scope,
            claims,
            conflicts,
            compiled_at: Utc::now(),
        }
    }
}

fn normalized_positive_key(statement: &str) -> Option<String> {
    let normalized = normalize_statement(statement);
    if let Some((left, right)) = normalized.split_once(" requires ") {
        return Some(format!("{} requires {}", left.trim(), right.trim()));
    }
    if let Some((left, right)) = normalized.split_once(" is ") {
        return Some(format!("{} is {}", left.trim(), right.trim()));
    }
    None
}

fn normalized_negative_key(statement: &str) -> Option<String> {
    let normalized = normalize_statement(statement);
    if let Some((left, right)) = normalized.split_once(" does not require ") {
        return Some(format!("{} requires {}", left.trim(), right.trim()));
    }
    if let Some((left, right)) = normalized.split_once(" is not ") {
        return Some(format!("{} is {}", left.trim(), right.trim()));
    }
    None
}

fn normalize_statement(statement: &str) -> String {
    statement
        .trim()
        .trim_end_matches('.')
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn stable_group_key(key: &str) -> String {
    key.chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}
