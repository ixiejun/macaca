use async_trait::async_trait;
use chrono::Utc;
use macaca_proto::{AgentId, ApplicationId, MacacaResult, MemoryEntry, MemoryId};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::{
    MemoryDeleteRequest, MemoryFacade, MemoryGetRequest, MemoryScope, MemorySearchRequest,
    MemoryStatusReport, MemoryWriteRequest,
};

use super::{
    audit_candidate_capture, compiled_digest_candidates, ArtifactContent, ArtifactKind,
    CandidateDecision, CandidateSource, CitationArtifact, ClaimGroup, ConservativePromotionPolicy,
    DefaultMemoryCandidateCapturePolicy, DisabledMemoryCompactionStrategy, FacadeDeletePropagator,
    GovernedMemoryFacade, InMemoryGovernanceJournal, KnowledgeCompileCapability,
    KnowledgeCompileRequest, KnowledgeCompiler, MemoryArtifact, MemoryAuditEventKind,
    MemoryCandidate, MemoryCandidateCapture, MemoryCandidateCapturePolicy, MemoryCandidateStore,
    MemoryCompactionStrategy, MemoryDeletePropagation, MemoryDeletePropagationStep,
    MemoryGovernanceJournal, MemoryPromotionPolicy, MemoryProviderMigrationPlan,
    MemoryProviderMigrationPort, MemoryProviderMigrationRuntime, MemoryProviderMigrationStatus,
    MemoryTombstone, ProjectDecisionLogArtifact, WikiDigestArtifact,
};

fn private_scope() -> MemoryScope {
    MemoryScope::agent_private(ApplicationId::new(), AgentId::new())
}

fn candidate(id: &str, source: CandidateSource, confidence: f32) -> MemoryCandidate {
    MemoryCandidate::new(
        id,
        private_scope(),
        source,
        "remember the architecture invariant",
    )
    .confidence(confidence)
}

fn statement_candidate(id: &str, statement: &str) -> MemoryCandidate {
    MemoryCandidate::new(
        id,
        private_scope(),
        CandidateSource::AgentSummary,
        statement,
    )
    .confidence(0.9)
    .recurrence_count(2)
}

#[derive(Default)]
struct FakeMemoryFacade {
    entries: RwLock<Vec<MemoryEntry>>,
    delete_fails: bool,
}

#[async_trait]
impl MemoryFacade for FakeMemoryFacade {
    async fn remember(&self, request: MemoryWriteRequest) -> MacacaResult<MemoryId> {
        let id = MemoryId::new();
        self.entries.write().await.push(MemoryEntry {
            id,
            layer: request.layer,
            content: request.content,
            metadata: request.metadata,
            agent_id: request.scope.agent_id_value(),
            created_at: Utc::now(),
            expires_at: None,
        });
        Ok(id)
    }

    async fn search(&self, _request: MemorySearchRequest) -> MacacaResult<Vec<MemoryEntry>> {
        Ok(self.entries.read().await.clone())
    }

    async fn get(&self, request: MemoryGetRequest) -> MacacaResult<Option<MemoryEntry>> {
        Ok(self
            .entries
            .read()
            .await
            .iter()
            .find(|entry| entry.id == request.id)
            .cloned())
    }

    async fn delete(&self, request: MemoryDeleteRequest) -> MacacaResult<()> {
        if self.delete_fails {
            return Err(macaca_proto::MacacaError::Memory("delete failed".into()));
        }
        self.entries
            .write()
            .await
            .retain(|entry| entry.id != request.id);
        Ok(())
    }

    fn status(&self) -> MemoryStatusReport {
        MemoryStatusReport::healthy("fake", crate::MemoryCapabilitySet::basic_store_search())
    }
}

#[test]
fn candidate_capture_does_not_commit_long_term_memory() {
    let mut store = MemoryCandidateCapture::new();
    let id = store.capture(candidate("candidate-1", CandidateSource::AutoCapture, 0.4));
    let rows = store.list(None);

    assert_eq!(id, "candidate-1");
    assert_eq!(rows.len(), 1);
    assert!(rows[0].committed_memory_id.is_none());
}

