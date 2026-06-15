//! Destination routing for Skill Evolution experience candidates.
//!
//! The Skill service owns classification and proposal governance, but Memory
//! and Knowledge destinations belong to the Memory service family.  This module
//! keeps that cross-service routing in a focused Strategy/Facade adapter so the
//! main Skill provider remains a thin command adapter and does not grow into a
//! hidden memory or knowledge implementation.

use std::sync::Arc;

use macaca_memory::{
    CandidateSource, KnowledgeCompileRequest, MemoryRuntimeFacade, MemoryScope, MemoryWriteRequest,
};
use macaca_proto::{MemoryLayer, TraceContext};
use macaca_skill::{
    SkillExperienceCandidate, SkillExperienceCandidateDestination,
    SkillExperienceDestinationRouteResult, SkillServiceScope,
};

/// Optional routing strategy for non-skill experience destinations.
///
/// Absence is explicit: hosts that do not wire a Memory runtime receive
/// structured `Unavailable` route results instead of fake success.  This keeps
/// optional-module behavior aligned with Macaca's serviceization rules.
#[derive(Default)]
pub(crate) struct SkillExperienceDestinationRouter {
    memory_runtime: Option<Arc<dyn MemoryRuntimeFacade>>,
}

impl SkillExperienceDestinationRouter {
    pub(crate) fn with_memory_runtime(memory_runtime: Arc<dyn MemoryRuntimeFacade>) -> Self {
        Self {
            memory_runtime: Some(memory_runtime),
        }
    }

    /// Route the candidate's selected destination through its owning service.
    ///
    /// Skill destinations remain in the governance proposal workflow. Memory
    /// and Knowledge destinations call the Memory runtime facade with bounded
    /// summaries and identifier metadata only.
    pub(crate) async fn route(
        &self,
        scope: &SkillServiceScope,
        candidate: &SkillExperienceCandidate,
        trace: &TraceContext,
    ) -> SkillExperienceDestinationRouteResult {
        match candidate.destination {
            SkillExperienceCandidateDestination::MemoryFact => {
                self.route_memory_fact(scope, candidate, trace).await
            }
            SkillExperienceCandidateDestination::KnowledgeDigest => {
                self.route_knowledge_digest(scope, candidate, trace).await
            }
            SkillExperienceCandidateDestination::ExistingSkillPatchProposal
            | SkillExperienceCandidateDestination::NewSkillDraft
            | SkillExperienceCandidateDestination::SupportFileDraft
            | SkillExperienceCandidateDestination::NoOp => {
                SkillExperienceDestinationRouteResult::skipped(
                    candidate.destination.clone(),
                    "destination remains in skill governance proposal workflow",
                )
            }
        }
    }

    async fn route_memory_fact(
        &self,
        scope: &SkillServiceScope,
        candidate: &SkillExperienceCandidate,
        trace: &TraceContext,
    ) -> SkillExperienceDestinationRouteResult {
        let Some(memory_runtime) = self.memory_runtime.as_ref() else {
            return SkillExperienceDestinationRouteResult::unavailable(
                candidate.destination.clone(),
                "memory runtime facade is unavailable",
            );
        };
        let Some(memory_scope) = memory_scope_from_skill_scope(scope, candidate) else {
            return SkillExperienceDestinationRouteResult::unavailable(
                candidate.destination.clone(),
                "memory destination requires application scope",
            );
        };
        match memory_runtime
            .remember(
                MemoryWriteRequest::new(memory_scope, candidate.bounded_summary.clone())
                    .layer(MemoryLayer::Session)
                    .metadata(sanitized_destination_metadata(candidate, trace)),
            )
            .await
        {
            Ok(memory_id) => {
                tracing::info!(
                    trace_id = %trace.trace_id,
                    task_id = %candidate.task_id,
                    memory_id = %memory_id.0,
                    "skill experience memory destination routed through memory facade"
                );
                SkillExperienceDestinationRouteResult::routed(
                    candidate.destination.clone(),
                    format!("memory://{}", memory_id.0),
                )
            }
            Err(err) => SkillExperienceDestinationRouteResult::failed(
                candidate.destination.clone(),
                err.to_string(),
            ),
        }
    }

