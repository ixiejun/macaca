//! Production memory runtime boundary.
//!
//! This module deliberately stays small: it composes already-built memory
//! capabilities behind one facade so upper crates can stop depending on legacy
//! managers directly. The runtime itself does not choose providers, build
//! embeddings, or talk to vendor SDKs; those responsibilities stay behind the
//! lower-layer provider/runtime abstractions.

pub mod builder;
pub mod facade;
pub mod status;

pub use builder::MemoryRuntimeBuilder;
pub use facade::{ComposedMemoryRuntime, MemoryRuntimeFacade};
pub use status::MemoryRuntimeStatus;

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use macaca_proto::{ApplicationId, MemoryEntry, MemoryId, MemoryLayer};

    use crate::core::status::MemoryCapabilitySet;
    use crate::core::{
        ActiveRecallBudget, ActiveRecallCandidate, ActiveRecallCapability, ActiveRecallDecision,
        ActiveRecallRequest, ActiveRecallResult, MemoryDeleteRequest, MemoryFacade,
        MemoryGetRequest, MemorySearchRequest, MemoryStatusReport, MemoryWriteRequest,
    };
    use crate::governance::{
        ClaimConfidence, ClaimEvidence, ClaimFreshness, KnowledgeClaim, KnowledgeCompileCapability,
        KnowledgeCompileRequest, KnowledgeCompileResult,
    };
    use crate::runtime::{ComposedMemoryRuntime, MemoryRuntimeBuilder, MemoryRuntimeFacade};
    use crate::MemoryScope;

    struct FakeFacade {
        remembers: Arc<Mutex<Vec<String>>>,
        searches: Arc<Mutex<Vec<String>>>,
        status: MemoryStatusReport,
    }

    impl FakeFacade {
        fn new(status: MemoryStatusReport) -> Self {
            Self {
                remembers: Arc::new(Mutex::new(Vec::new())),
                searches: Arc::new(Mutex::new(Vec::new())),
                status,
            }
        }
    }

    #[async_trait]
    impl MemoryFacade for FakeFacade {
        async fn remember(
            &self,
            request: MemoryWriteRequest,
        ) -> macaca_proto::MacacaResult<MemoryId> {
            self.remembers.lock().unwrap().push(request.content.clone());
            Ok(MemoryId::new())
        }

        async fn search(
            &self,
            request: MemorySearchRequest,
        ) -> macaca_proto::MacacaResult<Vec<MemoryEntry>> {
            self.searches.lock().unwrap().push(request.query.clone());
            Ok(vec![MemoryEntry {
                id: MemoryId::new(),
                layer: MemoryLayer::Session,
                content: request.query,
                metadata: serde_json::Value::Null,
                agent_id: None,
                created_at: chrono::Utc::now(),
                expires_at: None,
            }])
        }

        async fn get(
            &self,
            _request: MemoryGetRequest,
        ) -> macaca_proto::MacacaResult<Option<MemoryEntry>> {
            Ok(None)
        }

        async fn delete(&self, _request: MemoryDeleteRequest) -> macaca_proto::MacacaResult<()> {
            Ok(())
        }

        fn status(&self) -> MemoryStatusReport {
            self.status.clone()
        }
    }

    #[derive(Default)]
    struct FakeActiveRecall;

    #[async_trait]
    impl ActiveRecallCapability for FakeActiveRecall {
        async fn prefetch(
            &self,
            request: ActiveRecallRequest,
        ) -> macaca_proto::MacacaResult<ActiveRecallResult> {
            let entry = MemoryEntry {
                id: MemoryId::new(),
                layer: MemoryLayer::Session,
                content: request.query,
                metadata: serde_json::Value::Null,
                agent_id: None,
                created_at: chrono::Utc::now(),
                expires_at: None,
            };
            Ok(ActiveRecallResult {
                provider_id: "fake-active-recall".into(),
                candidates: vec![ActiveRecallCandidate {
                    entry: entry.clone(),
                    score: 99,
                    estimated_tokens: 3,
                    decision: ActiveRecallDecision::selected("within_budget"),
                }],
                selected: vec![entry],
                latency_ms: 1,
                diagnostics: vec!["ok".into()],
            })
        }
    }

    #[derive(Default)]
    struct FakeKnowledgeCompiler;

    impl KnowledgeCompileCapability for FakeKnowledgeCompiler {
        fn compile(&self, request: KnowledgeCompileRequest) -> KnowledgeCompileResult {
            let mut claim = KnowledgeClaim::new("claim-1", request.scope, "compiled claim");
            claim.confidence = ClaimConfidence(0.9);
            claim.freshness = ClaimFreshness(0.8);
            claim.evidence.push(ClaimEvidence {
                source_kind: "memory".into(),
                source_id: "evidence-1".into(),
            });
            KnowledgeCompileResult {
                scope: claim.scope.clone(),
                claims: vec![claim],
                conflicts: Vec::new(),
                compiled_at: chrono::Utc::now(),
            }
        }
    }

    #[tokio::test]
    async fn runtime_facade_routes_through_inner_components() {
        let facade = Arc::new(FakeFacade::new(MemoryStatusReport::healthy(
            "fake-provider",
            MemoryCapabilitySet {
                store: true,
                search: true,
                prompt: false,
                lifecycle: false,
                flush: false,
                artifact: false,
                governance: false,
                knowledge: false,
            },
        )));
        let active_recall = Arc::new(FakeActiveRecall);
        let knowledge = Arc::new(FakeKnowledgeCompiler);
        let runtime =
            MemoryRuntimeBuilder::new(facade.clone(), active_recall.clone(), knowledge.clone())
                .runtime_id("memory-runtime-test")
                .provider_profile("profile-a")
                .build();

        let scope = MemoryScope::session_shared(ApplicationId::new(), "session-1");
        let id = runtime
            .remember(MemoryWriteRequest::new(scope.clone(), "remembered"))
            .await
            .unwrap();
        assert_ne!(id, MemoryId::default());

        let rows = runtime
            .search(MemorySearchRequest::new(scope.clone(), "search term", 10))
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].content, "search term");

        let recall = runtime
            .active_recall(ActiveRecallRequest {
                scope: scope.clone(),
                query: "active query".into(),
                budget: ActiveRecallBudget::default(),
            })
            .await
            .unwrap();
        assert_eq!(recall.selected.len(), 1);

        let compiled = runtime
            .compile_knowledge(KnowledgeCompileRequest {
                scope,
                candidates: Vec::new(),
                existing_claims: Vec::new(),
            })
            .await
            .unwrap();
        assert_eq!(compiled.claims.len(), 1);

        assert_eq!(facade.remembers.lock().unwrap().len(), 1);
        assert_eq!(facade.searches.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn runtime_status_reports_composed_capabilities() {
        let facade = Arc::new(FakeFacade::new(MemoryStatusReport::healthy(
            "fake-provider",
            MemoryCapabilitySet {
                store: true,
                search: true,
                prompt: false,
                lifecycle: false,
                flush: false,
                artifact: false,
                governance: false,
                knowledge: false,
            },
        )));
        let runtime = ComposedMemoryRuntime::new(
            facade,
            Arc::new(FakeActiveRecall),
            Arc::new(FakeKnowledgeCompiler),
        )
        .with_runtime_id("memory-runtime-test")
        .with_provider_profile(Some("profile-a".into()));

        let status = runtime.status().await;
        assert_eq!(status.runtime_id, "memory-runtime-test");
        assert_eq!(status.provider_profile.as_deref(), Some("profile-a"));
        assert!(status.store_available);
        assert!(status.search_available);
        assert!(status.active_recall_available);
        assert!(status.knowledge_available);
    }
}