#[test]
fn conservative_policy_defers_weak_auto_capture() {
    let policy = ConservativePromotionPolicy;
    let decision = policy.evaluate(&candidate("candidate-1", CandidateSource::AutoCapture, 0.4));

    assert_eq!(decision.decision, CandidateDecision::Defer);
    assert!(decision.target.is_none());
    assert_eq!(decision.policy_id.0, "conservative-default");
}

#[test]
fn conservative_policy_promotes_explicit_memory() {
    let policy = ConservativePromotionPolicy;
    let decision = policy.evaluate(&candidate(
        "candidate-1",
        CandidateSource::UserExplicit,
        0.4,
    ));

    assert_eq!(decision.decision, CandidateDecision::Promote);
    assert!(decision.target.is_some());
}

#[tokio::test]
async fn governed_facade_audits_explicit_memory_writes() {
    let inner = Arc::new(FakeMemoryFacade::default());
    let governed = GovernedMemoryFacade::new(inner, ConservativePromotionPolicy);
    let id = governed
        .remember(MemoryWriteRequest::new(
            private_scope(),
            "explicit high-confidence memory",
        ))
        .await
        .unwrap();
    let audits = governed.audit_events().await;

    assert_eq!(audits.len(), 1);
    assert_eq!(audits[0].kind, MemoryAuditEventKind::Promoted);
    assert_eq!(audits[0].memory_id, Some(id));
}

#[tokio::test]
async fn governed_facade_promotes_candidate_through_inner_facade() {
    let inner = Arc::new(FakeMemoryFacade::default());
    let governed = GovernedMemoryFacade::new(inner, ConservativePromotionPolicy);
    governed
        .capture_candidate(candidate("candidate-1", CandidateSource::UserExplicit, 0.6))
        .await
        .unwrap();
    let result = governed
        .promote_candidate("candidate-1")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(result.decision.decision, CandidateDecision::Promote);
    assert!(result.committed_memory_id.is_some());
    assert_eq!(governed.audit_events().await.len(), 2);
}

#[test]
fn tombstone_remains_authoritative_when_propagation_fails() {
    let tombstone = MemoryTombstone::new(MemoryId::new(), private_scope(), "user requested delete");
    let propagation = MemoryDeletePropagation {
        tombstone: tombstone.clone(),
        steps: vec![MemoryDeletePropagationStep {
            backend: "remote-provider".into(),
            attempted: true,
            succeeded: false,
            retryable: true,
            message: "timeout".into(),
        }],
    };

    assert_eq!(propagation.tombstone.memory_id, tombstone.memory_id);
    assert!(propagation.steps[0].retryable);
    assert!(propagation.tombstone.propagate);
}

#[tokio::test]
async fn governed_facade_tombstones_and_records_delete_propagation() {
    let primary = Arc::new(FakeMemoryFacade::default());
    let secondary = Arc::new(FakeMemoryFacade {
        entries: RwLock::new(Vec::new()),
        delete_fails: true,
    });
    let governed = GovernedMemoryFacade::new(primary.clone(), ConservativePromotionPolicy)
        .with_delete_propagator(Arc::new(FacadeDeletePropagator::new(
            "secondary",
            secondary,
        )));
    let id = governed
        .remember(MemoryWriteRequest::new(private_scope(), "delete me"))
        .await
        .unwrap();
    governed
        .delete(MemoryDeleteRequest {
            scope: private_scope(),
            id,
        })
        .await
        .unwrap();
    let tombstones = governed.tombstones().await;
    let propagations = governed.propagations().await;

    assert_eq!(tombstones.len(), 1);
    assert_eq!(propagations[0].steps.len(), 2);
    assert!(propagations[0].steps.iter().any(|step| step.retryable));
}

#[test]
fn compiler_emits_claims_with_evidence_for_strong_candidates() {
    let scope = private_scope();
    let compiler = KnowledgeCompiler;
    let result = compiler.compile(KnowledgeCompileRequest {
        scope: scope.clone(),
        candidates: vec![candidate("candidate-1", CandidateSource::AgentSummary, 0.9)],
        existing_claims: Vec::new(),
    });

    assert_eq!(result.scope, scope);
    assert_eq!(result.claims.len(), 1);
    assert_eq!(result.claims[0].evidence[0].source_id, "candidate-1");
}