    async fn route_knowledge_digest(
        &self,
        scope: &SkillServiceScope,
        candidate: &SkillExperienceCandidate,
        trace: &TraceContext,
    ) -> SkillExperienceDestinationRouteResult {
        let Some(memory_runtime) = self.memory_runtime.as_ref() else {
            return SkillExperienceDestinationRouteResult::unavailable(
                candidate.destination.clone(),
                "knowledge compiler facade is unavailable",
            );
        };
        let Some(memory_scope) = memory_scope_from_skill_scope(scope, candidate) else {
            return SkillExperienceDestinationRouteResult::unavailable(
                candidate.destination.clone(),
                "knowledge destination requires application scope",
            );
        };
        let candidate_id = format!("skill-experience-{}", candidate.task_id.trim());
        let mut memory_candidate = macaca_memory::MemoryCandidate::new(
            candidate_id.clone(),
            memory_scope.clone(),
            CandidateSource::AgentSummary,
            candidate.bounded_summary.clone(),
        )
        .layer(MemoryLayer::Session)
        .confidence(0.8)
        .metadata(sanitized_destination_metadata(candidate, trace));
        memory_candidate.notes = Some("routed from skill experience destination".into());
        match memory_runtime
            .compile_knowledge(KnowledgeCompileRequest {
                scope: memory_scope,
                candidates: vec![memory_candidate],
                existing_claims: Vec::new(),
            })
            .await
        {
            Ok(result) => {
                tracing::info!(
                    trace_id = %trace.trace_id,
                    task_id = %candidate.task_id,
                    claims = result.claims.len(),
                    conflicts = result.conflicts.len(),
                    "skill experience knowledge destination routed through memory knowledge facade"
                );
                SkillExperienceDestinationRouteResult::routed(
                    candidate.destination.clone(),
                    format!("knowledge://candidate/{candidate_id}"),
                )
            }
            Err(err) => SkillExperienceDestinationRouteResult::failed(
                candidate.destination.clone(),
                err.to_string(),
            ),
        }
    }
}

fn memory_scope_from_skill_scope(
    scope: &SkillServiceScope,
    candidate: &SkillExperienceCandidate,
) -> Option<MemoryScope> {
    let application_id = scope.application_id.or(candidate.application_id)?;
    let mut memory_scope = if let Some(session_id) = scope
        .session_id
        .as_ref()
        .or(candidate.session_id.as_ref())
        .filter(|id| !id.trim().is_empty())
    {
        MemoryScope::session_shared(application_id, session_id.clone())
    } else if let Some(agent_name) = scope
        .agent_name
        .as_ref()
        .or(candidate.agent_name.as_ref())
        .filter(|name| !name.trim().is_empty())
    {
        MemoryScope::agent_private_named(application_id, agent_name.clone())
    } else {
        MemoryScope::new(
            application_id,
            macaca_memory::MemoryVisibility::ApplicationShared,
        )
    };
    if let Some(tenant_id) = scope.tenant_id.as_ref().filter(|id| !id.trim().is_empty()) {
        memory_scope = memory_scope.tenant_id(tenant_id.clone());
    }
    Some(memory_scope.namespace("skill-experience"))
}

fn sanitized_destination_metadata(
    candidate: &SkillExperienceCandidate,
    trace: &TraceContext,
) -> serde_json::Value {
    serde_json::json!({
        "source": "skill_experience",
        "trace_id": trace.trace_id,
        "task_id": candidate.task_id,
        "evidence_refs": candidate
            .evidence_ids
            .iter()
            .filter(|id| !id.trim().is_empty())
            .take(16)
            .cloned()
            .collect::<Vec<_>>(),
        "trace_digest_ref": candidate.trace_digest,
        "memory_digest_refs": candidate
            .memory_digest_refs
            .iter()
            .filter(|id| !id.trim().is_empty())
            .take(16)
            .cloned()
            .collect::<Vec<_>>(),
    })
}
