//! Contract tests for experience destination routing through Memory facades.

use std::sync::Arc;

use macaca_kernel::SystemService;
use macaca_memory::MemoryRuntimeFacade;
use macaca_proto::{ApplicationId, TraceContext};
use macaca_skill::{
    SkillEvolutionProposalAction, SkillExperienceCandidateDestination,
    SkillExperienceProposalCommand, SkillExperienceProposalResult, SkillServiceScope,
    SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
};

use crate::SkillSystemServiceProvider;

use super::fixtures::{reusable_experience_candidate, traced_command};
use super::test_doubles::RecordingMemoryRuntime;

#[tokio::test]
async fn skill_experience_memory_destination_routes_through_memory_facade() {
    let memory_runtime = Arc::new(RecordingMemoryRuntime::default());
    let provider = SkillSystemServiceProvider::new()
        .with_memory_runtime(Arc::clone(&memory_runtime) as Arc<dyn MemoryRuntimeFacade>);
    let trace = TraceContext::new("trace-skill-experience-memory-route");
    let application_id = ApplicationId::new();
    let mut candidate = reusable_experience_candidate(vec!["artifact-proof-memory".into()]);
    candidate.application_id = Some(application_id);
    candidate.destination = SkillExperienceCandidateDestination::MemoryFact;
    candidate.recommended_action = SkillEvolutionProposalAction::Discard;
    let command = SkillExperienceProposalCommand {
        trace: trace.clone(),
        scope: SkillServiceScope {
            application_id: Some(application_id),
            session_id: Some("session-memory-route".into()),
            tenant_id: Some("tenant-route".into()),
            agent_name: Some("agent".into()),
        },
        candidate,
    };

    let result = provider
        .call(traced_command(
            SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
            command,
            trace,
        ))
        .await
        .expect("memory destination should route through the injected memory facade");
    let proposal: SkillExperienceProposalResult =
        serde_json::from_value(result.output).expect("proposal result should decode");

    assert!(!proposal.mutated);
    assert_eq!(proposal.destination_route.status.as_str(), "routed");
    assert_eq!(
        proposal.destination_route.destination,
        SkillExperienceCandidateDestination::MemoryFact
    );
    assert!(proposal
        .destination_route
        .target_ref
        .as_deref()
        .is_some_and(|target| target.starts_with("memory://")));
    let writes = memory_runtime.writes.lock().await;
    assert_eq!(writes.len(), 1);
    assert_eq!(writes[0].scope.identity.application_id, application_id);
    assert_eq!(
        writes[0].scope.identity.session_id.as_deref(),
        Some("session-memory-route")
    );
    assert!(writes[0]
        .content
        .contains("verified task produced a reusable skill maintenance procedure"));
    assert!(!writes[0].content.contains("SKILL.md"));
}

#[tokio::test]
async fn skill_experience_knowledge_destination_routes_through_knowledge_facade() {
    let memory_runtime = Arc::new(RecordingMemoryRuntime::default());
    let provider = SkillSystemServiceProvider::new()
        .with_memory_runtime(Arc::clone(&memory_runtime) as Arc<dyn MemoryRuntimeFacade>);
    let trace = TraceContext::new("trace-skill-experience-knowledge-route");
    let application_id = ApplicationId::new();
    let mut candidate = reusable_experience_candidate(vec!["artifact-proof-knowledge".into()]);
    candidate.application_id = Some(application_id);
    candidate.destination = SkillExperienceCandidateDestination::KnowledgeDigest;
    candidate.recommended_action = SkillEvolutionProposalAction::Discard;
    let command = SkillExperienceProposalCommand {
        trace: trace.clone(),
        scope: SkillServiceScope {
            application_id: Some(application_id),
            session_id: Some("session-knowledge-route".into()),
            tenant_id: None,
            agent_name: Some("agent".into()),
        },
        candidate,
    };

    let result = provider
        .call(traced_command(
            SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
            command,
            trace,
        ))
        .await
        .expect("knowledge destination should route through the injected knowledge facade");
    let proposal: SkillExperienceProposalResult =
        serde_json::from_value(result.output).expect("proposal result should decode");

    assert_eq!(proposal.destination_route.status.as_str(), "routed");
    assert_eq!(
        proposal.destination_route.destination,
        SkillExperienceCandidateDestination::KnowledgeDigest
    );
    assert!(proposal
        .destination_route
        .target_ref
        .as_deref()
        .is_some_and(|target| target.starts_with("knowledge://")));
    let compiles = memory_runtime.knowledge_compiles.lock().await;
    assert_eq!(compiles.len(), 1);
    assert_eq!(compiles[0].scope.identity.application_id, application_id);
    assert_eq!(compiles[0].candidates.len(), 1);
    assert_eq!(
        compiles[0].candidates[0].source,
        macaca_memory::CandidateSource::AgentSummary
    );
    assert!(compiles[0].candidates[0]
        .content
        .contains("verified task produced a reusable skill maintenance procedure"));
}