#[test]
fn compiler_groups_boolean_contradictions_by_subject() {
    let compiler = KnowledgeCompiler;
    let result = compiler.compile(KnowledgeCompileRequest {
        scope: private_scope(),
        candidates: vec![
            statement_candidate("candidate-a", "deployment requires redis"),
            statement_candidate("candidate-b", "deployment does not require redis"),
        ],
        existing_claims: Vec::new(),
    });

    assert_eq!(result.conflicts.len(), 1);
    let conflict = &result.conflicts[0];
    assert!(conflict.claims.contains(&"claim-candidate-a".to_string()));
    assert!(conflict.claims.contains(&"claim-candidate-b".to_string()));
    assert!(result
        .claims
        .iter()
        .all(|claim| claim.conflict_group == Some(conflict.group_id.clone())));
}

#[test]
fn compiler_preserves_exact_evidence_ids_for_claims() {
    let compiler = KnowledgeCompiler;
    let result = compiler.compile(KnowledgeCompileRequest {
        scope: private_scope(),
        candidates: vec![statement_candidate("memory-row-42", "runtime is pluggable")],
        existing_claims: Vec::new(),
    });

    assert_eq!(result.claims.len(), 1);
    assert_eq!(result.claims[0].evidence[0].source_id, "memory-row-42");
}

#[test]
fn compiled_digest_exports_context_candidates() {
    let compiler = KnowledgeCompiler;
    let result = compiler.compile(KnowledgeCompileRequest {
        scope: private_scope(),
        candidates: vec![candidate("candidate-1", CandidateSource::AgentSummary, 0.9)],
        existing_claims: Vec::new(),
    });
    let candidates = compiled_digest_candidates(&result);

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].source_id, "claim-candidate-1");
    assert_eq!(candidates[0].evidence[0], "AgentSummary:candidate-1");
    assert!(candidates[0].redacted);
}

#[test]
fn conflict_groups_preserve_contradictory_claim_ids() {
    let group = ClaimGroup {
        group_id: "conflict-architecture-source".into(),
        claims: vec!["claim-old".into(), "claim-new".into()],
        reason: "newer evidence supersedes older architecture note".into(),
    };

    assert_eq!(group.claims.len(), 2);
    assert!(group.reason.contains("supersedes"));
}

#[test]
fn artifacts_default_to_redacted_summaries() {
    let artifact = MemoryArtifact {
        id: "memory-report".into(),
        kind: ArtifactKind::MemoryReport,
        scope: private_scope(),
        path: Some("memory/report.md".into()),
        content: ArtifactContent {
            content_type: "text/markdown".into(),
            body: "summary only; evidence ids omitted from this fixture".into(),
            redacted: true,
            metadata: serde_json::Value::Null,
        },
        updated_at: Utc::now(),
    };

    assert!(artifact.content.redacted);
    assert!(!artifact.content.body.contains("secret"));
}

#[test]
fn typed_artifacts_keep_evidence_refs_without_raw_secret_body() {
    let base = MemoryArtifact {
        id: "wiki-digest".into(),
        kind: ArtifactKind::WikiDigest,
        scope: private_scope(),
        path: Some("memory/wiki.md".into()),
        content: ArtifactContent {
            content_type: "text/markdown".into(),
            body: "summary only; evidence_refs=[evidence-1]".into(),
            redacted: true,
            metadata: serde_json::json!({"evidence_ids":["evidence-1"]}),
        },
        updated_at: Utc::now(),
    };
    let wiki = WikiDigestArtifact {
        artifact: base.clone(),
        evidence_ids: vec!["evidence-1".into()],
    };
    let decision = ProjectDecisionLogArtifact {
        artifact: MemoryArtifact {
            kind: ArtifactKind::ProjectDecisionLog,
            ..base.clone()
        },
        evidence_ids: wiki.evidence_ids.clone(),
    };
    let citation = CitationArtifact {
        artifact: MemoryArtifact {
            kind: ArtifactKind::Citation,
            ..base
        },
        evidence_ids: vec!["evidence-1".into()],
    };

    assert!(wiki.artifact.content.redacted);
    assert!(!wiki.artifact.content.body.contains("raw-secret"));
    assert_eq!(decision.evidence_ids, vec!["evidence-1"]);
    assert_eq!(citation.evidence_ids, vec!["evidence-1"]);
}

