use macaca_proto::{AgentId, ApplicationId, MacacaResult, MemoryId};

use crate::core::MemoryScope;
use crate::vector_backend::{VectorBackendStatus, VectorMemoryBackend, VectorMemoryRecord};

/// Run reusable conformance checks against any vector memory backend.
pub async fn assert_vector_backend_conformance<B>(backend: &B) -> MacacaResult<()>
where
    B: VectorMemoryBackend + ?Sized,
{
    assert_private_collections_are_isolated(backend).await?;
    assert_shared_collection_is_explicit(backend).await?;
    assert_status_reports_topology(backend).await;
    Ok(())
}

pub async fn assert_private_collections_are_isolated<B>(backend: &B) -> MacacaResult<()>
where
    B: VectorMemoryBackend + ?Sized,
{
    let app_id = ApplicationId::new();
    let agent_a = MemoryScope::agent_private(app_id, AgentId::new());
    let agent_b = MemoryScope::agent_private(app_id, AgentId::new());
    let record_a = VectorMemoryRecord::new(
        MemoryId::new(),
        agent_a.clone(),
        "agent a private vector",
        vec![1.0, 0.0, 0.0],
    );
    let record_b = VectorMemoryRecord::new(
        MemoryId::new(),
        agent_b.clone(),
        "agent b private vector",
        vec![1.0, 0.0, 0.0],
    );

    backend.upsert_record(record_a.clone()).await?;
    backend.upsert_record(record_b.clone()).await?;

    let hits_a = backend.search(&agent_a, vec![1.0, 0.0, 0.0], 10).await?;
    let hits_b = backend.search(&agent_b, vec![1.0, 0.0, 0.0], 10).await?;

    assert_eq!(hits_a.len(), 1);
    assert_eq!(hits_a[0].memory_id, record_a.memory_id);
    assert_eq!(hits_b.len(), 1);
    assert_eq!(hits_b[0].memory_id, record_b.memory_id);
    Ok(())
}

pub async fn assert_shared_collection_is_explicit<B>(backend: &B) -> MacacaResult<()>
where
    B: VectorMemoryBackend + ?Sized,
{
    let app_id = ApplicationId::new();
    let private_scope = MemoryScope::agent_private(app_id, AgentId::new());
    let shared_scope = MemoryScope::session_shared(app_id, "session-1");

    backend
        .upsert_record(VectorMemoryRecord::new(
            MemoryId::new(),
            private_scope,
            "private only",
            vec![1.0, 0.0, 0.0],
        ))
        .await?;

    let shared_hits = backend
        .search(&shared_scope, vec![1.0, 0.0, 0.0], 10)
        .await?;
    assert!(shared_hits.is_empty());
    Ok(())
}

pub async fn assert_status_reports_topology<B>(backend: &B) -> VectorBackendStatus
where
    B: VectorMemoryBackend + ?Sized,
{
    let scope = MemoryScope::session_shared(ApplicationId::new(), "session-1");
    let status = backend.status(Some(&scope)).await;
    assert!(status.available);
    assert!(status.topology.is_some());
    status
}