#[tokio::test]
async fn governance_runtime_captures_candidate_and_lists_audit() {
    let journal = InMemoryGovernanceJournal::new();
    let candidate = candidate("candidate-runtime-1", CandidateSource::AgentSummary, 0.9);
    audit_candidate_capture(&journal, &candidate).await.unwrap();
    let audits = journal.list_audits(&candidate.scope, 10).await.unwrap();

    assert_eq!(audits.len(), 1);
    assert_eq!(audits[0].kind, MemoryAuditEventKind::CapturedCandidate);
    assert_eq!(
        audits[0].candidate_id.as_deref(),
        Some("candidate-runtime-1")
    );
}

#[test]
fn default_candidate_capture_policy_is_conservative() {
    let policy = DefaultMemoryCandidateCapturePolicy;
    assert!(policy.should_capture(&CandidateSource::UserExplicit, "remember this"));
    assert!(policy.should_capture(&CandidateSource::AgentSummary, "summary"));
    assert!(!policy.should_capture(&CandidateSource::AutoCapture, "ordinary chat"));
    assert!(!policy.should_capture(&CandidateSource::UserExplicit, "   "));
}

#[tokio::test]
async fn disabled_compaction_strategy_is_truthful() {
    let strategy = DisabledMemoryCompactionStrategy;
    let result = strategy.compact(private_scope(), Vec::new()).await.unwrap();

    assert!(result.candidates.is_empty());
    assert_eq!(result.diagnostics, vec!["compaction_disabled"]);
}

#[derive(Clone)]
struct MigrationPort {
    entries: Arc<RwLock<Vec<MemoryEntry>>>,
    drop_writes: bool,
}

#[async_trait]
impl MemoryProviderMigrationPort for MigrationPort {
    async fn list_entries(
        &self,
        _scope: &MemoryScope,
        limit: usize,
    ) -> MacacaResult<Vec<MemoryEntry>> {
        let mut rows = self.entries.read().await.clone();
        rows.truncate(limit);
        Ok(rows)
    }

    async fn write_entry(&self, entry: MemoryEntry) -> MacacaResult<()> {
        if !self.drop_writes {
            self.entries.write().await.push(entry);
        }
        Ok(())
    }
}

#[tokio::test]
async fn migration_keeps_source_authoritative_when_verification_fails() {
    let scope = private_scope();
    let source = MigrationPort {
        entries: Arc::new(RwLock::new(vec![
            MemoryEntry {
                id: MemoryId::new(),
                layer: macaca_proto::MemoryLayer::Session,
                content: "one".into(),
                metadata: serde_json::Value::Null,
                agent_id: None,
                created_at: Utc::now(),
                expires_at: None,
            },
            MemoryEntry {
                id: MemoryId::new(),
                layer: macaca_proto::MemoryLayer::Session,
                content: "two".into(),
                metadata: serde_json::Value::Null,
                agent_id: None,
                created_at: Utc::now(),
                expires_at: None,
            },
        ])),
        drop_writes: false,
    };
    let target = MigrationPort {
        entries: Arc::new(RwLock::new(Vec::new())),
        drop_writes: true,
    };
    let runtime = MemoryProviderMigrationRuntime::new(InMemoryGovernanceJournal::new());
    let result = runtime
        .migrate(
            MemoryProviderMigrationPlan {
                source_provider_id: "source".into(),
                target_provider_id: "target".into(),
                scope,
                batch_size: 10,
            },
            &source,
            &target,
        )
        .await
        .unwrap();

    assert_eq!(result.status, MemoryProviderMigrationStatus::Failed);
    assert!(result.source_authoritative);
    assert_eq!(result.copied_entries, 2);
    assert_eq!(result.verified_entries, 0);
}
